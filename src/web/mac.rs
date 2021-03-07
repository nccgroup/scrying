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

use super::{HEIGHT, WIDTH};
use crate::reporting::{FileError, ReportMessage, ReportMessageContent};
use crate::{
    argparse::Mode::Web, error::Error, parsing::Target,
    util::target_to_filename,
};
use crate::{InputLists, Opts};
use headless_chrome::{
    protocol::page::ScreenshotFormat, Browser, LaunchOptionsBuilder, Tab,
};

#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::collections::HashMap;
use std::convert::TryInto;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::{fs::File, io::Write};

pub fn web_worker(
    targets: Arc<InputLists>,
    opts: Arc<Opts>,
    report_tx: mpsc::Sender<ReportMessage>,
    caught_ctrl_c: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut chrome_env = HashMap::new();
    if let Some(p) = &opts.web_proxy {
        chrome_env.insert("http_proxy".to_string(), p.clone());
        chrome_env.insert("https_proxy".to_string(), p.clone());
    }
    let launch_options = LaunchOptionsBuilder::default()
        .headless(true)
        .window_size(Some((
            WIDTH.try_into().unwrap(),
            HEIGHT.try_into().unwrap(),
        )))
        .process_envs(Some(chrome_env))
        .args(vec![OsStr::new("--ignore-certificate-errors")])
        .build()?;
    let browser = Browser::new(launch_options).expect("failed to init chrome");
    let tab = browser.wait_for_initial_tab().expect("Failed to init tab");

    for target in &targets.web_targets {
        if caught_ctrl_c.load(Ordering::SeqCst) {
            break;
        }
        if let Err(e) = capture(target, &opts.output_dir, &tab, &report_tx) {
            match e {
                Error::IoError(e) => {
                    // Should probably abort on an IO error
                    error!("IO error: {}", e);
                    break;
                }
                Error::ChromeError(e) => {
                    warn!("Failed to capture image: {}", e);
                }
                _ => unreachable!(),
            }
        }
    }
    Ok(())
}

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
