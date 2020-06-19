use crate::argparse::Opts;
use crate::rdp::RdpOutput;
use crate::web::WebOutput;
use std::path::Path;
use std::sync::mpsc;
use std::sync::Arc;

#[allow(unused)]
use log::{debug, error, info, trace, warn};

#[derive(Debug)]
pub enum ReportMessage {
    RdpOutput(RdpOutput),
    WebOutput(WebOutput),
    GenerateReport,
}

pub trait AsReportMessage {
    fn as_report_message(self) -> ReportMessage;
}

pub fn reporting_thread(
    rx: mpsc::Receiver<ReportMessage>,
    opts: Arc<Opts>,
) -> Result<(), ()> {
    // Vecs to collect the output messages in
    let mut rdp_outputs: Vec<RdpOutput> = Vec::new();
    let mut web_outputs: Vec<WebOutput> = Vec::new();

    // Main loop listening on the channel
    while let Ok(msg) = rx.recv() {
        use ReportMessage::*;
        debug!("Received message: {:?}", msg);
        match msg {
            GenerateReport => break,
            RdpOutput(out) => rdp_outputs.push(out),
            WebOutput(out) => web_outputs.push(out),
        }
    }

    info!("Generating report");

    println!("RDP outputs: {:?}", rdp_outputs);
    println!("Web outputs: {:?}", web_outputs);

    let report_file = Path::new(&opts.output_dir).join("report.html");
    info!("Report saved to {:?}", report_file);
    Ok(())
}
