/*
 *   This file is part of NCC Group Scamper https://github.com/nccgroup/scamper
 *   Copyright 2020 David Young <david(dot)young(at)nccgroup(dot)com>
 *   Released as open source by NCC Group Plc - https://www.nccgroup.com
 *
 *   Scamper is free software: you can redistribute it and/or modify
 *   it under the terms of the GNU General Public License as published by
 *   the Free Software Foundation, either version 3 of the License, or
 *   (at your option) any later version.
 *
 *   Scamper is distributed in the hope that it will be useful,
 *   but WITHOUT ANY WARRANTY; without even the implied warranty of
 *   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *   GNU General Public License for more details.
 *
 *   You should have received a copy of the GNU General Public License
 *   along with Scamper.  If not, see <https://www.gnu.org/licenses/>.
*/

use crate::parsing::Target;
use crate::util::target_to_filename;
use image::{DynamicImage, ImageBuffer, Rgba};
use rdp::core::client::Connector;
use rdp::core::event::RdpEvent;
use rdp::core::event::{PointerButton, PointerEvent};
use std::collections::HashMap;
use std::net::TcpStream;
use std::path::Path;

#[allow(unused)]
use log::{debug, error, info, trace, warn};

//TODO maybe make this configurable
const IMAGE_WIDTH: u16 = 1280;
const IMAGE_HEIGHT: u16 = 1024;

struct BitmapChunk {
    width: u32,
    height: u32,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    bpp: u32,
    data: Vec<u8>,
}

enum ColourMode {
    Rgb,
    Rgba,
    Bgr,
    Bgra,
    Luma,
    LumaA,
}

#[derive(Default)]
struct Image {
    buffer: Option<DynamicImage>,
    colour: Option<ColourMode>,
    component_width: Option<usize>,
    width: Option<u32>,
    height: Option<u32>,
    filled_progress: HashMap<(u32, u32), u32>,
}

impl Image {
    fn add_chunk(&mut self, chunk: &BitmapChunk) -> Result<(), ()> {
        //TODO return sensible errors when things are inconsistent

        if self.buffer.is_none() {
            // Image type has not been determined yet
            self.initialise_buffer(chunk)?;
        }

        //TODO assert that the buffer is the right length etc.

        // Initialise an accumulator for calculating the average pixel value
        // 64x64 chunk with 16-bit RGB values (ignoring A) fits inside a u32:
        // 64*64*3*2*255 = 6266880 << 2^32 = 4294967296
        let mut pixval_acc: u32 = 0;

        let mut x: u32 = chunk.left;
        let mut y: u32 = chunk.top;
        for (idx, pixel) in
            chunk.data.chunks(self.component_width.unwrap()).enumerate()
        {
            trace!("idx: {}, pixel: {:?}, at ({}, {})", idx, pixel, x, y);

            match &mut self.buffer {
                Some(DynamicImage::ImageRgba8(img)) => {
                    //let x: usize = img;
                    img.put_pixel(
                        x,
                        y,
                        Rgba([pixel[0], pixel[1], pixel[2], 255 - pixel[3]]),
                    );
                    pixval_acc +=
                        pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32;
                }
                _ => unimplemented!(),
            }

            // Increment x and y around the chunk
            x += 1;
            if x > chunk.right {
                trace!("CR");
                x = chunk.left;
                y += 1;
            }
        }

        // Put average pixel value into hashmap
        let avg = pixval_acc / (self.width.unwrap() * self.height.unwrap());
        self.filled_progress.insert((chunk.top, chunk.left), avg);

        Ok(())
    }

    fn initialise_buffer(&mut self, chunk: &BitmapChunk) -> Result<(), ()> {
        //TODO get these values properly
        // IMAGE_WIDTH and IMAGE_HEIGHT are u16
        self.width = Some(IMAGE_WIDTH as u32);
        self.height = Some(IMAGE_HEIGHT as u32);
        self.colour = Some(ColourMode::Rgba);
        self.component_width = Some(4);
        self.buffer = Some(DynamicImage::ImageRgba8(ImageBuffer::<
            Rgba<u8>,
            Vec<u8>,
        >::new(
            IMAGE_WIDTH as u32,
            IMAGE_HEIGHT as u32,
        )));

        Ok(())
    }

