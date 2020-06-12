#!/usr/bin/env python3

#   This file is part of NCC Group Scamper https://github.com/nccgroup/scamper
#   Copyright 2020 David Young <david(dot)young(at)nccgroup(dot)com>
#   Released as open source by NCC Group Plc - https://www.nccgroup.com
#
#   Scamper is free software: you can redistribute it and/or modify
#   it under the terms of the GNU General Public License as published by
#   the Free Software Foundation, either version 3 of the License, or
#   (at your option) any later version.
#
#   Scamper is distributed in the hope that it will be useful,
#   but WITHOUT ANY WARRANTY; without even the implied warranty of
#   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
#   GNU General Public License for more details.
#
#   You should have received a copy of the GNU General Public License
#   along with Scamper.  If not, see <https://www.gnu.org/licenses/>.

import argparse
import itertools
import os
import pathlib
import requests
import shutil
import sys

# URLs of the built wkhtmltopdf artefacts
artefacts = {
	"linux": "https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6-1/wkhtmltox_0.12.6-1.bionic_amd64.deb",
	"windows": "https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6-1/wkhtmltox-0.12.6-1.msvc2015-win64.exe",
	"macos": "https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6-1/wkhtmltox-0.12.6-1.macos-cocoa.pkg",
}

def main():
	parser = argparse.ArgumentParser()
	parser.add_argument("os", help = "linux, windows, macos")
	args = parser.parse_args()

	if args.os not in ["linux", "windows", "macos"]:
		print("OS must be one of 'linux', 'windows', 'macos'")
		exit()

	# Make a target directory for the download
	pathlib.Path("target/shared_lib").mkdir(parents=True, exist_ok=True)

	# Download the relevant file if it doesn't already exist
	filename = artefacts[args.os].split('/')[-1]
	filepath = "target/shared_lib/" + filename
	if not pathlib.Path(filepath).is_file():
		response = requests.get(artefacts[args.os], stream=True)
		if response.status_code != requests.codes.ok:
			print("Received unexpected response code " + response.status_code)
			exit()
		print("Saving as " + filepath)

		with open(filepath, "wb") as file:
			spinner = itertools.cycle(['-', '/', '|', '\\'])
			for chunk in response.iter_content(1024):
				sys.stdout.write(next(spinner))
				sys.stdout.flush()
				sys.stdout.write('\b')
				file.write(chunk)
	else:
		print("File already exists, skipping download")

	print("Download complete, extracting library...")
	if args.os == "linux":
		os.system("dpkg-deb -x " + filepath + " target/shared_lib/")
		shutil.copy("target/shared_lib/usr/local/lib/libwkhtmltox.so", ".")
	elif args.os == "windows":
		oldpwd = os.getcwd()
		os.chdir("target/shared_lib")
		os.system("7z e " + filename)
		os.chdir(oldpwd)
		shutil.copy("target/shared_lib/wkhtmltox.dll", ".")
	elif args.os == "macos":
		oldpwd = os.getcwd()
		os.chdir("target/shared_lib")
		os.system("xar -xf " + filename)
		os.system("tar -xzf Payload")
		os.chdir("usr/local/share/wkhtmltox-installer")
		os.system("tar -xzf wkhtmltox.tar.gz")
		os.chdir(oldpwd)
		shutil.copy("target/shared_lib/usr/local/share/wkhtmltox-installer/lib/libwkhtmltox.0.dylib", ".")
	print("Extraction complete!")

	
if __name__ == "__main__":
	main()
