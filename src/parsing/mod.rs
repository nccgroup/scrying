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

use crate::argparse::{Mode, Opts};
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use nessus_xml_parser::NessusScan;
use nmap_xml_parser::{port::PortState, NmapResults};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, prelude::*, BufReader};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use url::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Target {
    Address(SocketAddr),
    //  Hostname(String),
    Url(Url),
}

// InputLists moved above the impl on Target because the impl is
// pretty long
#[derive(Default, Debug, Eq, PartialEq, PartialOrd)]
pub struct InputLists {
    pub rdp_targets: Vec<Target>,
    pub web_targets: Vec<Target>,
    pub vnc_targets: Vec<Target>,
}

impl InputLists {
    fn append(&mut self, list: &mut Self) {
        self.rdp_targets.append(&mut list.rdp_targets);
        self.web_targets.append(&mut list.web_targets);
        self.vnc_targets.append(&mut list.vnc_targets);
    }

    fn dedup(&mut self) {
        self.rdp_targets.sort();
        self.rdp_targets.dedup();
        self.web_targets.sort();
        self.web_targets.dedup();
        self.vnc_targets.sort();
        self.vnc_targets.dedup();
    }
}

impl PartialOrd for Target {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        Some(self.to_string().cmp(&rhs.to_string()))
    }
}

impl Ord for Target {
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        self.to_string().cmp(&rhs.to_string())
    }
}

impl<'a> Target {
    fn parse(input: &'a str, mode: Mode) -> Result<Vec<Self>, &'a str> {
        use url::Host;
        // Parse a &str into a Target using the mode hint to guide output.
        // It doesn't make much sense to use a URL for RDP, etc.
        use Mode::*;

        // "Auto" is not supported here because this function returns a
        // Vec of Targets and cannot tag them as RDP or Web
        assert!(mode != Auto, "Mode cannot be Auto here");

        //TODO basic auth

        // Try to match a URL format. Examples could be:
        // * http://example.com
        // * https://192.0.2.3
        // * https://[2001:db8::5]:8080
        // * rdp://192.0.2.4:3390
        // * rdp://[2001:db8:6]
        // * rdp://localhost
        if let Ok(u) = Url::parse(&input) {
            match u.scheme() {
                "http" | "https" => {
                    trace!("Parsed as HTTP/HTTPS web url");
                    if mode != Web {
                        return Err("Non-web mode requested for web-type URL");
                    }
                    return Ok(vec![Target::Url(u)]);
                }
                "rdp" => {
                    trace!("Parsed as RDP url");
                    if mode != Rdp {
                        return Err("Non-rdp mode requested for rdp-type URL");
                    }
                    let port = u.port().unwrap_or(3389);
                    let address: SocketAddr = match &u
                        .host()
                        .expect("URL expected to have host")
                    {
                        Host::Ipv4(a) => {
                            SocketAddr::from((IpAddr::V4(*a), port))
                        }
                        Host::Ipv6(a) => {
                            SocketAddr::from((IpAddr::V6(*a), port))
                        }
                        //TODO work out how to get ? to work here rather
                        // than unwrap
                        Host::Domain(d) => domain_to_sockaddr(d, port).unwrap(),
                    };
                    return Ok(vec![Target::Address(address)]);
                }
                "vnc" => {
                    //TODO code reuse
                    trace!("Parsed as VNC url");
                    if mode != Vnc {
                        return Err("Non-VNC mode requested for VNC-type URL");
                    }
                    let port = u.port().unwrap_or(5900);
                    let address: SocketAddr = match &u
                        .host()
                        .expect("URL expected to have host")
                    {
                        Host::Ipv4(a) => {
                            SocketAddr::from((IpAddr::V4(*a), port))
                        }
                        Host::Ipv6(a) => {
                            SocketAddr::from((IpAddr::V6(*a), port))
                        }
                        //TODO work out how to get ? to work here rather
                        // than unwrap
                        Host::Domain(d) => domain_to_sockaddr(d, port).unwrap(),
                    };
                    return Ok(vec![Target::Address(address)]);
                }

                _ => return Err("Invalid scheme"),
            }
        } else {
            // Handle the case where rdp://2001:db8::100 drops through
            // to the forced-prefix stage when it should fail as an
            // invalid URL
            if input.starts_with("rdp://")
                || input.starts_with("https://")
                || input.starts_with("http://")
                || input.starts_with("vnc://")
            {
                return Err("Parsing error");
            }
        }

        match mode {
            Auto => unimplemented!(), // do both
            Rdp => {
                // if no port specified then assume 3389, otherwise take
                // the provided port

                // Try forcing a parse that includes the port
                if let Ok(addr) = ip_port_to_sockaddr(&input) {
                    return Ok(vec![Target::Address(addr)]);
                }

                // If that didn't work then try parsing it as just an address
                if let Ok(addr) = domain_to_sockaddr(&input, 3389) {
                    return Ok(vec![Target::Address(addr)]);
                }

                // If none of these worked then it's probably not salvageable
                Err("Unable to parse target")
            }
            Web => {
                // add URLs for HTTP and HTTPS because we don't know
                // ahead of time which protocol it uses
                let mut targets = Vec::new();

                // Try parsing as https://$INPUT
                // if that fails then try https://[$INPUT] in case it is
                // a v6 address without square brackets

                // Try slapping an HTTP:// on the front and see whether
                // it parses
                if let Ok(u) = Url::parse(&format!("https://{}", input)) {
                    targets.push(Target::Url(u));
                } else if let Ok(u) =
                    Url::parse(&format!("https://[{}]", input))
                {
                    targets.push(Target::Url(u));
                } else {
                    //TODO include error string
                    return Err("Unable to parse HTTPS URL");
                }

                if let Ok(u) = Url::parse(&format!("http://{}", input)) {
                    targets.push(Target::Url(u));
                } else if let Ok(u) = Url::parse(&format!("http://[{}]", input))
                {
                    targets.push(Target::Url(u));
                } else {
                    //TODO include error string
                    return Err("Unable to parse HTTP URL");
                }

                Ok(targets)
            }
            Vnc => {
                // add VNC targets
                // if no port specified then assume 5900, otherwise take
                // the provided port
                //TODO code reuse

                // Try forcing a parse that includes the port
                if let Ok(addr) = ip_port_to_sockaddr(&input) {
                    return Ok(vec![Target::Address(addr)]);
                }

                // If that didn't work then try parsing it as just an address
                if let Ok(addr) = domain_to_sockaddr(&input, 5900) {
                    return Ok(vec![Target::Address(addr)]);
                }

                // If none of these worked then it's probably not salvageable
                Err("Unable to parse target")
            }
        }
    }
}

