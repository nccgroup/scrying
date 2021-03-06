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

use crate::argparse::Opts;
use crate::reporting::ReportMessage;
use error::Error;
use headless_chrome::{Browser, LaunchOptionsBuilder};
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use parsing::{generate_target_lists, InputLists};
use simplelog::{
    CombinedLogger, Config, LevelFilter, SharedLogger, TermLogger,
    TerminalMode, WriteLogger,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::create_dir_all;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

mod argparse;
mod error;
mod parsing;
mod rdp;
mod reporting;
mod util;
mod vnc;
mod web;

pub enum ThreadStatus {
    Complete,
}

fn main() {
    println!("Starting NCC Group Scrying...");
    let opts = Arc::new(argparse::parse().unwrap());

    // Configure logging
    let mut log_dests: Vec<Box<dyn SharedLogger>> = Vec::new();

    if let Some(log_file) = &opts.log_file {
        // Enable logging to a file at INFO level by default
        // Increasing global log verbosity increases log file verbosity
        // accordingly. Combinations such as --silent -vv make sense
        // when using a log file as the file will get TRACE messages
        // while the terminal only gets WARN and higher.
        let level_filter = match opts.verbose {
            0 => LevelFilter::Info,
            1 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        };
        log_dests.push(WriteLogger::new(
            level_filter,
            Config::default(),
            File::create(log_file).unwrap(),
        ));
    }

    let level_filter = if !opts.silent {
        match opts.verbose {
            0 => LevelFilter::Info,
            1 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        }
    } else {
        LevelFilter::Warn
    };

    log_dests.push(TermLogger::new(
        level_filter,
        Config::default(),
        TerminalMode::Mixed,
    ));

    CombinedLogger::init(log_dests).unwrap();

    debug!("Got opts:\n{:?}", opts);

    // Load in the target lists, parsed from arguments, files, and nmap
    let targets = Arc::new(generate_target_lists(&opts));
    println!("{}", targets);

    if opts.test_import {
        info!("--test-import was supplied, exiting");
        return;
    }

    // Verify that targets have been processed
    if targets.rdp_targets.is_empty()
        && targets.web_targets.is_empty()
        && targets.vnc_targets.is_empty()
    {
        error!("No targets imported, exiting");
        return;
    }

    // Create output directories if they do not exist
    let output_base = Path::new(&opts.output_dir);
    let rdp_output_dir = output_base.join("rdp");
    if !targets.rdp_targets.is_empty() && !rdp_output_dir.is_dir() {
        create_dir_all(&rdp_output_dir).unwrap_or_else(|_| {
            panic!("Error creating directory {}", rdp_output_dir.display())
        });
    }
    let web_output_dir = output_base.join("web");
    if !targets.web_targets.is_empty() && !web_output_dir.is_dir() {
        create_dir_all(&web_output_dir).unwrap_or_else(|_| {
            panic!("Error creating directory {}", web_output_dir.display())
        });
    }
    let vnc_output_dir = output_base.join("vnc");
    if !targets.vnc_targets.is_empty() && !vnc_output_dir.is_dir() {
        create_dir_all(&vnc_output_dir).unwrap_or_else(|_| {
            panic!("Error creating directory {}", vnc_output_dir.display())
        });
    }

    // Attach interrupt handler to catch ctrl-c
    let caught_ctrl_c = Arc::new(AtomicBool::new(false));
    let caught_ctrl_c_clone_for_handler = caught_ctrl_c.clone();
    ctrlc::set_handler(move || {
        warn!("Caught interrupt signal, cleaning up...");
        caught_ctrl_c_clone_for_handler.store(true, Ordering::SeqCst);
    })
    .expect("Unable to attach interrupt signal handler");

    // Start report collating thread
    let (report_tx, report_rx): (
        mpsc::Sender<ReportMessage>,
        mpsc::Receiver<_>,
    ) = mpsc::channel();
    let opts_clone = opts.clone();
    let targets_clone = targets.clone();
    let reporting_handle = thread::spawn(move || {
        debug!("Starting report thread");
        reporting::reporting_thread(report_rx, opts_clone, targets_clone)
    });

    // Spawn threads to iterate over the targets
    let rdp_handle = if !targets.rdp_targets.is_empty() {
        let targets_clone = targets.clone();
        let opts_clone = opts.clone();
        let report_tx_clone = report_tx.clone();
        let caught_ctrl_c_clone = caught_ctrl_c.clone();
        Some(thread::spawn(move || {
            debug!("Starting RDP worker threads");
            rdp_worker(
                targets_clone,
                opts_clone,
                report_tx_clone,
                caught_ctrl_c_clone,
            )
        }))
    } else {
        None
    };

    let web_handle = if !targets.web_targets.is_empty() {
        let targets_clone = targets.clone();
        let opts_clone = opts.clone();
        let report_tx_clone = report_tx.clone();
        let caught_ctrl_c_clone = caught_ctrl_c.clone();
        Some(thread::spawn(move || {
            debug!("Starting Web worker threads");
            web_worker(
                targets_clone,
                opts_clone,
                report_tx_clone,
                caught_ctrl_c_clone,
            )
            .unwrap()
        }))
    } else {
        None
    };

    let vnc_handle = if !targets.vnc_targets.is_empty() {
        // clone here will be more useful when there are more target types
        let targets_clone = targets; //.clone();
        let opts_clone = opts; //.clone();
        let report_tx_clone = report_tx.clone();
        let caught_ctrl_c_clone = caught_ctrl_c; //.clone();
        Some(thread::spawn(move || {
            debug!("Starting VNC worker threads");
            vnc_worker(
                targets_clone,
                opts_clone,
                report_tx_clone,
                caught_ctrl_c_clone,
            )
            .unwrap()
        }))
    } else {
        None
    };

    // wait for the workers to complete
    if let Some(h) = rdp_handle {
        h.join().unwrap().unwrap();
    }
    if let Some(h) = web_handle {
        h.join().unwrap();
    }
    if let Some(h) = vnc_handle {
        h.join().unwrap();
    }
    report_tx.send(ReportMessage::GenerateReport).unwrap();
    reporting_handle.join().unwrap().unwrap();
}

fn rdp_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<(), ()> {
    use mpsc::{Receiver, Sender};
    let max_workers = opts.threads;
    let mut num_workers: usize = 0;
    let mut targets_iter = targets.rdp_targets.iter();
    let mut workers: Vec<_> = Vec::new();
    let (thread_status_tx, thread_status_rx): (
        Sender<ThreadStatus>,
        Receiver<ThreadStatus>,
    ) = mpsc::channel();
    while !caught_ctrl_c.load(Ordering::SeqCst) {
        // check for status messages
        // Turn off clippy's single_match warning here because match
        // matches the intuition for how try_recv is processed better
        // than an if let.
        #[allow(clippy::single_match)]
        match thread_status_rx.try_recv() {
            Ok(ThreadStatus::Complete) => {
                debug!("Thread complete, yay");
                num_workers -= 1;
            }
            Err(_) => {}
        }
        if num_workers < max_workers {
            if let Some(target) = targets_iter.next() {
                let target = target.clone();
                info!("Adding worker for {:?}", target);
                let opts_clone = opts.clone();
                let tx = thread_status_tx.clone();
                let report_tx_clone = report_tx.clone();
                let handle = thread::spawn(move || {
                    rdp::capture(&target, &opts_clone, tx, &report_tx_clone)
                });

                workers.push(handle);
                num_workers += 1;
            } else {
                break;
            }
        }
    }
    debug!("At the join part");
    for w in workers {
        debug!("Joining {:?}", w);
        if w.join().is_err() {
            debug!("Thread finished with errors");
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn web_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::parsing::Target;
    use crossbeam_channel::unbounded;
    use native_windows_gui::{self as nwg, Window};
    use once_cell::sync::OnceCell;
    use std::sync::RwLock;
    use webview2::{Controller, Stream, WebErrorStatus};
    use winapi::um::winuser::*;

    type CaptureResult = Result<Vec<u8>, Option<WebErrorStatus>>;

    let (target_sender, target_receiver) = unbounded::<Target>();
    let (result_sender, result_receiver) = mpsc::channel::<CaptureResult>();

    nwg::init().unwrap();

    let mut window = Window::default();

    Window::builder()
        .title("WebView2 - NWG")
        // CW_USEDEFAULT incidentally works, because it's actually i32::MIN, and
        // after saturating mul_div, it's still i32::MIN.
        .position((CW_USEDEFAULT, CW_USEDEFAULT))
        .size((1600, 900))
        .build(&mut window)
        .unwrap();

    let window_handle = window.handle;
    let hwnd = window_handle.hwnd().unwrap();

    let controller: Arc<OnceCell<Controller>> = Arc::new(OnceCell::new());
    let controller_clone = controller.clone();

    trace!("Building webview");
    let _res = webview2::EnvironmentBuilder::new()
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
                let target_receiver_clone = target_receiver.clone();
                webview
                    .add_navigation_completed(move |wv, args| {
                        trace!("Navigation completed handler");
                        let mut stream = Stream::from_bytes(&[]);
                        let target_receiver_clone = target_receiver.clone();
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
                                    println!("image: {:?}", stream);

                                    //TODO work out how to save the image
                                    //
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
                                        .send(Ok(Vec::new()))
                                        .unwrap();

                                    Ok(())
                                },
                            )
                            .unwrap();
                        } else {
                            let status = args
                                .get_web_error_status()
                                .map(|s| Some(s))
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
        })
        .unwrap();

    let window_handle = window.handle;
    let controller_clone = controller.clone();
    nwg::bind_raw_event_handler(
        &window_handle,
        0xffff + 1,
        move |_, msg, _, _| {
            match msg {
                WM_CLOSE => {
                    println!("close window");

                    nwg::stop_thread_dispatch();
                }
                _ => {}
            }
            None
        },
    )
    .unwrap();

    // Launch the gui threads in a nonblocking way
    trace!("Dispatch thread events");
    let mut first_run = true;
    let mut exit_signal_sent = false;
    let mut idx = 0;
    nwg::dispatch_thread_events_with_callback(move || {
        use mpsc::TryRecvError;

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
                        info!("Received result: {:?}", msg);

                        // Load in the next target
                        if idx < targets.web_targets.len() {
                            if let Target::Url(u) = &targets.web_targets[idx] {
                                wv.navigate(u.as_str()).unwrap();
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
                    Ok(Err(Some(e))) => {
                        warn!("Capture error: {:?}", e);
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
                    if idx < targets.web_targets.len() {
                        if let Target::Url(u) = &targets.web_targets[idx] {
                            wv.navigate(u.as_str()).unwrap();
                            idx += 1;
                        } else {
                            error!("Target is not a URL");
                            c.close().unwrap();
                        }
                    } else {
                        debug!("Reached end of target list");
                        c.close().unwrap();
                    }

                    first_run = false;
                }
            }
        }
    });

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn web_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut chrome_env = HashMap::new();
    if let Some(p) = &opts.web_proxy {
        chrome_env.insert("http_proxy".to_string(), p.clone());
        chrome_env.insert("https_proxy".to_string(), p.clone());
    }
    let launch_options = LaunchOptionsBuilder::default()
        .headless(true)
        .window_size(Some((1280, 720)))
        .process_envs(Some(chrome_env))
        .args(vec![OsStr::new("--ignore-certificate-errors")])
        .build()?;
    let browser = Browser::new(launch_options).expect("failed to init chrome");
    let tab = browser.wait_for_initial_tab().expect("Failed to init tab");

    for target in &targets.web_targets {
        if caught_ctrl_c.load(Ordering::SeqCst) {
            break;
        }
        if let Err(e) = web::capture(target, &opts.output_dir, &tab, &report_tx)
        {
            match e {
                Error::IoError(e) => {
                    // Should probably abort on an IO error
                    error!("IO error: {}", e);
                    break;
                }
                Error::ChromeError(e) => {
                    warn!("Failed to capture image: {}", e);
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}

fn vnc_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    use mpsc::{Receiver, Sender};
    let max_workers = opts.threads;
    let mut num_workers: usize = 0;
    let mut targets_iter = targets.vnc_targets.iter();
    let mut workers: Vec<_> = Vec::new();
    let (thread_status_tx, thread_status_rx): (
        Sender<ThreadStatus>,
        Receiver<ThreadStatus>,
    ) = mpsc::channel();
    while !caught_ctrl_c.load(Ordering::SeqCst) {
        // check for status messages
        // Turn off clippy's single_match warning here because match
        // matches the intuition for how try_recv is processed better
        // than an if let.
        #[allow(clippy::single_match)]
        match thread_status_rx.try_recv() {
            Ok(ThreadStatus::Complete) => {
                info!("Thread complete, yay");
                num_workers -= 1;
            }
            Err(_) => {}
        }
        if num_workers < max_workers {
            if let Some(target) = targets_iter.next() {
                let target = target.clone();
                info!("Adding VNC worker for {:?}", target);
                let opts_clone = opts.clone();
                let tx = thread_status_tx.clone();
                let report_tx_clone = report_tx.clone();
                let handle = thread::spawn(move || {
                    vnc::capture(&target, &opts_clone, tx, &report_tx_clone)
                });

                workers.push(handle);
                num_workers += 1;
            } else {
                break;
            }
        }
    }
    debug!("At the join part");
    for w in workers {
        debug!("Joining {:?}", w);
        w.join().unwrap();
    }

    Ok(())
}
