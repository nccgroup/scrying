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

use crate::error::Error;
use crate::parsing::Target;
use crate::util::target_to_filename;
use crate::ThreadStatus;
use image::{DynamicImage, ImageBuffer, Rgba};
use rdp::core::client::Connector;
use rdp::core::client::RdpClient;
use rdp::core::event::RdpEvent;
use std::collections::HashMap;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, mpsc::Receiver, mpsc::Sender};
use std::thread;
use std::time::Duration;

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

#[allow(dead_code)]
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

        // If the chunk has zero size then we have a problem
        if chunk.left == chunk.right || chunk.top == chunk.bottom {
            warn!("Received zero-size chunk");
            return Err(());
        }

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
                        Rgba([
                            pixel[0], pixel[1], pixel[2],
                            0xff,
                            //TODO: alpha pixel[3],
                            // Sometimes pixel[3] is correct, sometimes
                            // 0xff - pixel[3] is correct.
                        ]),
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
        let chunk_width = chunk.right - chunk.left;
        let chunk_height = chunk.bottom - chunk.top;
        let avg = pixval_acc / (chunk_width * chunk_height);
        debug!("avg: {}", avg);
        self.filled_progress.insert((chunk.top, chunk.left), avg);

        Ok(())
    }

    fn initialise_buffer(&mut self, _chunk: &BitmapChunk) -> Result<(), ()> {
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
}

fn capture_worker(target: &Target, output_dir: &Path) -> Result<(), Error> {
    info!("Connecting to {:?}", target);
    let addr = match target {
        Target::Address(sock_addr) => sock_addr,
        Target::Url(_) => {
            return Err(Error::RdpError(format!(
                "Invalid RDP target: {}",
                target
            )));
        }
    };

    let tcp = TcpStream::connect(&addr)?;

    let mut connector = Connector::new()
        .screen(IMAGE_WIDTH, IMAGE_HEIGHT)
        .use_nla(false)
        .check_certificate(false)
        .blank_creds(true)
        .credentials("".to_string(), "".to_string(), "".to_string());
    let client = connector.connect(tcp)?;

    let mut rdp_image: Image = Default::default();
    {
        // Spawn a thread to listen for bitmap events
        let (bmp_sender, bmp_receiver): (Sender<BitmapChunk>, Receiver<_>) =
            mpsc::channel();
        let _bmp_thread_handle = thread::spawn(move || {
            bmp_thread(client, bmp_sender);
        });

        let timeout = Duration::from_secs(2);
        loop {
            match bmp_receiver.recv_timeout(timeout) {
                Err(_) => {
                    warn!("Timeout reached");
                    break;
                }
                Ok(chunk) => {
                    if rdp_image.add_chunk(&chunk).is_err() {
                        warn!("Attempted to add invalid chunk");
                        //break;
                    }
                }
            }
        }
    }
    match rdp_image.buffer {
        Some(di) => {
            info!(
                "Received image in {} chunks",
                rdp_image.filled_progress.iter().count()
            );
            let filename = target_to_filename(&target);
            let filename = format!("{}.png", filename);
            let filepath = output_dir.join(filename);
            info!("Saving image as {}", filepath.display());
            di.save(filepath)?;
        }
        _ => unimplemented!(),
    }

    Ok(())
}

fn bmp_thread<T: Read + Write>(
    mut client: RdpClient<T>,
    sender: Sender<BitmapChunk>,
) {
    let break_cond = AtomicBool::new(false);
    while !break_cond.load(Ordering::Relaxed) {
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
                    bitmap
                        .decompress()
                        .expect("Error decompressing bitmap chunk")
                } else {
                    bitmap.data
                };
                chunk.data = data;

                debug!(
                    "Received {}x{} bmp pos {}, {}, {}, {}, bpp: {}, len {}",
                    chunk.width,
                    chunk.height,
                    chunk.left,
                    chunk.top,
                    chunk.right,
                    chunk.bottom,
                    chunk.bpp,
                    chunk.data.len(),
                );

                if sender.send(chunk).is_err() {
                    // Recevier disconnected, most likely because the timeout
                    // was reached
                    info!("Bitmap channel disconnected");
                    break_cond.store(true, Ordering::Relaxed);
                }
            }
            RdpEvent::Pointer(_) => info!("Pointer event!"),
            RdpEvent::Key(_) => info!("Key event!"),
        }) {
            Ok(_) => (),
            Err(e) => {
                error!("{:?}", e);
                break;
            }
        }
    }
}

pub fn capture(
    target: &Target,
    output_dir: &Path,
    tx: mpsc::Sender<ThreadStatus>,
) {
    if let Err(e) = capture_worker(target, output_dir) {
        warn!("error: {}", e);
    }

    tx.send(ThreadStatus::Complete).unwrap();
}
