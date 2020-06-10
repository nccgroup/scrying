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

use crate::parsing::Target;
use crate::util::target_to_filename;
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::path::Path;
use std::path::PathBuf;

use std::process::Command;

// Fail if compiled witout the wkhtmltoimage feature
#[cfg(not(feature = "wkhtmltoimage"))]
pub fn capture(
    _target: &Target,
    _output_dir: &Path,
    _wkhtmltoimage_path: &Path,
) -> Result<(), ()> {
    unimplemented!();
}

#[cfg(feature = "wkhtmltoimage")]
pub fn capture(
    target: &Target,
    output_dir: &Path,
    wkhtmltoimage_path: &Path,
) -> Result<(), ()> {
    info!("Processing {}", target);

    let filename = target_to_filename(&target).unwrap();
    let filename = format!("{}.png", filename);
    let output_file = output_dir.join(filename).display().to_string();
    info!("Saving image as {}", output_file);

    let target = format!("{}", target);
    // Call the external wkhtmltoimage program
    Command::new(wkhtmltoimage_path)
        .args(&[target, output_file])
        .output()
        .expect("failed to execute wkhtmltoimage");

    Ok(())
}

#[cfg(feature = "wkhtmltoimage")]
pub fn get_wkhtmltoimage_path() -> Option<PathBuf> {
    //TODO windows?? other paths??
    let possible_paths = vec!["/usr/bin/wkhtmltoimage"];
    for possible in possible_paths {
        let path = Path::new(possible);
        if path.is_file() {
            // exists, so return it
            return Some(path.to_path_buf());
        }
    }
    None
}