impl Display for Target {
    fn fmt(
        &self,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        match self {
            Target::Address(addr) => write!(fmt, "{}", addr),
            Target::Url(url) => write!(fmt, "{}", url),
        }
    }
}

impl Display for InputLists {
    fn fmt(
        &self,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        // Format the targets as separate lists for the different target
        // types.
        write!(fmt, "RDP targets:")?;
        if self.rdp_targets.is_empty() {
            write!(fmt, " None")?;
        }
        for t in &self.rdp_targets {
            write!(fmt, "\n    {}", t)?;
        }

        write!(fmt, "\nWeb targets:")?;
        if self.web_targets.is_empty() {
            write!(fmt, " None")?;
        }
        for t in &self.web_targets {
            write!(fmt, "\n    {}", t)?;
        }

        write!(fmt, "\nVNC targets:")?;
        if self.vnc_targets.is_empty() {
            write!(fmt, " None")?;
        }
        for t in &self.vnc_targets {
            write!(fmt, "\n    {}", t)?;
        }

        Ok(())
    }
}

fn domain_to_sockaddr(
    domain: &str,
    port: u16,
) -> Result<SocketAddr, io::Error> {
    // It's currently the case that "rdp://192.0.2.1"
    // gets parsed as a domain rather than an IPv4
    // address. This is due to oddities in the URL
    // standard that servo/rust-url is following and
    // does not look like it will be resolved in a
    // way that is favourable to applications like
    // this because they are coming from a web-
    // focused mindset and we are trying to parse
    // thoroughly non-web URLs.
    //
    // To bypass this, if the host looks like a
    // domain then try to parse it as an IPv4
    // address. We specifically exclude IPv6 here as
    // that is currently parsed correctly from the
    // URL and the tests failing will act as an
    // interesting canary.

    // Try to resolve the domain to an IP-port combination. The domain
    // in theory should not have a port alongside it, so this should
    // "just work", provided the domain resolves to a valid address.
    let mut addrs = (domain, port).to_socket_addrs()?;

    if let Some(sockaddr) = addrs.next() {
        return Ok(sockaddr);
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Unknown error resolving domain",
    ))
}

