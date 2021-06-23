/*
 *   This file is part of NCC Group Scrying https://github.com/nccgroup/scrying
 *   Copyright 2020-2021 David Young <david(dot)young(at)nccgroup(dot)com>
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

use super::save;
use crate::{
    argparse::Opts, parsing::Target, reporting::ReportMessage, InputLists,
};
use gdk::prelude::WindowExtManual;
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
use std::{thread, time::Duration};
use webkit2gtk::{
    TLSErrorsPolicy, UserContentManager, WebContext, WebContextExt, WebView,
    WebViewExt, WebViewExtManual,
};

enum GuiMessage {
    Navigate(String),
    Exit,
    PageReady,
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
        window.set_default_size(opts.size.0 as i32, opts.size.1 as i32);
        window.set_position(WindowPosition::Center);
        window.set_title("Scrying WebCapture");
        //TODO work out how to make the window invisible
        // Maybe 'draw' can be used to draw it to some sort of headless
        // context:
        // https://docs.rs/gtk/0.9.2/gtk/trait.WidgetExt.html#tymethod.draw
        //window.set_visible(false); // this doesn't work for some reason
        //window.hide(); // this also has no effect
        // window.set_opacity(0.0); // this also has no effect

        // Create a webview
        let manager = UserContentManager::new();
        let context = WebContext::new();

        // Ignore certificate errors
        context.set_tls_errors_policy(TLSErrorsPolicy::Ignore);
        let webview = WebView::new_with_context_and_user_content_manager(
            &context, &manager,
        );

        // Make a channel for sending captured images back to the
        // supervisor thread
        let (img_tx, img_rx) = mpsc::channel::<Result<Vec<u8>, String>>();

        let targets_exhausted_clone = targets_exhausted_clone.clone();
        webview.connect_ready_to_show(move |_wv| {
            info!("Ready to show!");
        });

        // Create a communication channel
        let main_context = glib::MainContext::default();
        let (sender, receiver) =
            glib::MainContext::channel::<GuiMessage>(glib::Priority::default());

        let gui_sender = sender.clone();
        let (delayed_gui_sender, delayed_gui_receiver) =
            mpsc::channel::<GuiMessage>();

        // This is a horrendous bodge to make sure the webview has enough
        // time to render the page before we screenshot it. This is
        // because webkit2gtk only gives us a callback when the page has
        // *loaded*, not when it has *rendered*. Without this bodge in
        // place the captured images end up out of sync, because
        // callback (n+1) will fire after page (n+1) has been loaded but
        // while page (n) is still displayed. If page (n+1) fails to
        // render properly/in time then page (n) will get repeated. I
        // haven't been able to come up with a good workaround -
        // suggestions are welcome!
        //
        // Perhaps a reasonable idea could be to inject a Javascript
        // to run at body.onload time that calls back to a Rust function
        // (it's a webview, so we can cheat like that). Potential issues
        // here are that it might interfere with javascript on the page
        // and that someone could theoretically build a webpage that
        // somehow scans or watches for extra events firing and stops
        // them, which would break Scrying and I'd have to build in a
        // timeout watchdog.
        thread::spawn(move || {
            while let Ok(msg) = delayed_gui_receiver.recv() {
                thread::sleep(Duration::from_millis(2000));
                gui_sender.send(msg).unwrap();
            }
        });

        webview.connect_load_changed(move |wv, evt| {
            use webkit2gtk::LoadEvent::*;
            trace!(
                "Webview event: {} from `{:?}`",
                evt,
                wv.get_uri().map(|s| s.as_str().to_string())
            );
            if targets_exhausted_clone.load(Ordering::SeqCst) {
                // no targets left to capture, so ignore this event
                trace!("Targets exhausted, ignoring event");
                return;
            }
            if let Finished = evt {
                // grab screenshot
                delayed_gui_sender.send(GuiMessage::PageReady).unwrap();
            }
        });

        window.add(&webview);
        // Removing window.show_all successfully hides the window, but
        // then screen capturing fails as a result:
        // "[WARN] Capture failed: Unable to find window"
        window.show_all();

        // Dimensions need to be captured by the closure
        let width = opts.size.0 as i32;
        let height = opts.size.1 as i32;
        receiver.attach(Some(&main_context), move |msg| match msg {
            GuiMessage::Navigate(u) => {
                trace!("Navigating to target: {}", u);
                webview.load_uri(&u);
                glib::source::Continue(true)
            }
            GuiMessage::Exit => {
                info!("Exit signal received, closing window");
                window.close();
                glib::source::Continue(false)
            }
            GuiMessage::PageReady => {
                if let Some(win) = webview.get_window() {
                    match win.get_pixbuf(0, 0, width, height) {
                        Some(pix) => match pix.save_to_bufferv("png", &[]) {
                            Ok(buf) => {
                                trace!("Got pixbuf length {}", buf.len());
                                img_tx.send(Ok(buf)).unwrap();
                            }
                            Err(e) => {
                                img_tx
                                    .send(Err(format!(
                                        "Failed to process pixbuf: {}",
                                        e
                                    )))
                                    .unwrap();
                            }
                        },
                        None => {
                            img_tx
                                .send(Err(
                                    "Failed to retrieve pixbuf".to_string()
                                ))
                                .unwrap();
                        }
                    }
                } else {
                    img_tx
                        .send(Err("Unable to find window".to_string()))
                        .unwrap();
                }
                glib::source::Continue(true)
            }
        });

        let targets_clone = targets.clone();
        let report_tx_clone = report_tx.clone();
        let opts_clone = opts.clone();
        let targets_exhausted_clone = targets_exhausted.clone();
        let caught_ctrl_c_clone = caught_ctrl_c.clone();
        thread::spawn(move || {
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
                        trace!("Screen capture received! (len {})", img.len());
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
