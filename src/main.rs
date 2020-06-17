/*
 *   This file is part of NCC Group Scamper https://github.com/nccgroup/scamper
 *   Copyright 2020 David Young <david(dot)young(at)nccgroup(dot)com>
 *   Released as open source by NCC Group Plc - https://www.nccgroup.com
 *
 *   Scamper is free software: you can redistribute it and/or modify
 *   it under the terms of the GNU General Public License as published by
 *   the Free Software Foundation, either version 3 of the License, or
 *   (at your option) any later version.
 *
 *   Scamper is distributed in the hope that it will be useful,
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *   GNU General Public License for more details.
 *
 *   You should have received a copy of the GNU General Public License
 *   along with Scamper.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::argparse::Opts;
use error::Error;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::create_dir_all;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
//use argparse::Mode;
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use parsing::{generate_target_lists, InputLists};
use simplelog::{
    CombinedLogger, Config, LevelFilter, SharedLogger, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;
use std::sync::Arc;

use headless_chrome::{Browser, LaunchOptionsBuilder};

mod argparse;
mod error;
mod parsing;
mod rdp;
mod util;
mod web;

pub enum ThreadStatus {
    Complete,
}

fn main() {
    println!("Starting NCC Group Scamper...");
    let opts = Arc::new(argparse::parse());

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

    // Create output directories if they do not exist

    let rdp_output_dir = Path::new("./output/rdp");
    if !targets.rdp_targets.is_empty() && !rdp_output_dir.is_dir() {
        create_dir_all(rdp_output_dir).unwrap_or_else(|_| {
            panic!("Error creating directory {}", rdp_output_dir.display())
        });
    }
    let web_output_dir = Path::new("./output/web");
    if !targets.rdp_targets.is_empty() && !web_output_dir.is_dir() {
        create_dir_all(web_output_dir).unwrap_or_else(|_| {
            panic!("Error creating directory {}", web_output_dir.display())
        });
    }

    // Spawn threads to iterate over the targets
    let rdp_handle = if !targets.rdp_targets.is_empty() {
        let targets_clone = targets.clone();
        let opts_clone = opts.clone();
        Some(thread::spawn(move || {
            debug!("Starting RDP worker threads");
            rdp_worker(targets_clone, rdp_output_dir, opts_clone)
        }))
    } else {
        None
    };

    let web_handle = if !targets.web_targets.is_empty() {
        // clone here will be more useful when there are more target types
        let targets_clone = targets; //.clone();
        let opts_clone = opts; //.clone();
        Some(thread::spawn(move || {
            debug!("Starting Web worker threads");
            web_worker(targets_clone, &web_output_dir, opts_clone).unwrap()
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
}

fn rdp_worker(
    targets: Arc<InputLists>,
    output_dir: &'static Path,
    opts: Arc<Opts>,
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
    loop {
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
                println!("Adding worker for {:?}", target);
                let tx = thread_status_tx.clone();
                let handle = thread::spawn(move || {
                    rdp::capture(&target, &output_dir, tx)
                });

                workers.push(handle);
                num_workers += 1;
            } else {
                break;
            }
        }
    }
    println!("At the join part");
    for w in workers {
        print!("Joining {:?}", w);
        w.join().unwrap();
    }

    Ok(())
}

fn web_worker(
    targets: Arc<InputLists>,
    output_dir: &Path,
    opts: Arc<Opts>,
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
        if let Err(e) = web::capture(target, output_dir, &tab) {
            match e {
                Error::IoError(e) => {
                    // Should probably abort on an IO error
                    error!("IO error: {}", e);
                    break;
                }
                Error::ChromeError(e) => {
                    warn!("Failed to capture image: {}", e);
                }
                Error::RdpError(_) => unreachable!(),
            }
        }
    }
    Ok(())
}