fn host_to_socketaddr(host: &str, port: u16) -> Result<SocketAddr, io::Error> {
    // The nessus file just gives us the "host name" as a string, which
    // could be an IP address, a legacy IP address, a DNS name, or maybe
    // even something else entirely. We try to parse it as each type of
    // thing and see what happens.

    let mut addrs = (host, port).to_socket_addrs()?;

    if let Some(sockaddr) = addrs.next() {
        Ok(sockaddr)
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Unknown error resolving {}", host),
        ))
    }
}

fn ip_port_to_sockaddr(input: &str) -> Result<SocketAddr, io::Error> {
    let mut addrs = input.to_socket_addrs()?;

    if let Some(sockaddr) = addrs.next() {
        return Ok(sockaddr);
    }

    Err(io::Error::new(
        io::ErrorKind::Other,
        "Invalid sockaddr string",
    ))
}

pub fn generate_target_lists(opts: &Opts) -> InputLists {
    use Mode::*;
    let mut input_lists: InputLists = Default::default();

    // Process the optional command-line target argument
    for t in &opts.targets {
        let mut parse_successful = false;
        match &opts.mode {
            Auto => {
                // Try parsing as both web and RDP, saving any that stick
                if let Ok(mut targets) = Target::parse(&t, Rdp) {
                    input_lists.rdp_targets.append(&mut targets);
                    parse_successful = true;
                    debug!("{} parsed as RDP target", t);
                }
                if let Ok(mut targets) = Target::parse(&t, Web) {
                    input_lists.web_targets.append(&mut targets);
                    parse_successful = true;
                    debug!("{} parsed as Web target", t);
                }
                if let Ok(mut targets) = Target::parse(&t, Vnc) {
                    input_lists.vnc_targets.append(&mut targets);
                    parse_successful = true;
                    debug!("{} parsed as VNC target", t);
                }
            }
            Web => {
                if let Ok(mut targets) = Target::parse(&t, Web) {
                    input_lists.web_targets.append(&mut targets);
                    parse_successful = true;
                    debug!("{} parsed as Web target", t);
                }
            }
            Rdp => {
                if let Ok(mut targets) = Target::parse(&t, Rdp) {
                    input_lists.rdp_targets.append(&mut targets);
                    parse_successful = true;
                    debug!("{} parsed as RDP target", t);
                }
            }
            Vnc => {
                if let Ok(mut targets) = Target::parse(&t, Vnc) {
                    input_lists.vnc_targets.append(&mut targets);
                    parse_successful = true;
                    debug!("{} parsed as VNC target", t);
                }
            }
        }
        if !parse_successful {
            warn!("Unable to parse {}", t);
        }
    }

    // Process the optional input file
    for file_name in &opts.files {
        let mut parse_successful_count: usize = 0;
        let mut parse_total_count: usize = 0;
        let mut parse_unsuccessful_count: usize = 0;
        // This is horribly deep nesting, but it has to try opening the
        // provided file, iterate over a reader, parse each line into a
        // string type and then behave slightly differently depending on
        // the selected mode, failing gracefully at each stage if any
        // errors occur.
        //TODO make nesting less deep somehow
        match File::open(file_name) {
            Ok(file) => {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    debug!("Reading target {:?}", line);
                    match line {
                        Ok(t) => {
                            // Try to parse the line into a Target
                            parse_total_count += 1;

                            match &opts.mode {
                                Auto => {
                                    // Try parsing as web, RDP, and VNC,
                                    // saving any that stick
                                    let mut success = false;
                                    if let Ok(mut targets) =
                                        Target::parse(&t, Rdp)
                                    {
                                        input_lists
                                            .rdp_targets
                                            .append(&mut targets);
                                        parse_successful_count += 1;
                                        success = true;
                                        info!("{} loaded as RDP target", t);
                                    }
                                    if let Ok(mut targets) =
                                        Target::parse(&t, Web)
                                    {
                                        input_lists
                                            .web_targets
                                            .append(&mut targets);
                                        parse_successful_count += 1;
                                        success = true;
                                        info!("{} loaded as Web target", t);
                                    }
                                    if let Ok(mut targets) =
                                        Target::parse(&t, Vnc)
                                    {
                                        input_lists
                                            .vnc_targets
                                            .append(&mut targets);
                                        parse_successful_count += 1;
                                        success = true;
                                        info!("{} loaded as VNC target", t);
                                    }
                                    if !success {
                                        warn!("Unable to parse {}", t);
                                        parse_unsuccessful_count += 1;
                                    }
                                }
                                Web => {
                                    if let Ok(mut targets) =
                                        Target::parse(&t, Web)
                                    {
                                        input_lists
                                            .web_targets
                                            .append(&mut targets);
                                        parse_successful_count += 1;
                                        info!("{} loaded as Web target", t);
                                    } else {
                                        warn!(
                                            "{} is not a valid Web target",
                                            t
                                        );
                                        parse_unsuccessful_count += 1;
                                    }
                                }
                                Rdp => {
                                    if let Ok(mut targets) =
                                        Target::parse(&t, Rdp)
                                    {
                                        input_lists
                                            .rdp_targets
                                            .append(&mut targets);
                                        parse_successful_count += 1;
                                        info!("{} loaded as RDP target", t);
                                    } else {
                                        warn!(
                                            "{} is not a valid RDP target",
                                            t
                                        );
                                        parse_unsuccessful_count += 1;
                                    }
                                }
                                Vnc => {
                                    if let Ok(mut targets) =
                                        Target::parse(&t, Vnc)
                                    {
                                        input_lists
                                            .vnc_targets
                                            .append(&mut targets);
                                        parse_successful_count += 1;
                                        info!("{} loaded as VNC target", t);
                                    } else {
                                        warn!(
                                            "{} is not a valid RDP target",
                                            t
                                        );
                                        parse_unsuccessful_count += 1;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Error reading line {}", e);
                            parse_unsuccessful_count += 1;
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Error opening file: {:?}", e);
            }
        }
        info!(
            "Loaded {} targets from {} lines from {} with {} errors",
            parse_successful_count,
            parse_total_count,
            file_name,
            parse_unsuccessful_count,
        );
    }

    // Parse nmap file
    for file in &opts.nmaps {
        info!("Loading nmap file {}", file);

        match fs::read_to_string(file) {
            Err(e) => {
                warn!("Error opening file: {}", e);
            }
            Ok(content) => {
                match NmapResults::parse(&content) {
                    Err(e) => {
                        warn!("Error parsing nmap file: {}", e);
                    }
                    Ok(results) => {
                        debug!("Successfully parsed file");
                        //TODO filter for host being UP
                        for (host, port) in results.iter_ports() {
                            // for each host check for some common open ports
                            // and add relevant ones to the list

                            // this has been broken out into a separate function
                            // for readability
                            input_lists.append(&mut lists_from_nmap(
                                host, port, &opts,
                            ));
                        }
                    }
                }
            }
        }
    }

    // Parse nessus file
    for file in &opts.nessus {
        info!("Loading nessus file {}", file);

        match fs::read_to_string(file) {
            Err(e) => {
                warn!("Error opening file: {}", e);
            }
            Ok(content) => {
                match NessusScan::parse(&content) {
                    Err(e) => {
                        warn!("Error parsing nessus file: {}", e);
                    }
                    Ok(results) => {
                        debug!("Successfully parsed file");
                        //TODO filter for host being UP
                        for (host, port) in results.ports() {
                            // for each host check for some common open ports
                            // and add relevant ones to the list

                            // this has been broken out into a separate function
                            // for readability
                            input_lists.append(&mut lists_from_nessus(
                                host, port, &opts.mode,
                            ));
                        }
                    }
                }
            }
        }
    }

    // Put in web paths
    let mut additional_web_targets =
        Vec::with_capacity(input_lists.web_targets.len() * opts.web_path.len());
    for target in &input_lists.web_targets {
        for path in &opts.web_path {
            if let Target::Url(ref u) = target {
                let mut u = u.clone();
                u.set_path(&path);
                additional_web_targets.push(Target::Url(u));
            }
        }
    }
    input_lists.web_targets.append(&mut additional_web_targets);

    input_lists.dedup();
    input_lists
}

fn lists_from_nmap(
    host: &nmap_xml_parser::host::Host,
    port: &nmap_xml_parser::port::Port,
    opts: &Opts,
) -> InputLists {
    use nmap_xml_parser::host::Address;

    let mut list: InputLists = Default::default();

    //TODO service discovery for ports identified as
    // "web", etc.
    //TODO break this out into a function
    //TODO code reuse
    debug!("Parsing host {:?}", (host, port));
    if port.status.state == PortState::Open {
        debug!("open port");
        // Found an open port, now add it to the
        // input lists if it is appropriate
        //TODO identify Web
        let service_name = if let Some(info) = &port.service_info {
            info.name.as_str()
        } else {
            ""
        };
        match (port.port_number, service_name) {
            // RDP signatures
            (3389, _) | (_, "ms-wbt-server")
                if opts.mode.selected(Mode::Rdp) =>
            {
                debug!("Identified RDP");
                let port = port.port_number;
                // Iterate over the host's addresses. It may have multiple
                // IPv6, IPv4, and MAC addresses and we want to add them
                // all (well, maybe not the MAC addresses)
                for address in host.addresses() {
                    let target_string = match address {
                        Address::IpAddr(IpAddr::V6(a)) => {
                            trace!("address: {:?}", a);
                            format!("[{}]:{}", a, port)
                        }
                        Address::IpAddr(IpAddr::V4(a)) => {
                            trace!("legacy address: {:?}", a);
                            format!("{}:{}", a, port)
                        }
                        Address::MacAddr(a) => {
                            trace!("Ignoring MAC address {}", a);
                            // Ignore the MAC address and move on
                            continue;
                        }
                    };

                    // target_string now contains a string sockaddr
                    // representation, so we parse it as RDP and see what
                    // happens
                    match Target::parse(&target_string, Mode::Rdp) {
                        Ok(mut target) => {
                            debug!("Successfully parsed as RDP");
                            list.rdp_targets.append(&mut target);
                        }
                        Err(e) => {
                            warn!("Error parsing target as RDP: {}", e);
                        }
                    }
                }
            }
            // HTTP(S) signatures
            (80, _)
            | (443, _)
            | (631, _)
            | (7443, _)
            | (8080, _)
            | (8443, _)
            | (8000, _)
            | (3000, _)
            | (_, "http")
            | (_, "http-mgt")
            | (_, "https")
            | (_, "http-alt")
            | (_, "https-alt")
                if opts.mode.selected(Mode::Web) =>
            {
                debug!("Idenfified web");
                let port = port.port_number;
                // Iterate over the host's addresses. It may have multiple
                // IPv6, IPv4, and MAC addresses and we want to add them
                // all (well, maybe not the MAC addresses)
                for address in host.addresses() {
                    let target_string = match address {
                        Address::IpAddr(IpAddr::V6(a)) => {
                            trace!("address: {:?}", a);
                            format!("[{}]:{}", a, port)
                        }
                        Address::IpAddr(IpAddr::V4(a)) => {
                            trace!("legacy address: {:?}", a);
                            format!("{}:{}", a, port)
                        }
                        Address::MacAddr(a) => {
                            trace!("Ignoring MAC address {}", a);
                            // Ignore the MAC address and move on
                            continue;
                        }
                    };

                    // target_string now contains a string sockaddr
                    // representation, so we parse it as Web and see what
                    // happens
                    match Target::parse(&target_string, Mode::Web) {
                        Ok(mut target) => {
                            debug!("Successfully parsed as Web");
                            list.web_targets.append(&mut target);
                        }
                        Err(e) => {
                            warn!("Error parsing target as Web: {}", e);
                        }
                    }
                }
            }
            // VNC signatures
            (5900, _)
            | (5901, _)
            | (5902, _)
            | (5903, _)
            | (_, "vnc")
            | (_, "vnc-1")
            | (_, "vnc-2")
            | (_, "vnc-3")
                if opts.mode.selected(Mode::Vnc) =>
            {
                debug!("Identified VNC");
                let port = port.port_number;
                // Iterate over the host's addresses. It may have multiple
                // IPv6, IPv4, and MAC addresses and we want to add them
                // all (well, maybe not the MAC addresses)
                for address in host.addresses() {
                    let target_string = match address {
                        Address::IpAddr(IpAddr::V6(a)) => {
                            trace!("address: {:?}", a);
                            format!("[{}]:{}", a, port)
                        }
                        Address::IpAddr(IpAddr::V4(a)) => {
                            trace!("legacy address: {:?}", a);
                            format!("{}:{}", a, port)
                        }
                        Address::MacAddr(a) => {
                            trace!("Ignoring MAC address {}", a);
                            // Ignore the MAC address and move on
                            continue;
                        }
                    };

                    // target_string now contains a string sockaddr
                    // representation, so we parse it as RDP and see what
                    // happens
                    match Target::parse(&target_string, Mode::Vnc) {
                        Ok(mut target) => {
                            debug!("Successfully parsed as VNC");
                            list.vnc_targets.append(&mut target);
                        }
                        Err(e) => {
                            warn!("Error parsing target as VNC: {}", e);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    list
}

fn lists_from_nessus(
    host: &nessus_xml_parser::ReportHost,
    port: nessus_xml_parser::Port,
    mode: &Mode,
) -> InputLists {
    let mut list: InputLists = Default::default();

    debug!("Parsing host: {}, port: {}", host, port.id);

    // Interpret the host.name as an address or hostname
    if let Ok(target) = host_to_socketaddr(&host.name, port.id) {
        //let target_string = format!("{}", target);
        match (port.id, port.service.as_str()) {
            (3389, _) | (_, "msrdp") if mode.selected(Mode::Rdp) => {
                debug!("Identified RDP");
                list.rdp_targets.push(Target::Address(target));
            }
            (80, _)
            | (443, _)
            | (631, _)
            | (7443, _)
            | (8080, _)
            | (8443, _)
            | (8000, _)
            | (3000, _)
            | (_, "www")
            | (_, "https?")
                if mode.selected(Mode::Web) =>
            {
                debug!("Identified Web");
                list.web_targets.push(Target::Address(target));
            }
            (5900, _) | (5901, _) | (5902, _) | (5903, _) | (_, "vnc")
                if mode.selected(Mode::Vnc) =>
            {
                debug!("Identified VNC");
                list.vnc_targets.push(Target::Address(target));
            }
            _ => {}
        }
    }

    list
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn parse_target_as_url() {
        use Mode::{Rdp, Vnc, Web};
        let test_cases: Vec<(&str, Target, Mode)> = vec![
            (
                "http://example.com",
                Target::Url(Url::parse("http://example.com").unwrap()),
                Web,
            ),
            (
                "http://[2001:db8::1]",
                Target::Url(Url::parse("http://[2001:db8::1]").unwrap()),
                Web,
            ),
            (
                "https://192.0.2.3",
                Target::Url(Url::parse("https://192.0.2.3").unwrap()),
                Web,
            ),
            (
                "https://[2001:db8::5]:8080",
                Target::Url(Url::parse("https://[2001:db8::5]:8080").unwrap()),
                Web,
            ),
            (
                "rdp://192.0.2.4:3390",
                Target::Address(
                    "192.0.2.4:3390".to_socket_addrs().unwrap().next().unwrap(),
                ),
                Rdp,
            ),
            (
                "rdp://[2001:db8::6]",
                Target::Address(
                    "[2001:db8::6]:3389"
                        .to_socket_addrs()
                        .unwrap()
                        .next()
                        .unwrap(),
                ),
                Rdp,
            ),
            (
                "vnc://[2001:db8::6]",
                Target::Address(
                    "[2001:db8::6]:5900"
                        .to_socket_addrs()
                        .unwrap()
                        .next()
                        .unwrap(),
                ),
                Vnc,
            ),
        ];

        for case in test_cases {
            eprintln!("Test case: {:?}", case);
            let parsed = Target::parse(&case.0, case.2).unwrap();
            assert_eq!(parsed.len(), 1, "Parsed wrong number of addresses");
            assert_eq!(parsed[0], case.1,);
        }
    }

    #[test]
    fn parse_target_as_url_with_domain() {
        use Mode::Rdp;

        let u = "rdp://localhost";

        let possible_addresses = vec![
            Target::Address(
                "[::1]:3389".to_socket_addrs().unwrap().next().unwrap(),
            ),
            Target::Address(
                "127.0.0.1:3389".to_socket_addrs().unwrap().next().unwrap(),
            ),
        ];

        let parsed = Target::parse(u, Rdp).unwrap();
        assert_eq!(parsed.len(), 1, "Parsed wrong number of addresses");
        assert!(
            possible_addresses.contains(&parsed[0]),
            "Unable to resolve URL to address"
        );
    }

    #[test]
    fn parse_target_from_ip() {
        use Mode::{Rdp, Web};

        let test_cases: Vec<(&str, Target, Mode)> = vec![
            (
                "192.0.2.4",
                Target::Address(
                    "192.0.2.4:3389".to_socket_addrs().unwrap().next().unwrap(),
                ),
                Rdp,
            ),
            (
                "192.0.2.5:3390",
                Target::Address(
                    "192.0.2.5:3390".to_socket_addrs().unwrap().next().unwrap(),
                ),
                Rdp,
            ),
            (
                "2001:db8::100",
                Target::Address(
                    "[2001:db8::100]:3389"
                        .to_socket_addrs()
                        .unwrap()
                        .next()
                        .unwrap(),
                ),
                Rdp,
            ),
            (
                "[2001:db8::101]:3000",
                Target::Address(
                    "[2001:db8::101]:3000"
                        .to_socket_addrs()
                        .unwrap()
                        .next()
                        .unwrap(),
                ),
                Rdp,
            ),
        ];
        let vec_test_cases: Vec<(&str, Vec<Target>, Mode)> = vec![
            (
                "2001:db8::1",
                vec![
                    Target::Url(Url::parse("https://[2001:db8::1]").unwrap()),
                    Target::Url(Url::parse("http://[2001:db8::1]").unwrap()),
                ],
                Web,
            ),
            /*( // TODO
                "fe80::24%ens0",
                vec![
                    Target::Url(Url::parse("https://[2001:db8::1]").unwrap()),
                    Target::Url(Url::parse("http://[2001:db8::1]").unwrap()),
                ],
                Web,
            ),*/
            (
                "[2001:db8::1]",
                vec![
                    Target::Url(Url::parse("https://[2001:db8::1]").unwrap()),
                    Target::Url(Url::parse("http://[2001:db8::1]").unwrap()),
                ],
                Web,
            ),
            (
                "[2001:db8::1]:8080",
                vec![
                    Target::Url(
                        Url::parse("https://[2001:db8::1]:8080").unwrap(),
                    ),
                    Target::Url(
                        Url::parse("http://[2001:db8::1]:8080").unwrap(),
                    ),
                ],
                Web,
            ),
            (
                "192.0.2.14",
                vec![
                    Target::Url(Url::parse("https://192.0.2.14").unwrap()),
                    Target::Url(Url::parse("http://192.0.2.14").unwrap()),
                ],
                Web,
            ),
            (
                "192.0.2.14:8443",
                vec![
                    Target::Url(Url::parse("https://192.0.2.14:8443").unwrap()),
                    Target::Url(Url::parse("http://192.0.2.14:8443").unwrap()),
                ],
                Web,
            ),
        ];

        for case in test_cases {
            eprintln!("Test case: {:?}", case);
            let parsed = Target::parse(&case.0, case.2).unwrap();
            assert_eq!(parsed.len(), 1, "Parsed wrong number of addresses");
            assert_eq!(parsed[0], case.1,);
        }

        for case in vec_test_cases {
            eprintln!("Test case: {:?}", case);
            let parsed = Target::parse(&case.0, case.2).unwrap();

            // Each address should result in an HTTPS and HTTP URL
            assert_eq!(parsed.len(), 2, "Parsed wrong number of addresses");

            assert_eq!(parsed, case.1,);
        }
    }

    #[test]
    fn parse_invalid_addresses() {
        use Mode::{Rdp, Web};
        let test_cases: Vec<(&str, Mode)> = vec![
            ("http://192.0.2.4", Rdp),
            ("http://192.0.2.5:3390", Rdp),
            ("rdp://2001:db8::100", Web),
            ("rdp://[2001:db8::101]:3000", Web),
            // These get treated as hostnames and time out on DNS
            // resolution, which is probably okay
            //("10.0.0.0.0.1", Rdp),
            //("2001:db8", Web),
        ];

        for case in test_cases {
            eprintln!("Test case: {:?}", case);

            let result = Target::parse(case.0, case.1);
            eprintln!("Result: {:?}", result);
            assert!(result.is_err());
        }
    }

    #[test]
    fn target_lists_from_cli_target() {
        use Mode::{Auto, Rdp, Web};
        // Don't need to do such thorough testing a for Target::parse
        // because a lot of the code is the same. Just need valid web
        // and RDP addresses for the dedicated and auto modes, as well
        // a sample of invalid cases
        let mut opts: Opts = Default::default();

        let test_cases: Vec<(&str, InputLists, Mode)> = vec![
            ("rdp://192.0.2.1", Default::default(), Web),
            ("http://192.0.2.1", Default::default(), Rdp),
            (
                "rdp://[2001:db8::6]:3300",
                InputLists {
                    rdp_targets: vec![Target::Address(
                        "[2001:db8::6]:3300"
                            .to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                    )],
                    web_targets: Vec::new(),
                    vnc_targets: Vec::new(),
                },
                Rdp,
            ),
            (
                "rdp://[2001:db8::6]:3300",
                InputLists {
                    rdp_targets: vec![Target::Address(
                        "[2001:db8::6]:3300"
                            .to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                    )],
                    web_targets: Vec::new(),
                    vnc_targets: Vec::new(),
                },
                Auto,
            ),
            (
                "https://[2001:db8::6]:8080",
                InputLists {
                    rdp_targets: Vec::new(),
                    web_targets: vec![Target::Url(
                        Url::parse("https://[2001:db8::6]:8080").unwrap(),
                    )],
                    vnc_targets: Vec::new(),
                },
                Web,
            ),
            (
                "https://[2001:db8::6]",
                InputLists {
                    rdp_targets: Vec::new(),
                    web_targets: vec![Target::Url(
                        Url::parse("https://[2001:db8::6]").unwrap(),
                    )],
                    vnc_targets: Vec::new(),
                },
                Auto,
            ),
            (
                "2001:db8::6",
                InputLists {
                    rdp_targets: Vec::new(),
                    web_targets: vec![
                        Target::Url(
                            Url::parse("http://[2001:db8::6]").unwrap(),
                        ),
                        Target::Url(
                            Url::parse("https://[2001:db8::6]").unwrap(),
                        ),
                    ],
                    vnc_targets: Vec::new(),
                },
                Web,
            ),
            (
                "[2001:db8::6]:3300",
                InputLists {
                    rdp_targets: vec![Target::Address(
                        "[2001:db8::6]:3300"
                            .to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                    )],
                    web_targets: Vec::new(),
                    vnc_targets: Vec::new(),
                },
                Rdp,
            ),
            (
                "[2001:db8::6]:3300",
                InputLists {
                    rdp_targets: vec![Target::Address(
                        "[2001:db8::6]:3300"
                            .to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                    )],
                    web_targets: vec![
                        Target::Url(
                            Url::parse("http://[2001:db8::6]:3300").unwrap(),
                        ),
                        Target::Url(
                            Url::parse("https://[2001:db8::6]:3300").unwrap(),
                        ),
                    ],
                    vnc_targets: vec![Target::Address(
                        "[2001:db8::6]:3300"
                            .to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                    )],
                },
                Auto,
            ),
        ];

        for (input, input_lists, mode) in test_cases {
            eprintln!("Test case: {:?}", (input, &input_lists, mode));
            opts.targets = vec![input.into()];
            opts.mode = mode;

            let parsed = generate_target_lists(&opts);

            assert_eq!(parsed, input_lists);
        }
    }

    #[test]
    fn load_from_nmap_xml() {
        // Load xml from a file and parse it
        let test_cases = vec![(
            "test/nmap.xml",
            InputLists {
                rdp_targets: vec![
                    Target::Address(
                        "172.24.5.57:3389"
                            .to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                    ),
                    Target::Address(
                        "192.168.59.146:3389"
                            .to_socket_addrs()
                            .unwrap()
                            .next()
                            .unwrap(),
                    ),
                ],
                web_targets: vec![
                    Target::Url(
                        Url::parse("http://192.168.59.128:8000/").unwrap(),
                    ),
                    Target::Url(Url::parse("http://192.168.59.146/").unwrap()),
                    Target::Url(
                        Url::parse("https://192.168.59.128:8000/").unwrap(),
                    ),
                    Target::Url(
                        Url::parse("https://192.168.59.146:80/").unwrap(),
                    ),
                ],
                vnc_targets: Vec::new(),
            },
        )];
        let mut opts: Opts = Default::default();
        for case in test_cases {
            eprintln!("Test case: {:?}", case);
            opts.nmaps = vec![case.0.into()];
            let parsed = generate_target_lists(&opts);
            eprintln!("Parsed: {:?}", parsed);

            assert_eq!(parsed, case.1);
        }
    }

    #[test]
    fn display_impl_for_target() {
        let test_cases = vec![
            (
                Target::Url(Url::parse("https://[2001:db8::6]").unwrap()),
                "https://[2001:db8::6]/",
            ),
            (
                Target::Url(Url::parse("https://192.0.2.3").unwrap()),
                "https://192.0.2.3/",
            ),
            (
                Target::Address(
                    "[::1]:3389".to_socket_addrs().unwrap().next().unwrap(),
                ),
                "[::1]:3389",
            ),
            (
                Target::Address(
                    "127.0.0.1:3389".to_socket_addrs().unwrap().next().unwrap(),
                ),
                "127.0.0.1:3389",
            ),
        ];

        for case in test_cases {
            eprintln!("Test case: {:?}", case);

            let disp = format!("{}", case.0);
            assert_eq!(disp, case.1);
        }
    }
}
