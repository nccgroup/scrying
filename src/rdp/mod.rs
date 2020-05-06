use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use rdp::core::client::Connector;
use rdp::core::event::RdpEvent;
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
}

impl Image {
    fn add_chunk(&mut self, chunk: &BitmapChunk) -> Result<(), ()> {
        //TODO return sensible errors when things are inconsistent

        if self.buffer.is_none() {
            // Image type has not been determined yet
            self.initialise_buffer(chunk)?;
        }

        //TODO assert that the buffer is the right length etc.

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
    while exit_count < 230 {
        client.read(|rdp_event| match rdp_event {
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
        }).unwrap();
    }

    match rdp_image.buffer {
        Some(di) => {
            info!("Saving image");
            di.save("/tmp/image.png").unwrap();
        }
        _ => unimplemented!(),
    }
}
