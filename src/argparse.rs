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

use clap::Clap;
use std::str::FromStr;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mode {
    Auto,
    Web,
    Rdp,
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
#[clap(version = "0.1", author = "David Y. <david.young@nccgroup.com>")]
pub struct Opts {
    #[clap(short, long)]
    pub input: Option<String>,

    #[clap(short, long)]
    pub file: Option<String>,

    #[clap(short, long)]
    pub target: Option<String>,

    #[clap(short, long, default_value = "auto")]
    pub mode: Mode,

    #[clap(short, long)]
    pub log_file: Option<String>,

    #[clap(short, long, default_value = "output")]
    pub output_dir: String,

    #[clap(short, long)]
    pub silent: bool,

    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u8,
}

pub fn parse() -> Opts {
    Opts::parse()
}
