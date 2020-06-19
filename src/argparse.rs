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

use clap::Clap;
use std::str::FromStr;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mode {
    Auto,
    Web,
    Rdp,
}

impl Mode {
    /// Determine whether the supplied mode filter is valid for the
    /// current mode. Combinations are:
    /// Mode::Auto -> all filters valid
    /// Mode::X -> only X and auto are valid
    pub fn selected(&self, filter: Self) -> bool {
        use Mode::*;
        self == &Auto || self == &filter || filter == Auto
    }
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Auto
    }
}

impl FromStr for Mode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Mode::{Auto, Rdp, Web};
        match s {
            "web" => Ok(Web),
            "rdp" => Ok(Rdp),
            "auto" => Ok(Auto),
            _ => Err("Mode must be \"auto\", \"web\" or \"rdp\""),
        }
    }
}

#[derive(Clap, Debug, Default)]
#[clap(version = "0.3.0", author = "David Y. <david.young@nccgroup.com>")]
pub struct Opts {
    #[clap(short, long, about = "Targets file, one per line")]
    pub file: Vec<String>,

    #[clap(short, long, about = "Target, e.g. http://example.com")]
    pub target: Vec<String>,

    #[clap(
        short,
        long,
        default_value = "auto",
        about = "Force `web` or `rdp`"
    )]
    pub mode: Mode,

    #[clap(
        long,
        default_value = "10",
        about = "How long after last bitmap to wait before saving image"
    )]
    pub rdp_timeout: usize,

    #[clap(
        long,
        default_value = "3",
        about = "Number of worker threads for each target type"
    )]
    pub threads: usize,

    #[clap(short, long, about = "Save logs to file")]
    pub log_file: Option<String>,

    #[clap(long, about = "Nmap XML file")]
    pub nmap: Vec<String>,

    #[clap(
        short,
        long,
        default_value = "output",
        about = "Directory to save the captured images in"
    )]
    pub output_dir: String,

    #[clap(long, about = "Proxy to use for web requests")]
    pub web_proxy: Option<String>,

    #[clap(long, about = "Proxy to use for RDP connections")]
    pub rdp_proxy: Option<String>,

    #[clap(short, long, about = "Suppress most log messages")]
    pub silent: bool,

    #[clap(
        short,
        long,
        parse(from_occurrences),
        about = "Increase log verbosity"
    )]
    pub verbose: u8,

    #[clap(long, about = "Exit after importing targets")]
    pub test_import: bool,
}

pub fn parse() -> Opts {
    Opts::parse()
}

#[cfg(test)]
mod test {
    #[test]
    fn mode_filter() {
        use super::Mode::*;

        let auto = Auto;
        let rdp = Rdp;
        let web = Web;

        assert!(auto.selected(Auto));
        assert!(auto.selected(Rdp));
        assert!(auto.selected(Web));

        assert!(rdp.selected(Auto));
        assert!(rdp.selected(Rdp));
        assert!(!rdp.selected(Web));

        assert!(web.selected(Auto));
        assert!(!web.selected(Rdp));
        assert!(web.selected(Web));
    }
}
