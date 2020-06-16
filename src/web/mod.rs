/*
 *   This file is part of NCC Group Scamper https://github.com/nccgroup/scamper
 *   Copyright 2020 David Young <david(dot)young(at)nccgroup(dot)com>
 *   Released as open source by NCC Group Plc - https://www.nccgroup.com
 *
 *   Scamper is free software: you can redistribute it and/or modify
 *   it under the terms of the GNU General Public License as published by
 *   the Free Software Foundation, either version 3 of the License, or
 *   (at your option) any later version.
 *
 *   Scamper is distributed in the hope that it will be useful,
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *   GNU General Public License for more details.
 *
 *   You should have received a copy of the GNU General Public License
 *   along with Scamper.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::error::Error;
use crate::parsing::Target;
use crate::util::target_to_filename;
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::path::Path;

use headless_chrome::{protocol::page::ScreenshotFormat, Tab};

use std::{fs::File, io::Write};

pub fn capture(
    target: &Target,
    output_dir: &Path,
    tab: &Tab,
) -> Result<(), Error> {
    info!("Processing {}", target);

    let filename = target_to_filename(&target).unwrap();
    let filename = format!("{}.png", filename);
    let output_file = output_dir.join(filename).display().to_string();
    info!("Saving image as {}", output_file);
    if let Target::Url(target) = target {
        tab.navigate_to(target.as_str())?;
        tab.wait_until_navigated()?;
        let png_data = tab
            .capture_screenshot(ScreenshotFormat::PNG, None, true)
            .expect("error making screenshot");
        let mut file = File::create(output_file)?;
        file.write_all(&png_data)?;
    }
    Ok(())
}
