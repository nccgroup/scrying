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

use clap::{command, Arg, ArgEnum, ArgGroup, PossibleValue};
use color_eyre::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;

lazy_static! {
    static ref SIZE_REGEX: Regex = Regex::new(r"^(\d+)x(\d+)$").unwrap();
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mode {
    Auto,
    Web,
    Rdp,
    Vnc,
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
        use Mode::{Auto, Rdp, Vnc, Web};
        match s {
            "web" => Ok(Web),
            "rdp" => Ok(Rdp),
            "vnc" => Ok(Vnc),
            "auto" => Ok(Auto),
            _ => Err("Mode must be \"auto\", \"web\" or \"rdp\""),
        }
    }
}

#[derive(ArgEnum, Copy, Clone, PartialEq, Eq, Debug)]
pub enum WebMode {
    Chrome,
    Native,
}

impl Default for WebMode {
    fn default() -> WebMode {
        WebMode::Chrome
    }
}

impl std::str::FromStr for WebMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        for variant in Self::value_variants() {
            if variant.to_possible_value().unwrap().matches(s, false) {
                return Ok(*variant);
            }
        }
        Err(format!("Invalid variant: {}", s))
    }
}

impl WebMode {
    pub fn possible_values() -> impl Iterator<Item = PossibleValue<'static>> {
        WebMode::value_variants()
            .iter()
            .filter_map(ArgEnum::to_possible_value)
    }
}

#[derive(Debug, Default)]
pub struct Opts {
    pub files: Vec<String>,
    pub targets: Vec<String>,
    pub mode: Mode,
    pub rdp_timeout: usize,
    pub threads: usize,
    pub log_file: Option<String>,
    pub nmaps: Vec<String>,
    pub nessus: Vec<String>,
    pub output_dir: String,
    pub web_proxy: Option<String>,
    pub rdp_proxy: Option<String>,
    pub vnc_auth: Option<String>,
    pub web_path: Vec<String>,
    pub size: (usize, usize),
    pub silent: bool,
    pub verbose: u64,
    pub test_import: bool,
    pub web_mode: WebMode,
}

