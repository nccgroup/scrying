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

use crate::argparse::Mode::Rdp;
use crate::argparse::Opts;
use crate::parsing::Target;
use crate::reporting::ReportMessageContent;
use crate::reporting::{FileError, ReportMessage};
use crate::util::target_to_filename;
use crate::ThreadStatus;
#[allow(unused)]
use crate::{debug, error, info, trace, warn};
use color_eyre::eyre::eyre;
use image::{DynamicImage, ImageBuffer, Rgba};
use rdp::core::client::{Connector, RdpClient};
use rdp::core::event::RdpEvent;
use socks::Socks5Stream;
use std::fmt::{self, Display, Formatter};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, mpsc::Receiver, mpsc::Sender};
use std::thread;
use std::time::Duration;

pub enum Error {
    Rdp(String),
    Other(color_eyre::Report),
}

impl Display for Error {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match self {
            Error::Rdp(e) => write!(fmt, "RDP error: {e}"),
            Error::Other(e) => write!(fmt, "{e}"),
        }
    }
}

impl<E> From<E> for Error
where
    E: Into<color_eyre::Report>,
{
    fn from(e: E) -> Self {
        Error::Other(e.into())
    }
}

/*impl From<rdp::model::error::Error> for Error {
    fn from(e: rdp::model::error::Error) -> Error {
        Error::Rdp(e.to_string())
    }
}*/

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

enum ImageMode {
    //HighColor16(DynamicImage),
    Rgba32(DynamicImage),
}

impl ImageMode {
    fn extract(self) -> DynamicImage {
        use ImageMode::*;
        match self {
            //HighColor16(di) => di,
            Rgba32(di) => di,
        }
    }
}

#[derive(Default)]
struct Image {
    image: Option<ImageMode>,
    //colour: Option<ColourMode>,
    component_width: Option<usize>,
    width: Option<u32>,
    height: Option<u32>,
}

impl Image {
    fn add_chunk(
        &mut self,
        opts: &Opts,
        target: &Target,
        chunk: &BitmapChunk,
    ) -> Result<(), ()> {
        use ImageMode::*;
        //TODO return sensible errors when things are inconsistent

        if self.image.is_none() {
            // Image type has not been determined yet
            self.initialise_buffer(opts, target, chunk)?;
        }

        //TODO assert that the buffer is the right length etc.

        // If the chunk has zero size then we have a problem
        if chunk.left == chunk.right || chunk.top == chunk.bottom {
            debug!(target, "Received zero-size chunk");
            return Err(());
        }

        let mut x: u32 = chunk.left;
        let mut y: u32 = chunk.top;

        // the enumerate is sometimes running more times that fits into
        // the height of the image
        for (idx, pixel) in
            chunk.data.chunks(self.component_width.unwrap()).enumerate()
        {
            trace!(
                target,
                "idx: {}, pixel: {:?}, at ({}, {})",
                idx,
                pixel,
                x,
                y
            );

            if y > chunk.bottom {
                debug!(target, "Pixel out of bounds!");
                break;
            }

            match &mut self.image {
                Some(Rgba32(DynamicImage::ImageRgba8(img))) => {
                    //let x: usize = img;
                    img.put_pixel(
                        x,
                        y,
                        Rgba([
                            pixel[2], pixel[1], pixel[0],
                            0xff,
                            //TODO: alpha pixel[3],
                            // Sometimes pixel[3] is correct, sometimes
                            // 0xff - pixel[3] is correct.
                        ]),
                    );
                }
                /*Some(HighColor16(DynamicImage::ImageRgb8(img))) => {
                    img.put_pixel(x, y, Rgb([pixel[0], pixel[1], 0]))
                }*/
                _ => unimplemented!(),
            }

            // Increment x and y around the chunk
            x += 1;
            if x > chunk.right {
                trace!(target, "CR");
                x = chunk.left;
                y += 1;
            }
        }

        Ok(())
    }

    fn initialise_buffer(
        &mut self,
        opts: &Opts,
        target: &Target,
        chunk: &BitmapChunk,
    ) -> Result<(), ()> {
        use ImageMode::*;
        debug!(target, "BITS PER PIXEL: {}", chunk.bpp);
        //TODO get these values properly
        let width = opts.size.0 as u32;
        let height = opts.size.1 as u32;

        let pixel_size = 4; //chunk.data.len() as u32
                            // / ((chunk.right - chunk.left) * (chunk.bottom - chunk.top));
        debug!(target, "PIXEL SIZE {}", pixel_size);

        // Have to do a let binding here and then transfer to the self.*
        // variables pending https://github.com/rust-lang/rfcs/pull/2909
        let (component_width, image) = match pixel_size {
            /*2 => {
                debug!("Detected HighColor16");
                (
                    // 16-bit RGB using 5 bits per colour; store as 8 bit colour
                    Some(4),
                    Some(HighColor16(DynamicImage::ImageRgb8(
                        ImageBuffer::<Rgb<u8>, Vec<u8>>::new(width, height),
                    ))),
                )
            }*/
            4 => {
                debug!(target, "Detected RGBA-32");
                (
                    Some(4),
                    Some(Rgba32(DynamicImage::ImageRgba8(ImageBuffer::<
                        Rgba<u8>,
                        Vec<u8>,
                    >::new(
                        width, height
                    )))),
                )
            }
            _ => unimplemented!(),
        };
        self.component_width = component_width;
        self.image = image;
        self.width = Some(width);
        self.height = Some(height);

        Ok(())
    }
}

/// Wrapper enum to hold TCP and Socks5 streams. This enum implements
/// Read and Write transitively
enum SocketType {
    Socks5(Socks5Stream),
    Tcp(TcpStream),
}

