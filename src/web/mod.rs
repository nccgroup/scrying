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

use crate::argparse::Mode::Web;
use crate::error::Error;
use crate::parsing::Target;
use crate::reporting::ReportMessageContent;
use crate::reporting::{FileError, ReportMessage};
use crate::util::target_to_filename;
use headless_chrome::{protocol::page::ScreenshotFormat, Tab};
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::path::Path;
use std::sync::mpsc;
use std::{fs::File, io::Write};

pub fn capture(
    target: &Target,
    output_dir: &str,
    tab: &Tab,
    report_tx: &mpsc::Sender<ReportMessage>,
) -> Result<(), Error> {
    info!("Processing {}", target);

    let filename = format!("{}.png", target_to_filename(&target));

    let relative_filepath = Path::new("web").join(&filename);
    let output_file = Path::new(output_dir).join(&relative_filepath);
    info!("Saving image as {}", output_file.display());
    if let Target::Url(target) = target {
        tab.navigate_to(target.as_str())?;
        tab.wait_until_navigated()?;
        let png_data = tab
            .capture_screenshot(ScreenshotFormat::PNG, None, true)
            .expect("error making screenshot");
        let mut file = File::create(&output_file)?;
        file.write_all(&png_data)?;

        let report_message = ReportMessage::Output(ReportMessageContent {
            mode: Web,
            target: target.to_string(),
            output: FileError::File(relative_filepath.display().to_string()),
        });
        report_tx.send(report_message)?;
    }
    Ok(())
}

