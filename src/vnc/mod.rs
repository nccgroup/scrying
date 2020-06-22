#![allow(unused)]


use crate::argparse::Opts;
use crate::error::Error;
use crate::parsing::Target;
use crate::reporting::{AsReportMessage, ReportMessage};
use crate::util::target_to_filename;
use crate::ThreadStatus;
use image::{DynamicImage, ImageBuffer, Rgba};
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::net::TcpStream;
use std::sync::{mpsc, mpsc::Receiver, mpsc::Sender};
use vnc::client::{AuthChoice, AuthMethod, Client};

fn vnc_capture(
    target: &Target,
    opts: &Opts,
    report_tx: &mpsc::Sender<ReportMessage>,
) -> Result<(), Error> {
    info!("Connecting to {:?}", target);
    let addr = match target {
        Target::Address(sock_addr) => sock_addr,
        Target::Url(_) => {
            return Err(Error::VncError(format!(
                "Invalid VNC target: {}",
                target
            )));
        }
    };

    Ok(())
}

pub fn capture(
    target: Target,
    opts: &Opts,
    tx: mpsc::Sender<ThreadStatus>,
    report_tx: &mpsc::Sender<ReportMessage>,
) {
    if let Err(e) = vnc_capture(&target, opts, report_tx) {
        warn!("VNC error: {}", e);
    }

    tx.send(ThreadStatus::Complete).unwrap();
}