impl Read for SocketType {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        use SocketType::*;
        match self {
            Socks5(s) => s.read(buf),
            Tcp(s) => s.read(buf),
        }
    }
}

impl Write for SocketType {
    fn write(
        &mut self,
        buf: &[u8],
    ) -> std::result::Result<usize, std::io::Error> {
        use SocketType::*;
        match self {
            Socks5(s) => s.write(buf),
            Tcp(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> Result<(), std::io::Error> {
        use SocketType::*;
        match self {
            Socks5(s) => s.flush(),
            Tcp(s) => s.flush(),
        }
    }
}

fn capture_worker(
    target: &Target,
    opts: &Opts,
    report_tx: &mpsc::Sender<ReportMessage>,
) -> Result<(), Error> {
    info!(target, "Connecting to {:?}", target);
    let addr = match target {
        Target::Address(sock_addr) => sock_addr,
        Target::Url(_) => {
            return Err(Error::Rdp(format!("Invalid RDP target: {}", target)));
        }
    };

    // If the proxy configuration is selected then create a Socks5
    // connection, otherwise create a regular TCP stream. The wrapper
    // enum is used to get around type errors and the limitation that
    // trait objects can only have one main trait (i.e. "dyn Read +
    // Write") is not possible.
    let stream = if let Some(proxy) = &opts.rdp_proxy {
        debug!(target, "Connecting to Socks proxy");
        SocketType::Socks5(Socks5Stream::connect(proxy, *addr)?)
    } else {
        SocketType::Tcp(TcpStream::connect(addr)?)
    };

    debug!(target, "RDP domain: {:?}", opts.rdp_domain);
    debug!(target, "RDP username: {:?}", opts.rdp_user);
    debug!(target, "RDP password set: {}", opts.rdp_pass.is_some());

    let mut connector = Connector::new()
        .screen(opts.size.0 as u16, opts.size.1 as u16)
        .check_certificate(false);

    if let (Some(user), Some(pass)) = (&opts.rdp_user, &opts.rdp_pass) {
        connector = connector.credentials(
            opts.rdp_domain.as_ref().cloned().unwrap_or_default(),
            user.to_string(),
            pass.to_string(),
        );
    } else {
        warn!(target, "Using blank RDP credentials");
        connector = connector.use_nla(false).blank_creds(true).credentials(
            "".to_string(),
            "".to_string(),
            "".to_string(),
        );
    };

    let client = connector.connect(stream).map_err(|e| eyre!("{e:?}"))?;

    let mut rdp_image: Image = Default::default();
    {
        // Spawn a thread to listen for bitmap events
        let (bmp_sender, bmp_receiver): (Sender<BitmapChunk>, Receiver<_>) =
            mpsc::channel();
        let target_clone = target.clone();
        let _bmp_thread_handle = thread::spawn(move || {
            bmp_thread(target_clone, client, bmp_sender);
        });

        let timeout = Duration::from_secs(2);
        loop {
            match bmp_receiver.recv_timeout(timeout) {
                Err(_) => {
                    warn!(target, "Timeout reached");
                    break;
                }
                Ok(chunk) => {
                    if rdp_image.add_chunk(opts, target, &chunk).is_err() {
                        debug!(target, "Attempted to add invalid chunk");
                        //break;
                    }
                }
            }
        }
    }
    match rdp_image.image {
        Some(di) => {
            info!(target, "Successfully received image");
            let filename = format!("{}.png", target_to_filename(target));
            let relative_filepath = Path::new("rdp").join(&filename);
            let filepath = Path::new(&opts.output_dir).join(&relative_filepath);
            info!(target, "Saving image as {}", filepath.display());
            di.extract().save(&filepath)?;
            let report_message = ReportMessage::Output(ReportMessageContent {
                mode: Rdp,
                target: target.to_string(),
                output: FileError::File(
                    relative_filepath.display().to_string(),
                ),
            });
            report_tx.send(report_message)?;
        }
        None => {
            warn!(target,
            "Error receiving image from {}. Perhaps the server disconnected",
            addr
            );
            return Err(Error::Rdp(
                "Error receiving image, perhaps the server disconnected"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

fn bmp_thread<T: Read + Write>(
    target: Target,
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
                    target,
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
                    info!(target, "Bitmap channel disconnected");
                    break_cond.store(true, Ordering::Relaxed);
                }
            }
            RdpEvent::Pointer(_) => info!(target, "Pointer event!"),
            RdpEvent::Key(_) => info!(target, "Key event!"),
        }) {
            Ok(_) => (),
            Err(e) => {
                error!(target, "Error reading RDP stream: {:?}", e);
                return;
            }
        }
    }
}

pub fn capture(
    target: &Target,
    opts: &Opts,
    tx: mpsc::Sender<ThreadStatus>,
    report_tx: &mpsc::Sender<ReportMessage>,
) {
    if let Err(e) = capture_worker(target, opts, report_tx) {
        warn!(target, "error: {}", e);
        let report_message = match &e {
            Error::Rdp(r) if r.contains("failed to fill whole buffer") => {
                ReportMessage::Output(ReportMessageContent {
                    mode: Rdp,
                    target: target.to_string(),
                    output: FileError::Error(
                        concat!(
                            "Unexpected disconnection, target may be XP-era ",
                            "which is currently unsupported"
                        )
                        .to_string(),
                    ),
                })
            }
            _ => ReportMessage::Output(ReportMessageContent {
                mode: Rdp,
                target: target.to_string(),
                output: FileError::Error(e.to_string()),
            }),
        };
        report_tx
            .send(report_message)
            .expect("Reporting thread seems to have disconnected");
    }

    tx.send(ThreadStatus::Complete).unwrap();
}