pub fn parse() -> Result<Opts> {
    let args = command!()
        .about("Automatic RDP, Web, and VNC screenshotting tool")
        .arg_required_else_help(true)
        .arg(
            Arg::new("FILE")
                .help("Targets file, one per line")
                .long("file")
                .multiple_occurrences(true)
                .short('f')
                .takes_value(true),
        )
        .arg(
            Arg::new("TARGET")
                .help("Target, e.g. http://example.com, rdp://[2001:db8::4]")
                .long("target")
                .multiple_occurrences(true)
                .short('t')
                .takes_value(true),
        )
        .arg(
            Arg::new("MODE")
                .help("Force targets to be parsed as `web`, `rdp`, `vnc`")
                .default_value("auto")
                .long("mode")
                .possible_values(&["web", "rdp", "vnc", "auto"])
                .short('m')
                .takes_value(true),
        )
        .arg(
            Arg::new("RDP TIMEOUT")
                .help(
                    "Seconds to wait after last bitmap before saving an image",
                )
                .default_value("2")
                .long("rdp-timeout")
                .takes_value(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("THREADS")
                .help("Number of worker threads for each target type")
                .default_value("10")
                .long("threads")
                .takes_value(true),
        )
        .arg(
            Arg::new("LOG FILE")
                .help("Save logs to the given file")
                .long("log-file")
                .short('l')
                .takes_value(true),
        )
        .arg(
            Arg::new("NMAP XML FILE")
                .help("Nmap XML file")
                .long("nmap")
                .multiple_occurrences(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("NESSUS XML FILE")
                .help("Nessus XML file")
                .long("nessus")
                .multiple_occurrences(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("OUTPUT DIR")
                .help("Directory to save the captured images in")
                .default_value("output")
                .long("output")
                .short('o')
                .takes_value(true),
        )
        .arg(
            Arg::new("WEB PROXY")
                .help(concat!(
                    "HTTP/SOCKS Proxy to use for web requests",
                    " e.g. http://[::1]:8080"
                ))
                .long("web-proxy")
                .takes_value(true),
        )
        .arg(
            Arg::new("RDP PROXY")
                .help(concat!(
                    "SOCKS5 proxy to use for RDP connections",
                    " e.g. socks5://[::1]:1080"
                ))
                .long("rdp-proxy")
                .takes_value(true),
        )
        .arg(
            Arg::new("PROXY")
                .help(concat!(
                    "Default SOCKS5 proxy to use for connections",
                    " e.g. socks5://[::1]:1080"
                ))
                .long("proxy")
                .takes_value(true)
                .validator(is_socks5),
        )
        .arg(
            Arg::new("VNC AUTH")
                .help("Password to provide to VNC servers that request one")
                .long("vnc-auth")
                .takes_value(true),
        )
        .arg(
            Arg::new("WEB PATH")
                .help(concat!(
                    "Append a path to web requests. Provide multiple",
                    " to request each path sequentially"
                ))
                .long("web-path")
                .multiple_occurrences(true)
                .takes_value(true),
        )
        .arg(
            Arg::new("SIZE")
                .help(concat!(
                    "Set the size of captured images in pixels.",
                    " Due to protocol limitations, sizes greater than",
                    " 65535x65535 may get truncated in interesting ways.",
                    " This argument has no effect on VNC screenshots."
                ))
                .default_value("1280x1024")
                .long("size")
                .takes_value(true)
                .validator(size_validator),
        )
        .arg(
            Arg::new("WEB MODE")
                .help(concat!(
            "Choose between headless Chrom{e,ium} or native webview (GTK on ",
            "Linux, Edge WebView2 on Windows, Cocoa WebView on Mac"
        ))
                .default_value("chrome")
                .long("web-mode")
                .takes_value(true)
                .possible_values(WebMode::possible_values())
                .ignore_case(true),
        )
        .arg(
            Arg::new("SILENT")
                .help("Suppress most log messages")
                .long("silent")
                .short('s'),
        )
        .arg(
            Arg::new("VERBOSE")
                .help("Increase log verbosity")
                .long("verbose")
                .multiple_occurrences(true)
                .short('v')
                .takes_value(false),
        )
        .arg(
            Arg::new("TEST IMPORT")
                .help("Exit after importing targets")
                .long("test-import"),
        )
        .group(
            ArgGroup::new("inputs")
                .multiple(true)
                .required(true)
                .args(&["FILE", "NMAP XML FILE", "NESSUS XML FILE", "TARGET"]),
        )
        .get_matches();

    // Grab input files if present, otherwise an empty Vec
    let mut files: Vec<String> = Vec::new();
    if let Some(f) = args.values_of("FILE") {
        for file in f {
            files.push(file.to_string());
        }
    }

    // Grab targets if present, otherwise an empty Vec
    let mut targets: Vec<String> = Vec::new();
    if let Some(t) = args.values_of("TARGET") {
        for target in t {
            targets.push(target.to_string());
        }
    }

    // Grab Nmap files if present, otherwise an empty Vec
    let mut nmaps: Vec<String> = Vec::new();
    if let Some(n) = args.values_of("NMAP XML FILE") {
        for nmap in n {
            nmaps.push(nmap.to_string());
        }
    }

    // Grab Nessus files if present, otherwise an empty Vec
    let mut nessus: Vec<String> = Vec::new();
    if let Some(n) = args.values_of("NESSUS XML FILE") {
        for nessus_file in n {
            nessus.push(nessus_file.to_string());
        }
    }

    // If global proxy setting is configured then set all indivitual
    // proxy values to it. Then override each one in turn if applicable
    let mut web_proxy = None;
    let mut rdp_proxy = None;
    if let Some(p) = args.value_of("PROXY") {
        web_proxy = Some(p.to_string());
        rdp_proxy = Some(p.to_string());
    }

    if let Some(p) = args.value_of("RDP PROXY") {
        rdp_proxy = Some(p.to_string());
    }

    if let Some(p) = args.value_of("WEB PROXY") {
        web_proxy = Some(p.to_string());
    }

    // SIZE has already passed the regex validator, so the following
    // extraction probably won't panic
    let captures = SIZE_REGEX
        .captures_iter(args.value_of("SIZE").unwrap())
        .next()
        .unwrap();
    let size = (captures[1].parse().unwrap(), captures[2].parse().unwrap());

    Ok(Opts {
        files,
        targets,
        mode: args.value_of_t("MODE").unwrap(),
        rdp_timeout: args.value_of_t("RDP TIMEOUT").unwrap(),
        threads: args.value_of_t("THREADS").unwrap(),
        log_file: args
            .value_of("LOG FILE")
            .map_or_else(|| None, |s| Some(s.to_string())),
        nmaps,
        nessus,
        output_dir: args.value_of_t("OUTPUT DIR").unwrap(),
        web_proxy,
        rdp_proxy,
        vnc_auth: args
            .value_of("VNC AUTH")
            .map_or_else(|| None, |s| Some(s.to_string())),
        web_path: if let Some(paths) = args.values_of("WEB PATH") {
            paths.map(|p| p.to_string()).collect()
        } else {
            Vec::new()
        },
        size,
        silent: args.is_present("SILENT"),
        verbose: args.occurrences_of("VERBOSE"),
        test_import: args.is_present("TEST IMPORT"),
        web_mode: args.value_of_t("WEB MODE")?,
    })
}

fn is_socks5(val: &str) -> Result<(), String> {
    if !val.starts_with("socks5://") {
        Err("Global or RDP proxy must be a socks5:// URI".to_string())
    } else {
        Ok(())
    }
}

fn size_validator(val: &str) -> Result<(), String> {
    if SIZE_REGEX.is_match(val) {
        Ok(())
    } else {
        Err(r#"Size must be in the form "1280x720" (regex "^\d+x\d+$""#
            .to_string())
    }
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
