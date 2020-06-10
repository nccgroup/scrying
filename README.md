# NCC Group Scamper
A new tool for collecting RDP, web and VNC screenshots all in one place

This tool is still a work-in-progress and should be mostly usable but is not yet complete.
Please file any bugs or feature requests as [GitHub issues](https://github.com/nccgroup/scamper/issues)
## Motivation
Since Eyewitness recently [dropped support for RDP](https://github.com/FortyNorthSecurity/EyeWitness/issues/422#issuecomment-539690698) there isn't a working CLI tool for capturing RDP screenshots.
Nessus still works, but it's a pain to get the images out and they're not included in the export file.

I thought this was a good opportunity to write a fresh tool that's more powerful than those that came before. Check out the feature list!

## Prerequisites
For web screenshotting, scamper currently depends on wkhtmltopdf.
This can be installed from [their website](https://wkhtmltopdf.org/downloads.html) or via `pacman -S wkhtmltopdf`

## Usage
Grab a single web page or RDP server:
```
$ cargo run --release -- -t http://example.com
$ cargo run --release -- -t rdp://192.0.2.1
$ cargo run --release -- -t 2001:db8::5 --mode web
$ cargo run --release -- -t 2001:db8::5 --mode rdp
$ cargo run --release -- -t 192.0.2.2
```

Automatically grab screenshots from an nmap output:
```
$ nmap -iL targets.txt -p 80,443,8080,8443,3389 -oX targets.xml
$ cargo run --release -- --nmap targets.xml
```

Choose a different output directory for images:
```
$ cargo run --release -- -t 2001:db8::3 --output-dir /tmp/scamper_outputs
```

## Features:
Features with ticks next to them have been implemented, others are TODO
* ✔️ Automatically decide whether an input should be treated as a web address or RDP server
* ✔️ Automatically create output directory if it does not already exist
* ✔️ Save images with consistent and unique filenames derived from the host/IP
* ✔️ Full support for IPv6 and IPv4 literals as well as hostnames
* ✔️ Read targets from a file and decide whether they're RDP or HTTP or use hints
* ✔️ Parse targets smartly from nmap output
* ✔️ HTTP - currently implemented by shelling out to wkhtmltoimage, see [#3](https://github.com/nccgroup/scamper/issues/3)
* Full cross-platform support - pending working out web screenshotting properly
* RDP - mostly working, needs better heuristic for determining when it has received a full login/desktop screen image
* VNC
* Video streams
* option for timestamps in filenames
* Read targets from a msf services -o csv output
* Parse targets smartly from nessus output - [WIP](https://github.com/sciguy16/nessus_xml_parser-rs)
* OCR on RDP usernames, either live or on a directory of images
* Readme has pretty pictures of the output
* NLA/auth to test credentials
* Parse Dirble JSON output to grab screenshots of an entire website - waiting for [nccgroup/dirble#51](https://github.com/nccgroup/dirble/issues/51)

## Help text
USAGE:
    scamper [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -s, --silent
    -v, --verbose
    -V, --version    Prints version information

OPTIONS:
    -f, --file <file>
    -l, --log-file <log-file>
    -m, --mode <mode>                 [default: auto]
        --nmap <nmap>
    -o, --output-dir <output-dir>     [default: output]
    -t, --target <target>
        --timeout <timeout>           [default: 10]

