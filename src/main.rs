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

use crate::argparse::{Opts, WebMode};
use crate::reporting::ReportMessage;
//#[allow(unused)]
//use log::{debug, error, info, trace, warn};
use color_eyre::Result;
use parsing::{generate_target_lists, InputLists};
use simplelog::{
    ColorChoice, CombinedLogger, Config, LevelFilter, SharedLogger, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::{create_dir_all, File};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use web::{chrome_worker, web_worker};

//#[macro_use]
mod log_macros;

mod argparse;
mod parsing;
mod rdp;
mod reporting;
mod util;
mod vnc;
mod web;

pub enum ThreadStatus {
    Complete,
}

#[tokio::main]
async fn main() -> Result<()> {
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
        ColorChoice::Auto,
    ));

    CombinedLogger::init(log_dests).unwrap();

    log::debug!("Got opts:\n{:?}", opts);

    // Load in the target lists, parsed from arguments, files, and nmap
    let targets = Arc::new(generate_target_lists(&opts));
    println!("{}", targets);

    if opts.test_import {
        log::info!("--test-import was supplied, exiting");
        return Ok(());
    }

    // Verify that targets have been processed
    if targets.rdp_targets.is_empty()
        && targets.web_targets.is_empty()
        && targets.vnc_targets.is_empty()
    {
        log::error!("No targets imported, exiting");
        return Ok(());
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
        if caught_ctrl_c_clone_for_handler.load(Ordering::SeqCst) {
            log::error!("Multiple ctrl+c caught, force-exiting...");
            std::process::exit(-1);
        }
        log::warn!("Caught interrupt signal, cleaning up...");
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
        log::debug!("Starting report thread");
        reporting::reporting_thread(report_rx, opts_clone, targets_clone)
    });

    // Spawn threads to iterate over the targets
    let rdp_handle = if !targets.rdp_targets.is_empty() {
        let targets_clone = targets.clone();
        let opts_clone = opts.clone();
        let report_tx_clone = report_tx.clone();
        let caught_ctrl_c_clone = caught_ctrl_c.clone();
        Some(thread::spawn(move || {
            log::debug!("Starting RDP worker threads");
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

    let vnc_handle = if !targets.vnc_targets.is_empty() {
        let targets_clone = targets.clone();
        let opts_clone = opts.clone();
        let report_tx_clone = report_tx.clone();
        let caught_ctrl_c_clone = caught_ctrl_c.clone();
        Some(thread::spawn(move || {
            log::debug!("Starting VNC worker threads");
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

    // If there are any web targets then start the web GUI process -
    // due to limitations in the general design of GUI frameworks the
    // GUI will either error or silently do nothing if not invoked from
    // the main thread.
    if !targets.web_targets.is_empty() {
        let opts_clone = opts.clone();
        let report_tx_clone = report_tx.clone();

        log::debug!("Starting Web worker");
        match opts.web_mode {
            WebMode::Chrome => chrome_worker(
                targets,
                opts_clone,
                report_tx_clone,
                caught_ctrl_c,
            )
            .await
            .unwrap(),
            WebMode::Native => {
                web_worker(targets, opts_clone, report_tx_clone, caught_ctrl_c)
                    .unwrap()
            }
        }
    }

    // wait for the workers to complete
    if let Some(h) = rdp_handle {
        h.join().unwrap().unwrap();
    }
    if let Some(h) = vnc_handle {
        h.join().unwrap();
    }
    report_tx.send(ReportMessage::GenerateReport).unwrap();
    reporting_handle.join().unwrap().unwrap();

    Ok(())
}

fn rdp_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<()> {
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
                debug!("RDP", "Thread complete, yay");
                num_workers -= 1;
            }
            Err(_) => {}
        }
        if num_workers < max_workers {
            if let Some(target) = targets_iter.next() {
                let target = target.clone();
                info!("RDP", "Adding worker for {:?}", target);
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
    debug!("RDP", "At the join part");
    for w in workers {
        debug!("RDP", "Joining {:?}", w);
        if w.join().is_err() {
            debug!("RDP", "Thread finished with errors");
        }
    }

    Ok(())
}

fn vnc_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<()> {
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
                info!("VNC", "Thread complete, yay");
                num_workers -= 1;
            }
            Err(_) => {}
        }
        if num_workers < max_workers {
            if let Some(target) = targets_iter.next() {
                let target = target.clone();
                info!("VNC", "Adding worker for {:?}", target);
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
    debug!("VNC", "At the join part");
    for w in workers {
        debug!("VNC", "Joining {:?}", w);
        w.join().unwrap();
    }

    Ok(())
}
