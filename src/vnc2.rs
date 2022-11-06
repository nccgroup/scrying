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

use crate::argparse::Mode::Vnc;
use crate::argparse::Opts;
use crate::parsing::Target;
use crate::reporting::ReportMessageContent;
use crate::reporting::{FileError, ReportMessage};
use crate::util::target_to_filename;
use crate::ThreadStatus;
#[allow(unused)]
use crate::{debug, error, info, trace, warn};
use color_eyre::{eyre::eyre, Result};
use image::{DynamicImage, ImageBuffer, Rgb};
use std::cmp::min;
use std::convert::TryInto;
use std::path::Path;
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use vnc_rs::{PixelFormat, Rect, VncConnector, VncEvent, X11Event};

async fn vnc_capture(
    target: &Target,
    opts: &Opts,
    report_tx: &Sender<ReportMessage>,
) -> Result<()> {
    todo!()
}

pub async fn capture(
    target: &Target,
    opts: &Opts,
    tx: Sender<ThreadStatus>,
    report_tx: &Sender<ReportMessage>,
) {
    if let Err(e) = vnc_capture(target, opts, report_tx).await {
        warn!(target, "VNC error: {}", e);
    }

    tx.send(ThreadStatus::Complete).await.unwrap();
}
