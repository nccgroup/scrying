/*
 *   This file is part of NCC Group Scrying https://github.com/nccgroup/scrying
 *   Copyright 2020 David Young <david(dot)young(at)nccgroup(dot)com>
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

use crate::argparse::Mode::Web;
use crate::error::Error;
use crate::parsing::Target;
use crate::reporting::ReportMessageContent;
use crate::reporting::{FileError, ReportMessage};
use crate::util::target_to_filename;
use headless_chrome::{protocol::page::ScreenshotFormat, Tab};
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::path::Path;
use std::sync::mpsc;
use std::{fs::File, io::Write};

pub fn capture(
    target: &Target,
    output_dir: &str,
    tab: &Tab,
    report_tx: &mpsc::Sender<ReportMessage>,
) -> Result<(), Error> {
    info!("Processing {}", target);

    let filename = format!("{}.png", target_to_filename(&target));

    let relative_filepath = Path::new("web").join(&filename);
    let output_file = Path::new(output_dir).join(&relative_filepath);
    info!("Saving image as {}", output_file.display());
    if let Target::Url(target) = target {
        tab.navigate_to(target.as_str())?;
        tab.wait_until_navigated()?;
        let png_data = tab
            .capture_screenshot(ScreenshotFormat::PNG, None, true)
            .expect("error making screenshot");
        let mut file = File::create(&output_file)?;
        file.write_all(&png_data)?;

        let report_message = ReportMessage::Output(ReportMessageContent {
            mode: Web,
            target: target.to_string(),
            output: FileError::File(relative_filepath.display().to_string()),
        });
        report_tx.send(report_message)?;
    }
    Ok(())
}

pub fn save(
    target: &Target,
    output_dir: &str,
    png_data: &[u8],
    report_tx: &mpsc::Sender<ReportMessage>,
) -> Result<(), Error> {
    debug!("Saving image for {}", target);

    let filename = format!("{}.png", target_to_filename(&target));

    let relative_filepath = Path::new("web").join(&filename);
    let output_file = Path::new(output_dir).join(&relative_filepath);
    info!("Saving image as {}", output_file.display());

    let mut file = File::create(&output_file)?;
    file.write_all(&png_data)?;

    let report_message = ReportMessage::Output(ReportMessageContent {
        mode: Web,
        target: target.to_string(),
        output: FileError::File(relative_filepath.display().to_string()),
    });
    report_tx.send(report_message)?;

    Ok(())
}
