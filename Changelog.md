# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added

### Changed

### Deprecated

### Removed

### Fixed

### Security


## [v0.9.0] - 2022-11-06
### Added
* Customise the size of captured images with the `--size` option (web & RDP). Does not work on VNC because the server generally specifies the screen size [#36](https://github.com/nccgroup/scrying/issues/36)
* Optionally provide RDP credentials
* Option to skip producing a report.html [#45](https://github.com/nccgroup/scrying/issues/45)

### Changed
* Disable RDP certificate verification [#48](https://github.com/nccgroup/scrying/issues/48)

### Removed
* Removed support for platform-native webview to reduce maintenance requirements

### Fixed
* Replace question marks in URLs when generating filenames


## [v0.9.0-alpha.2] - 2021-06-22
### Added
* Add support for VNC passwords [#38](https://github.com/nccgroup/scrying/issues/38)
* Add support for requesting particular paths from web targets [#41](https://github.com/nccgroup/scrying/issues/41)

### Changed
* Log messages from the RDP and VNC modules are now tagged with the target IP [#42](https://github.com/nccgroup/scrying/issues/42)


### Removed
* Removed explicit support for macos because I can't easily test or develop for it and I don't know how best to screenshot a cocoa webview (contributions welcome if you have any ideas)


## [v0.9.0-alpha.1] - 2021-03-07

### Changed
* Windows builds now use the native Edge webview for web rendering
* Linux builds now use Webkit2GTK for web rendering
* Pressing ctrl+c once will ask the current processes to stop and still produce an output file. Pressing it again will cause Scrying to immediately exit with an error code

### Notes
* Missing proxy functionality
* Switch to native renderer on Macos is still TODO


## [v0.8.2] - 2020-11-19
### Fixed
* Debian package now depends on either `chromium`, `chromium-browser` or `google-chrome` because every Debian-derived distribution seems to have its own name for Chromium [#27](https://github.com/nccgroup/scrying/issues/27)


## [v0.8.1] - 2020-11-04
### Changed
* Enable integer overflow checks in release mode to investigate [#26](https://github.com/nccgroup/scrying/issues/26)


## [v0.8.0] - 2020-11-02

### Fixed
* Correctly parse Nmap files without full service information for each port ([nmap_xml_parser#7](https://github.com/Ayrx/nmap_xml_parser/issues/7))


## [v0.7.0] - 2020-06-29
### Added
* Added support for reading Nessus XML files
* RDP errors are collected and included in the report
* While XP is not currently supported, suspected XP-era machines have appropriate error messages [#21](https://github.com/nccgroup/scrying/issues/21)
* Catches CTRL+C so that active targets can be completed and a report produced before exiting

### Changed
* The program exits early if it couldn't parse any targets from the input files [#19](https://github.com/nccgroup/scrying/issues/19)

### Fixed
* Fix bug where connection issues with RDP would result in a panic [#22](https://github.com/nccgroup/scrying/issues/22)
* Fix bug where different input arguments would conflict [#23](https://github.com/nccgroup/scrying/issues/23)


## [v0.6.0] - 2020-06-29
### Added
* Added support for 15- and 24-bit colour depth and 8-bit colour maps on VNC

## [v0.5.0] - 2020-06-22
### Added
* Added support for VNC screenshotting [#6](https://github.com/nccgroup/scrying/issues/6)


## [v0.4.0] - 2020-06-19
### Added
* HTML report output [#7](https://github.com/nccgroup/scrying/issues/7)
* SOCKSv5 proxy support for RDP conenctions [#11](https://github.com/nccgroup/scrying/issues/11)

### Changed
* Targets are deduplicated across all input types before processing [#18](https://github.com/nccgroup/scrying/issues/18)

### Fixed
* Fixed inverted colours on RDP images
* Fixed bug where the output directory argument was ignored [#17](https://github.com/nccgroup/scrying/issues/17)

## [v0.3.0] - 2020-06-18
### Changed
* Changed project name to Scrying

## [v0.2.0] - 2020-06-17
### Added
* Implemented proxy support for web requests

### Fixed
* Fixed bug where RDP images were not received properly. [#2](https://github.com/nccgroup/scrying/issues/2)

## [v0.1.0] - 2020-06-16
### Added
* Implemented RDP screenshotting
* Implemented web screenshotting via headless Chrome
* Parse targets from Nmap XML files
