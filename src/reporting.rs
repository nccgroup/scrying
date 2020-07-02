use crate::argparse::Mode;
use crate::argparse::Opts;
use crate::error::Error;
use crate::parsing::InputLists;

use askama::Template;
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;

#[allow(unused)]
use log::{debug, error, info, trace, warn};

#[derive(Template)]
#[template(path = "report.html")]
struct ReportTemplate {
    targets: Arc<InputLists>,
    rdp_outputs: Vec<ReportItem>,
    rdp_errors: Vec<ReportError>,
    web_outputs: Vec<ReportItem>,
    web_errors: Vec<ReportError>,
    vnc_outputs: Vec<ReportItem>,
    vnc_errors: Vec<ReportError>,
}

#[derive(Debug)]
struct ReportItem {
    pub target: String,
    pub file: String,
}

#[derive(Debug)]
struct ReportError {
    pub target: String,
    pub error: String,
}

#[derive(Debug)]
pub enum ReportMessage {
    Output(ReportMessageContent),
    GenerateReport,
}

#[derive(Debug)]
pub struct ReportMessageContent {
    pub mode: Mode,
    pub target: String,
    pub output: FileError,
}

/// Capture the output status as either a file or an error
#[derive(Debug)]
pub enum FileError {
    File(String),
    Error(String),
}

pub fn reporting_thread(
    rx: mpsc::Receiver<ReportMessage>,
    opts: Arc<Opts>,
    targets: Arc<InputLists>,
) -> Result<(), Error> {
    use Mode::*;
    // Vecs to collect the output messages in
    let mut rdp_outputs: Vec<ReportItem> = Vec::new();
    let mut web_outputs: Vec<ReportItem> = Vec::new();
    let mut vnc_outputs: Vec<ReportItem> = Vec::new();

    let mut rdp_errors: Vec<ReportError> = Vec::new();
    let mut web_errors: Vec<ReportError> = Vec::new();
    let mut vnc_errors: Vec<ReportError> = Vec::new();

    // Main loop listening on the channel
    while let Ok(msg) = rx.recv() {
        use ReportMessage::*;
        debug!("Received message: {:?}", msg);
        match msg {
            GenerateReport => break,

            Output(content) => {
                match (content.output, content.mode) {
                    (FileError::File(file), Rdp) => {
                        rdp_outputs.push(ReportItem {
                            target: content.target,
                            file,
                        });
                    }
                    (FileError::File(file), Web) => {
                        web_outputs.push(ReportItem {
                            target: content.target,
                            file,
                        });
                    }
                    (FileError::File(file), Vnc) => {
                        vnc_outputs.push(ReportItem {
                            target: content.target,
                            file,
                        });
                    }
                    (FileError::Error(error), Rdp) => {
                        rdp_errors.push(ReportError {
                            target: content.target,
                            error,
                        });
                    }
                    (FileError::Error(error), Web) => {
                        web_errors.push(ReportError {
                            target: content.target,
                            error,
                        });
                    }
                    (FileError::Error(error), Vnc) => {
                        vnc_errors.push(ReportError {
                            target: content.target,
                            error,
                        });
                    }
                    (_, Auto) => {
                        // In theory there should never be an Auto making
                        // it to this stage
                        unreachable!()
                    }
                }
            }
        }
    }

    info!("Generating report");

    println!("RDP outputs: {:?}", rdp_outputs);
    println!("Web outputs: {:?}", web_outputs);

    let report_file = Path::new(&opts.output_dir).join("report.html");

    let report_template = ReportTemplate {
        targets,
        rdp_outputs,
        rdp_errors,
        web_outputs,
        web_errors,
        vnc_outputs,
        vnc_errors,
    };
    let report = report_template.render()?;
    debug!("Report: {:?}", report);
    fs::write(&report_file, report)?;
    info!("Report saved to {:?}", report_file);
    Ok(())
}
