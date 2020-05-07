use crate::argparse::{Mode, Opts};
#[allow(unused)]
use log::{debug, error, info, trace, warn};
use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use url::{Host, Url};

#[derive(Debug, PartialEq)]
pub enum Target {
    Address(SocketAddr),
    Hostname(String),
    Url(Url),
}

impl Target {
    fn parse(input: &str, mode: Mode) -> Result<Vec<Self>, &str> {
        // Parse a &str into a Target using the mode hint to guide output.
        // It doesn't make much sense to use a URL for RDP, etc.
        use Mode::{Rdp, Web};

        //TODO basic auth

        // Try to match a URL format. Examples could be:
        // * http://example.com
        // * https://192.0.2.3
        // * https://[2001:db8::5]:8080
        // * rdp://192.0.2.4:3390
        // * rdp://[2001:db8:6]
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

                _ => return Err("Invalid scheme"),
            }
        }

        match mode {
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
                return Err("Unable to parse target");
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
                } else {
                    if let Ok(u) = Url::parse(&format!("https://[{}]", input)) {
                        targets.push(Target::Url(u));
                    } else {
                        //TODO include error string
                        return Err("Unable to parse HTTPS URL");
                    }
                }

                if let Ok(u) = Url::parse(&format!("http://{}", input)) {
                    targets.push(Target::Url(u));
                } else {
                    if let Ok(u) = Url::parse(&format!("http://[{}]", input)) {
                        targets.push(Target::Url(u));
                    } else {
                        //TODO include error string
                        return Err("Unable to parse HTTP URL");
                    }
                }

                return Ok(targets);
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct InputLists {
    pub rdp_targets: Vec<Target>,
    pub web_targets: Vec<Target>,
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
    use Mode::{Rdp, Web};
    let mut input_lists: InputLists = Default::default();

    // Process the optional command-line target argument
    if let Some(t) = &opts.target {
        match &opts.mode {
            Web => unimplemented!(),
            Rdp => input_lists
                .rdp_targets
                .append(&mut Target::parse(&t, Rdp).unwrap()),
        }
    }

    input_lists
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn parse_target_as_url() {
        use Mode::{Rdp, Web};
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
}
