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
