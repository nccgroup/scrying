/*
 *   This file is part of NCC Group Scrying https://github.com/nccgroup/scrying
 *   Copyright 2020 David Young <david(dot)young(at)nccgroup(dot)com>
 *   Released as open source by NCC Group Plc - https://www.nccgroup.com
 *
 *   Scrying is free software: you can redistribute it and/or modify
 *   it under the terms of the GNU General Public License as published by
 *   the Free Software Foundation, either version 3 of the License, or
 *   (at your option) any later version.
 *
 *   Scrying is distributed in the hope that it will be useful,
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *   GNU General Public License for more details.
 *
 *   You should have received a copy of the GNU General Public License
 *   along with Scrying.  If not, see <https://www.gnu.org/licenses/>.
*/

use super::{save, HEIGHT, WIDTH};
use crate::{
    argparse::Opts, parsing::Target, reporting::ReportMessage, InputLists,
};
use gdk::prelude::{WindowExt, WindowExtManual};
use gio::prelude::*;
use gtk::{
    Application, ApplicationWindow, ContainerExt, GtkWindowExt, WidgetExt,
    WindowPosition,
};
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use webkit2gtk::{
    UserContentManager, WebContext, WebView, WebViewExt, WebViewExtManual,
};

enum GuiMessage {
    Navigate(String),
    Exit,
}

pub fn web_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a window
    let application = Application::new(
        Some("com.github.nccgroup.scrying"),
        Default::default(),
    )?;

    // "global" bool to turn off the LoadEvent::Finished handler when
    // the target list has been exhausted
    let targets_exhausted = Arc::new(AtomicBool::new(false));
    let targets_exhausted_clone = targets_exhausted.clone();
    application.connect_activate(move |app| {
        let window = ApplicationWindow::new(app);
        window.set_default_size(WIDTH, HEIGHT);
        window.set_position(WindowPosition::Center);
        window.set_title("Scrying WebCapture");
        //window.set_visible(false); // this doesn't work for some reason

        // Create a webview
        let manager = UserContentManager::new();
        let context = WebContext::new();
        let webview = WebView::new_with_context_and_user_content_manager(
            &context, &manager,
        );

        // Make a channel for sending captured images back to the
        // supervisor thread
        let (img_tx, img_rx) = mpsc::channel::<Result<Vec<u8>, String>>();

        let targets_exhausted_clone = targets_exhausted_clone.clone();
        webview.connect_load_changed(move |wv, evt| {
            use webkit2gtk::LoadEvent::*;
            trace!("Webview event: {}", evt);
            if targets_exhausted_clone.load(Ordering::SeqCst) {
                // no targets left to capture, so ignore this event
                trace!("Targets exhausted, ignoring event");
                return;
            }
            match evt {
                Finished => {
                    // grab screenshot
                    if let Some(win) = wv.get_window() {
                        match win.get_pixbuf(0, 0, WIDTH, HEIGHT) {
                            Some(pix) => {
                                match pix.save_to_bufferv("png", &[]) {
                                    Ok(buf) => {
                                        trace!(
                                            "Got pixbuf length {}",
                                            buf.len()
                                        );
                                        img_tx.send(Ok(buf)).unwrap();
                                    }
                                    Err(e) => {
                                        error!(
                                            "Failed to process pixbuf: {}",
                                            e
                                        );
                                    }
                                }
                            }
                            None => {
                                error!("Failed to retrieve pixbuf");
                            }
                        }
                    } else {
                        error!("Unable to find window");
                    }
                }
                _ => {}
            }
        });

        window.add(&webview);
        window.show_all();

        // Create a communication channel
        let main_context = glib::MainContext::default();
        let (sender, receiver) =
            glib::MainContext::channel::<GuiMessage>(glib::Priority::default());

        receiver.attach(Some(&main_context), move |msg| match msg {
            GuiMessage::Navigate(u) => {
                trace!("Navigating to target: {}", u);
                webview.load_uri(&u);
                glib::source::Continue(true)
            }
            GuiMessage::Exit => {
                info!("Exit signal received, closing window");
                //webview.stop_loading();
                //webview.get_window().unwrap().get_toplevel().destroy();
                window.close();
                glib::source::Continue(false)
            }
        });

        // let mut received_exit_signal = false;
        /*glib::source::idle_add(move || {
            // check ctrl+c?

            // Check end of target list
            match end_of_targets_rx.try_recv() {
                Err(TryRecvError::Empty) => {}
                e => {
                    // Empty does nothing, all other options (message or
                    // channel disconnected) result in closing the window
                    if !received_exit_signal {
                        info!("Received signal `{:?}`, closing webview", e);
                        received_exit_signal = true;
                    }
                    // +-- this but in a way that works across threads
                    // v   or can signal to the window somehow to close
                    //window.close();
                }
            }

            // check rendered?
            glib::source::Continue(true)
        });*/

        let targets_clone = targets.clone();
        let report_tx_clone = report_tx.clone();
        let opts_clone = opts.clone();
        let targets_exhausted_clone = targets_exhausted.clone();
        let caught_ctrl_c_clone = caught_ctrl_c.clone();
        std::thread::spawn(move || {
            for target in &targets_clone.web_targets {
                // If ctrl+c has been pressed then don't send any more targets
                if caught_ctrl_c_clone.load(Ordering::SeqCst) {
                    break;
                }

                if let Target::Url(u) = target {
                    sender
                        .send(GuiMessage::Navigate(u.as_str().to_string()))
                        .unwrap();
                } else {
                    warn!("Target `{}` is not a URL!", target);
                    continue;
                }

                // Wait for a response
                match img_rx.recv() {
                    Ok(Ok(img)) => {
                        trace!("Screen capture received!");
                        save(
                            &target,
                            &opts_clone.output_dir,
                            &img,
                            &report_tx_clone,
                        )
                        .unwrap();
                    }
                    Ok(Err(e)) => {
                        warn!("Capture failed: {}", e);
                    }
                    Err(e) => {
                        warn!("Channel disconnected: {}", e);
                        break;
                    }
                }
            }

            // Reached end of input list - close the window
            trace!("Reached end of input list, sending window close request");
            targets_exhausted_clone.store(true, Ordering::SeqCst);
            sender.send(GuiMessage::Exit).unwrap();
            //end_of_targets_tx.send(()).unwrap();
        });
    });

    application.connect_shutdown(|_app| {
        debug!("application reached SHUTDOWN");
    });

    trace!("application.run");
    application.run(Default::default());
    trace!("End of web_worker function");
    Ok(())
}
