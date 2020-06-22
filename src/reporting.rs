use crate::argparse::Opts;
use crate::error::Error;
use crate::parsing::InputLists;
use crate::rdp::RdpOutput;
use crate::vnc::VncOutput;
use crate::web::WebOutput;
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
    rdp_outputs: Vec<RdpOutput>,
    web_outputs: Vec<WebOutput>,
    vnc_outputs: Vec<VncOutput>,
}

#[derive(Debug)]
pub enum ReportMessage {
    RdpOutput(RdpOutput),
    WebOutput(WebOutput),
    VncOutput(VncOutput),
    GenerateReport,
}

pub trait AsReportMessage {
    /// Convert the object into an instance of the ReportMessage enum
    fn as_report_message(self) -> ReportMessage;

    /// Return the target, e.g. http://[2001:db8::2]:8080
    fn target(&self) -> &str;

    /// Return the filename relative to the "output" directory
    fn file(&self) -> &str;
}

pub fn reporting_thread(
    rx: mpsc::Receiver<ReportMessage>,
    opts: Arc<Opts>,
    targets: Arc<InputLists>,
) -> Result<(), Error> {
    // Vecs to collect the output messages in
    let mut rdp_outputs: Vec<RdpOutput> = Vec::new();
    let mut web_outputs: Vec<WebOutput> = Vec::new();
    let mut vnc_outputs: Vec<VncOutput> = Vec::new();

    // Main loop listening on the channel
    while let Ok(msg) = rx.recv() {
        use ReportMessage::*;
        debug!("Received message: {:?}", msg);
        match msg {
            GenerateReport => break,
            RdpOutput(out) => rdp_outputs.push(out),
            WebOutput(out) => web_outputs.push(out),
            VncOutput(out) => vnc_outputs.push(out),
        }
    }

    info!("Generating report");

    println!("RDP outputs: {:?}", rdp_outputs);
    println!("Web outputs: {:?}", web_outputs);

    let report_file = Path::new(&opts.output_dir).join("report.html");

    let report_template = ReportTemplate {
        targets,
        rdp_outputs,
        web_outputs,
        vnc_outputs,
    };
    let report = report_template.render()?;
    debug!("Report: {:?}", report);
    fs::write(&report_file, report)?;
    info!("Report saved to {:?}", report_file);
    Ok(())
}