pub fn save(
    target: &Target,
    output_dir: &str,
    png_data: &[u8],
    report_tx: &mpsc::Sender<ReportMessage>,
) -> Result<(), Error> {
    debug!("Saving image for {}", target);

    let filename = format!("{}.png", target_to_filename(&target));

    let relative_filepath = Path::new("web").join(&filename);
    let output_file = Path::new(output_dir).join(&relative_filepath);
    info!("Saving image as {}", output_file.display());

    let mut file = File::create(&output_file)?;
    file.write_all(&png_data)?;

    let report_message = ReportMessage::Output(ReportMessageContent {
        mode: Web,
        target: target.to_string(),
        output: FileError::File(relative_filepath.display().to_string()),
    });
    report_tx.send(report_message)?;

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn web_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::parsing::Target;
    use native_windows_gui::{self as nwg, Window, WindowFlags};
    use once_cell::sync::OnceCell;
    use std::io::Read;
    use webview2::{Controller, Stream, WebErrorStatus};
    use winapi::um::winuser::*;

    type CaptureResult = Result<Vec<u8>, Option<WebErrorStatus>>;

    let (result_sender, result_receiver) = mpsc::channel::<CaptureResult>();

    nwg::init()?;

    let mut window = Window::default();

    Window::builder()
        .title("WebView2 - NWG")
        // TODO work out how to make a proper invisible window
        .position((65536, 65536))
        .size((1600, 900))
        .flags(WindowFlags::MAIN_WINDOW | WindowFlags::VISIBLE)
        .build(&mut window)?;

    let window_handle = window.handle;
    let hwnd = window_handle.hwnd().unwrap();

    let controller: Arc<OnceCell<Controller>> = Arc::new(OnceCell::new());
    let controller_clone = controller.clone();

    trace!("Building webview");
    let _res = webview2::EnvironmentBuilder::new()
        .with_additional_browser_arguments("--ignore-certificate-errors")
        .build(move |env| {
            trace!("Built webview");
            env.unwrap().create_controller(hwnd, move |c| {
                let c = c.unwrap();
                trace!("get controller");
                unsafe {
                    let mut rect = std::mem::zeroed();
                    GetClientRect(hwnd, &mut rect);
                    c.put_bounds(rect).unwrap();
                }

                let webview = c.get_webview().unwrap();
                c.move_focus(webview2::MoveFocusReason::Programmatic)
                    .unwrap();
                trace!("Add event handler");
                webview
                    .add_navigation_completed(move |wv, args| {
                        trace!("Navigation completed handler");
                        let mut stream = Stream::from_bytes(&[]);
                        let result_sender_clone = result_sender.clone();
                        if Ok(true) == args.get_is_success() {
                            trace!("Navigation successful, start capture");
                            wv.capture_preview(
                                webview2::CapturePreviewImageFormat::PNG,
                                stream.clone(),
                                move |r| {
                                    trace!(
                                        "Capture successful, sending result"
                                    );
                                    use std::io::{Seek, SeekFrom};
                                    r?;
                                    stream.seek(SeekFrom::Start(0)).unwrap();

                                    let mut captured_image: Vec<u8> =
                                        Vec::new();
                                    stream
                                        .read_to_end(&mut captured_image)
                                        .unwrap();

                                    // The whole callback-centred architecture is
                                    // really difficult to work with. Saving the
                                    // image involves writing the data to a file
                                    // with a target-specific name and sending a
                                    // message down the report_tx channel with
                                    // target-specific data, but the image data
                                    // only exists within this callback where
                                    // we don't have access to that additional
                                    // information :(
                                    //
                                    // I also don't think that the controller is
                                    // Send, so it might be difficult to have a
                                    // supervisor thread controlling the
                                    // navigation (and taking "callbacks" via
                                    // mpsc)?

                                    result_sender_clone
                                        .send(Ok(captured_image))
                                        .unwrap();

                                    Ok(())
                                },
                            )
                            .unwrap();
                        } else {
                            let status = args
                                .get_web_error_status()
                                .map(Some)
                                .unwrap_or_default();
                            warn!("Capture failed with error: {:?}", status);
                            result_sender_clone.send(Err(status)).unwrap();
                        }

                        Ok(())
                    })
                    .unwrap();

                controller_clone.set(c).unwrap();

                Ok(())
            })
        })?;

    let window_handle = window.handle;

    nwg::bind_raw_event_handler(
        &window_handle,
        0xffff + 1,
        move |_, msg, _, _| {
            if msg == WM_CLOSE {
                info!("close window");

                nwg::stop_thread_dispatch();
            }

            None
        },
    )?;

    // Launch the gui threads in a nonblocking way
    trace!("Dispatch thread events");
    let mut first_run = true;
    let mut exit_signal_sent = false;
    let mut idx = 0;
    let mut current_target: Option<Target> = None;
    nwg::dispatch_thread_events_with_callback(move || {
        use mpsc::TryRecvError;

        let mut load_next_target = false;

        if let Some(c) = controller.get() {
            // handle ctrl+c
            if !exit_signal_sent && caught_ctrl_c.load(Ordering::SeqCst) {
                c.close().unwrap();
                exit_signal_sent = true;
            }

            if let Ok(wv) = c.get_webview() {
                // Handle any completed captures
                match result_receiver.try_recv() {
                    Ok(Ok(msg)) => {
                        info!(
                            "Received {} bytes from screen capture",
                            msg.len()
                        );
                        if let Some(t) = &current_target {
                            web::save(&t, &opts.output_dir, &msg, &report_tx)
                                .unwrap();
                        }
                        load_next_target = true;
                    }
                    Ok(Err(Some(e))) => {
                        warn!("Capture error: {:?}", e);
                        load_next_target = true;
                    }
                    Ok(Err(None)) => {
                        warn!("Unknown error");
                    }
                    Err(TryRecvError::Disconnected) => {
                        error!("Channel disconnected");
                        c.close().unwrap();
                    }
                    Err(TryRecvError::Empty) => {
                        // No messages to process: do nothing
                    }
                }

                // Handle timeout

                // Handle first run
                if first_run {
                    load_next_target = true;

                    first_run = false;
                }

                if load_next_target {
                    // Load in the next target
                    if idx < targets.web_targets.len() {
                        if let Target::Url(u) = &targets.web_targets[idx] {
                            wv.navigate(u.as_str()).unwrap();
                            current_target =
                                Some(targets.web_targets[idx].clone());
                            idx += 1;
                        } else {
                            error!("Target is not a URL");
                            c.close().unwrap();
                        }
                    } else {
                        debug!("Reached end of target list");
                        c.close().unwrap();
                        nwg::stop_thread_dispatch();
                    }
                }
            }
        }
    });

    Ok(())
}
