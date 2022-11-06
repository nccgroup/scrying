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

use crate::argparse::Mode;
use crate::argparse::Opts;
use crate::parsing::InputLists;

use askama::Template;
use color_eyre::Result;
use std::fs;
use std::path::Path;
use std::sync::{mpsc, Arc};

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
) -> Result<()> {
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

    if !opts.disable_report {
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
    }
    Ok(())
}