    fn is_complete(&self) -> bool {
        //TODO This method kinda relies on the server sending at least one blank
        // frame before the desktop to ensure that the hashmap is never in a
        // state where it has, say, four filled frames and no blanks. Can this
        // be better? Set minimum = size/size of first frame divided by some
        // conservative approximation?

        // If the hashmap is zero length return false
        if self.filled_progress.iter().count() == 0 {
            trace!("Image empty");
            return false;
        }
        // If ∃ k s.t. hash[k] = 0 then return false
        if self.filled_progress.values().any(|x| *x == 0) {
            trace!("∃ null chunk");
            return false;
        }
        // else return true
        true
    }
}

pub fn capture(target: &Target, output_dir: &Path) -> Result<(), ()> {
    //let ip = opts.target.clone().unwrap();
    let addr = match target {
        Target::Address(sock_addr) => sock_addr,
        Target::Url(_) => return Err(()),
    };

    //let addr = ip.parse::<SocketAddr>().unwrap();
    let tcp = TcpStream::connect(&addr).unwrap();

    let mut connector = Connector::new()
        .screen(IMAGE_WIDTH, IMAGE_HEIGHT)
        .use_nla(false)
        .check_certificate(false)
        .blank_creds(true)
        .credentials("".to_string(), "".to_string(), "".to_string());
    let mut client = connector.connect(tcp).unwrap();

    let mut rdp_image: Image = Default::default();

    let mut exit_count = 0_usize;
    //while exit_count < 230 {
    // A 320-chunk image might need several frames' worth of loops.
    // TODO implement a timeout, probably via tokio later?
    // TODO work out why the captured image sometimes is missing the bottom
    // right corner (all black) and sometimes the top section is overwritten
    // with an orangey-brown colour
    while !rdp_image.is_complete() && exit_count < 800 {
        match client.read(|rdp_event| match rdp_event {
            RdpEvent::Bitmap(bitmap) => {
                // numbers all come in as u16
                let mut chunk = BitmapChunk {
                    width: bitmap.width as u32,
                    height: bitmap.height as u32,
                    left: bitmap.dest_left as u32,
                    top: bitmap.dest_top as u32,
                    right: bitmap.dest_right as u32,
                    bottom: bitmap.dest_bottom as u32,
                    bpp: bitmap.bpp as u32,
                    data: Vec::new(),
                };

                let data = if bitmap.is_compress {
                    bitmap.decompress().unwrap()
                } else {
                    bitmap.data
                };
                chunk.data = data;

                debug!(
                    "Received {}x{} bmp pos {}, {}, {}, {}, bpp: {}, len {}, compress {}",
                    chunk.width,
                    chunk.height,
                    chunk.left,
                    chunk.top,
                    chunk.right,
                    chunk.bottom,
                    chunk.bpp,
                    chunk.data.len(),
                    true, //bitmap.is_compress,
                );

                if !rdp_image.is_complete() {
                    rdp_image.add_chunk(&chunk).unwrap();
                } else {
                    trace!("Image complete, ignoring chunk");
                }
                exit_count += 1;
                trace!("exit count is {}", exit_count);
            }
            RdpEvent::Pointer(_) => info!("Pointer event!"),
            RdpEvent::Key(_) => info!("Key event!"),
        }) {
            Ok(_) => (),
            Err(e) => {
                error!("{:?}", e);
                exit_count = 999;
            },
        }

        // send a mouse event
        client
            .write(RdpEvent::Pointer(PointerEvent {
                x: exit_count as u16,
                y: 100_u16,
                button: PointerButton::None,
                down: false,
            }))
            .unwrap();
    }
    if exit_count > 300 {
        info!("Exit count is {}", exit_count);
    }

    match rdp_image.buffer {
        Some(di) => {
            info!(
                "Received image in {} chunks",
                rdp_image.filled_progress.iter().count()
            );
            let filename = target_to_filename(&target).unwrap();
            let filename = format!("{}.png", filename);
            let filepath = output_dir.join(filename);
            info!("Saving image as {}", filepath.display());
            di.save(filepath).unwrap();
        }
        _ => unimplemented!(),
    }

    Ok(())
}
