use argparse::Mode;
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use parsing::generate_target_lists;
use simplelog::{
    CombinedLogger, Config, LevelFilter, SharedLogger, TermLogger,
    TerminalMode, WriteLogger,
};
use std::fs::File;

mod argparse;
mod parsing;
mod rdp;
mod util;
mod web;

fn main() {
    println!("Starting NCC Group AutoSnap...");
    let opts = argparse::parse();

    // Configure logging
    let mut log_dests: Vec<Box<dyn SharedLogger>> = Vec::new();

    if let Some(log_file) = &opts.log_file {
        // Enable logging to a file at INFO level
        log_dests.push(WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create(log_file).unwrap(),
        ));
    }

    let level_filter;
    if !opts.silent {
        level_filter = match opts.verbose {
            0 => LevelFilter::Info,
            1 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        };
    } else {
        level_filter = LevelFilter::Warn;
    }

    log_dests.push(
        TermLogger::new(level_filter, Config::default(), TerminalMode::Mixed)
            .unwrap(),
    );

    CombinedLogger::init(log_dests).unwrap();

    debug!("Got opts:\n{:?}", opts);

    // Load in the target lists, parsed from arguments, files, and nmap
    let targets = generate_target_lists(&opts);
    println!("target list: {:?}", targets);

    match opts.mode {
        Mode::Rdp => rdp::capture(&opts),
        Mode::Web => web::capture(&opts),
        Mode::Auto => unimplemented!(),
    }
}
