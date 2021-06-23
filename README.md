# NCC Group Scrying
![Build](https://github.com/nccgroup/scrying/workflows/Build/badge.svg)
![Release](https://github.com/nccgroup/scrying/workflows/Release/badge.svg)

A new tool for collecting RDP, web and VNC screenshots all in one place

This tool is still a work-in-progress and should be mostly usable but is not yet complete.
Please file any bugs or feature requests as [GitHub issues](https://github.com/nccgroup/scrying/issues)

## Caveats
* Web screenshotting on Linux relies on webkit2gtk and an X server (e.g. `xvfb`)
* Web screenshotting uses the system webviews, and hiding these windows is surprisingly hard
* I haven't found a good way to take screenshots of a cocoa webview on macos - please open an issue or PR if you're interested in using this on macos and have any idea how to implement it

## Motivation
Since Eyewitness recently [dropped support for RDP](https://github.com/FortyNorthSecurity/EyeWitness/issues/422#issuecomment-539690698) there isn't a working CLI tool for capturing RDP screenshots.
Nessus still works, but it's a pain to get the images out and they're not included in the export file.

I thought this was a good opportunity to write a fresh tool that's more powerful than those that came before. Check out the feature list!

## Installation
For web screenshotting, scrying currently depends on there being an installation of Chromium or Google Chrome. Install with `pacman -S chromium` or the equivalent for your OS.

Download the latest release from [the releases tab](https://github.com/nccgroup/scrying/releases). There's a Debian package available for distros that use them (install with `sudo dpkg -i scrying*.deb`), and zipped binaries for Windows, Mac, and other Linuxes.

## Usage
Grab a single web page, RDP server, or VNC server:
```
$ scrying -t http://example.com
$ scrying -t rdp://192.0.2.1
$ scrying -t 2001:db8::5 --mode web
$ scrying -t 2001:db8::5 --mode rdp
$ scrying -t 192.0.2.2
$ scrying -t vnc://[2001:db8::53]:5901
```

Run on a headless server:
```
# apt install xvfb # or OS equivalent
$ xvfb-run scrying -t http://example.com
```

Automatically grab screenshots from an nmap output:
```
$ nmap -iL targets.txt -p 80,443,8080,8443,3389 -oX targets.xml
$ scrying --nmap targets.xml
```

Choose a different output directory for images:
```
$ scrying -t 2001:db8::3 --output-dir /tmp/scrying_outputs
```

Run from a targets file:
```
$ cat targets.txt
http://example.com
rdp://192.0.2.1
2001:db8::5
$ scrying -f targets.txt
```

Run through a web proxy:
```
$ scrying -t http://example.com --web-proxy http://127.0.0.1:8080
$ scrying -t http://example.com --web-proxy socks5://\[::1\]:1080
```

Image files are saved as PNG in the following directory structure:
```
output
├── report.html
├── rdp
│   └── 192.0.2.1-3389.png
├── vnc
│   └── 192.0.2.1-5900.png
└── web
    └── https_example.com.png
```

Check out the report at `output/report.html`!

## Features:
Features with ticks next to them have been implemented, others are TODO
* ✔️ Automatically decide whether an input should be treated as a web address or RDP server
* ✔️ Automatically create output directory if it does not already exist
* ✔️ Save images with consistent and unique filenames derived from the host/IP
* ✔️ Full support for IPv6 and IPv4 literals as well as hostnames
* ✔️ Read targets from a file and decide whether they're RDP or HTTP or use hints
* ✔️ Parse targets smartly from Nmap and Nessus output
* ✔️ HTTP - uses platform web renderer, optionally provide paths to try on each server
* ✔️ Produces an HTML report to allow easy browsing of the results
* ✔️ VNC - supports sending auth
* ✔️ RDP - mostly working, does not support "plain RDP" mode, see [#15](https://github.com/nccgroup/scrying/issues/15)
* ✔️ Customise size of captured images (web & RDP; VNC does not generally allow this)
* Proxy support - SOCKS works for RDP. Web is currently broken pending [inclusion of the set_proxy command in webkit2gtk](https://github.com/gtk-rs/webkit2gtk-rs/issues/81) [#11](https://github.com/nccgroup/scrying/issues/11)
* Video streams - tracking issue [#5](https://github.com/nccgroup/scrying/issues/5)
* option for timestamps in filenames
* Read targets from a msf services -o csv output
* OCR on RDP usernames, either live or on a directory of images
* NLA/auth to test credentials
* Parse Dirble JSON output to grab screenshots of an entire website - waiting for [nccgroup/dirble#51](https://github.com/nccgroup/dirble/issues/51)
* Full cross-platform support - tested on Linux and Windows; Mac support has been deprioritised pending good ideas for screenshotting the cocoa webview


## Help text
```
USAGE:
    scrying [FLAGS] [OPTIONS] <--file <FILE>...|--nmap <NMAP XML FILE>...|--nessus <NESSUS XML FILE>...|--target <TARGET>...>

FLAGS:
    -s, --silent         Suppress most log messages
        --test-import    Exit after importing targets
    -v, --verbose        Increase log verbosity
    -h, --help           Prints help information
    -V, --version        Prints version information

OPTIONS:
    -f, --file <FILE>...                 Targets file, one per line
    -l, --log-file <LOG FILE>            Save logs to the given file
    -m, --mode <MODE>
            Force targets to be parsed as `web`, `rdp`, `vnc` [default: auto] [possible values: web,
            rdp, vnc, auto]

        --nessus <NESSUS XML FILE>...    Nessus XML file
        --nmap <NMAP XML FILE>...        Nmap XML file
    -o, --output <OUTPUT DIR>            Directory to save the captured images in [default: output]
        --proxy <PROXY>
            Default SOCKS5 proxy to use for connections e.g. socks5://[::1]:1080

        --rdp-proxy <RDP PROXY>
            SOCKS5 proxy to use for RDP connections e.g. socks5://[::1]:1080

        --rdp-timeout <RDP TIMEOUT>
            Seconds to wait after last bitmap before saving an image [default: 2]

        --size <SIZE>
            Set the size of captured images in pixels. Due to protocol limitations, sizes greater
            than 65535x65535 may get truncated in interesting ways. This argument has no effect on
            VNC screenshots. [default: 1280x1024]

    -t, --target <TARGET>...             Target, e.g. http://example.com, rdp://[2001:db8::4]
        --threads <THREADS>              Number of worker threads for each target type [default: 10]
        --vnc-auth <VNC AUTH>            Password to provide to VNC servers that request one
        --web-path <WEB PATH>...
            Append a path to web requests. Provide multiple to request each path sequentially

        --web-proxy <WEB PROXY>
            HTTP/SOCKS Proxy to use for web requests e.g. http://[::1]:8080
```

## Sample HTML report
![Sample report](images/scrying-report.png)
