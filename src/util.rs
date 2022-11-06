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

use crate::parsing::Target;
use std::net::SocketAddr;

//TODO maybe move this to impl fmt::Display rather than a function
pub fn target_to_filename(target: &Target) -> String {
    match target {
        Target::Address(SocketAddr::V4(addr)) => {
            format!("{}", addr).replace(':', "-")
        }
        Target::Address(SocketAddr::V6(addr)) => format!("{}", addr)
            .replace("]:", "-")
            .replace('[', "")
            .replace(':', "_"),
        Target::Url(u) => {
            // The :// scheme separator is converted to a hyphen
            // Any slashes in the URL are converted into hyphens
            // The port-separating colon is converted into an underscore
            // This avoids the edge case where http://example.com:8080/thing
            // and http://example.com/8080/thing convert to the same.
            // TODO maybe convert the colons in a v6 address to dots
            // rather than underscores
            let mut converted: String = String::from(u.as_str())
                .replace("://", "_") // Replace the scheme separator with -
                .replace('/', "-") // replace all slashes with -
                // replace colon (probably port, could be uname)
                .replace([':', '?'], "_")
                // Remove the square brackets as they are not needed for
                // uniqueness
                .replace(['[', ']'], "");
            while converted.ends_with('-') {
                // remove the trailing - if the URL had a trailing /
                converted.pop();
            }

            converted
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::net::ToSocketAddrs;
    use url::Url;
    #[test]
    fn test_target_to_filename() {
        let test_cases: Vec<(Target, &str)> = vec![
            (
                Target::Url(Url::parse("http://example.com///").unwrap()),
                "http_example.com",
            ),
            (
                Target::Url(Url::parse("http://example.com:8443").unwrap()),
                "http_example.com_8443",
            ),
            (
                Target::Url(
                    Url::parse("http://example.com:8443/this/is/a/path/")
                        .unwrap(),
                ),
                "http_example.com_8443-this-is-a-path",
            ),
            (
                Target::Url(Url::parse("http://192.0.2.65_8443/").unwrap()),
                "http_192.0.2.65_8443",
            ),
            (
                Target::Url(Url::parse("http://[2001:db8::56]:8443/").unwrap()),
                "http_2001_db8__56_8443",
            ),
            (
                Target::Address(
                    "[::1]:3389".to_socket_addrs().unwrap().next().unwrap(),
                ),
                "__1-3389",
            ),
            (
                Target::Address(
                    "[2001:db8::1]:3389"
                        .to_socket_addrs()
                        .unwrap()
                        .next()
                        .unwrap(),
                ),
                "2001_db8__1-3389",
            ),
            (
                Target::Address(
                    "192.0.2.45:3389"
                        .to_socket_addrs()
                        .unwrap()
                        .next()
                        .unwrap(),
                ),
                "192.0.2.45-3389",
            ),
        ];

        for case in test_cases {
            eprintln!("Test case: {:?}", case);
            let parsed = target_to_filename(&case.0);
            assert_eq!(parsed, case.1);
        }
    }
}
