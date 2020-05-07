use std::fs::create_dir_all;
use std::path::Path;

use argparse::Mode;
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use parsing::{generate_target_lists, InputLists};
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

    // Create output directories if they do not exist
    let rdp_output_dir = Path::new("./output/rdp");
    let web_output_dir = Path::new("./output/web");
    if !rdp_output_dir.is_dir() {
        create_dir_all(rdp_output_dir).expect(&format!(
            "Error creating directory {}",
            rdp_output_dir.display()
        ));
    }
    if !web_output_dir.is_dir() {
        create_dir_all(web_output_dir).expect(&format!(
            "Error creating directory {}",
            web_output_dir.display()
        ));
    }

    match opts.mode {
        Mode::Rdp => rdp_worker(&targets, &rdp_output_dir).unwrap(),
        Mode::Web => web_worker(&targets, &web_output_dir).unwrap(),
        Mode::Auto => unimplemented!(),
    }
}

fn rdp_worker(targets: &InputLists, output_dir: &Path) -> Result<(), ()> {
    for target in &targets.rdp_targets {
        rdp::capture(&target, output_dir)?;
    }

    Ok(())
}

fn web_worker(_targets: &InputLists, _output_dir: &Path) -> Result<(), ()> {
    Ok(())
}
