# Universal Screenshooter (name to be decided)
A new tool for collecting RDP, web and VNC screenshots all in one place

# Motivation
Since Eyewitness recently [dropped support for RDP](https://github.com/FortyNorthSecurity/EyeWitness/issues/422#issuecomment-539690698) there isn't a working CLI tool for capturing RDP screenshots.
Nessus still works, but it's a pain to get the images out and they're not included in the export file.

I thought this was a good opportunity to write a fresh tool that's more powerful than those that came before. Check out the feature list!


## Features:
Features with ticks next to them have been implemented, others are TODO
* ✔️ Automatically decide whether an input should be treated as a web address or RDP server
* ✔️ Automatically create output directory if it does not already exist
* ✔️ Save images with consistent and unique filenames derived from the host/IP
* ✔️ Full support for IPv6 and IPv4 literals as well as hostnames
* Read file of RDP hosts, web hosts, or automatic hosts
* RDP - mostly working, needs better heuristic for determining when it has received a full login/desktop screen image
* HTTP
* VNC
* Video streams
* option for timestamps in filenames
* Read targets from a msf services -o csv output
* Parse targets smartly from nessus and nmap output
* OCR on RDP usernames, either live or on a directory of images
* Readme has pretty pictures of the output
