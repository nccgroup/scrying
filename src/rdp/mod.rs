use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use rdp::core::client::Connector;
use rdp::core::event::RdpEvent;
use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};

#[allow(unused)]
use log::{debug, error, info, trace, warn};

use crate::argparse::Opts;

const IMAGE_WIDTH: u16 = 800;
const IMAGE_HEIGHT: u16 = 600;

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

pub fn capture(opts: &Opts) {
    let ip = opts.target.clone().unwrap();

    let addr = ip.parse::<SocketAddr>().unwrap();
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
    while !rdp_image.is_complete() && exit_count < 400 {
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

                rdp_image.add_chunk(&chunk).unwrap();
                exit_count += 1;
                trace!("exit count is {}", exit_count);
            }
            _event => {
                debug!("Received other event");
            }
        }) {
            Ok(_) => (),
            Err(e) => {
                error!("{:?}", e);
                exit_count = 999;
            },
        }
    }

    match rdp_image.buffer {
        Some(di) => {
            info!("Saving image");
            di.save("/tmp/image.png").unwrap();
        }
        _ => unimplemented!(),
    }
}
