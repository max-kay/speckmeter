use std::sync::Mutex;

use egui::ColorImage;
use image::{buffer::ConvertBuffer, ImageBuffer, Rgb, RgbaImage};
use log::error;
use once_cell::sync::Lazy;
use v4l::{buffer, io::traits::CaptureStream, prelude::*};

use super::Image;

static CAMERA_STREAM: Lazy<Mutex<Option<MmapStream>>> = Lazy::new(Default::default);

pub fn make_img_buf(buf: &[u8], width: u32, height: u32) -> Option<ImageBuffer<Rgb<u8>, &[u8]>> {
    let image = ImageBuffer::from_raw(width, height, buf)?;
    Some(image as ImageBuffer<Rgb<u8>, &[u8]>)
}

pub struct CameraStream;

impl CameraStream {
    pub fn get_img(width: u32, height: u32) -> Option<Image> {
        match CAMERA_STREAM.lock().unwrap().as_mut().unwrap().next() {
            Ok((buf, meta)) => match make_img_buf(buf, width, height) {
                Some(img) => Some(img.into()),
                None => {
                    error!(
                        "could not load image frame: {}, {} bytes received",
                        meta.sequence, meta.bytesused
                    );
                    None
                }
            },
            Err(err) => {
                error!("could not get frame: {}", err);
                None
            }
        }
    }

    pub fn get_img_as_texture(
        ctx: &egui::Context,
        width: u32,
        height: u32,
    ) -> Option<egui::TextureHandle> {
        match CAMERA_STREAM.lock().unwrap().as_mut()?.next() {
            Ok((buf, meta)) => match make_img_buf(buf, width, height) {
                Some(image) => {
                    let image: RgbaImage = image.convert();
                    let image = ColorImage::from_rgba_unmultiplied(
                        [width as usize, height as usize],
                        &image,
                    );
                    Some(ctx.load_texture(
                        format!("frame {}", meta.sequence),
                        image,
                        egui::TextureFilter::Linear,
                    ))
                }
                None => {
                    error!(
                        "could not load image frame: {},   {} bytes received",
                        meta.sequence, meta.bytesused
                    );
                    None
                }
            },
            Err(err) => {
                error!("failed to read frame: {}", err);
                None
            }
        }
    }

    pub fn open_stream(camera: &Device) {
        match MmapStream::with_buffers(
            camera,
            buffer::Type::VideoCapture,
            5,
        ) {
            Ok(stream) => *CAMERA_STREAM.lock().unwrap() = Some(stream),
            Err(err) => error!("Could not open stream:   {}", err),
        }
        
    }

    pub fn close() {
        *CAMERA_STREAM.lock().unwrap() = None
    }

    pub fn is_open() -> bool {
        CAMERA_STREAM.lock().unwrap().is_some()
    }
}
