use egui::{self, ColorImage, Context, TextureHandle};
use image::{ImageBuffer, Rgb, RgbaImage, buffer::ConvertBuffer};
use line_drawing::XiaolinWu;
use nokhwa::{NokhwaError, pixel_format::RgbFormat};

use crate::calibration_module::Line;

pub struct Image {
    width: usize,
    height: usize,
    data: Vec<u8>,
    texture: Option<TextureHandle>
}

impl Image {
    pub fn get_texture(&mut self, ctx: &Context) -> &egui::TextureHandle {
        if self.texture.is_some() {
            return self.texture.as_ref().unwrap();
        }
        let buf: RgbaImage = ImageBuffer::<Rgb<u8>, &[u8]>::from_raw(
            self.width as u32,
            self.height as u32,
            &self.data,
        )
        .expect("building buffer failed")
        .convert();
        let image = ColorImage::from_rgba_unmultiplied([self.width, self.height], &buf);
        self.texture = Some(ctx.load_texture(
            "Image",
            image,
            egui::TextureFilter::Linear,
        ));
        self.texture.as_ref().unwrap()
    }

    pub fn get(&self, x: usize, y: usize) -> Option<(u8, u8, u8)> {
        if self.width < x {
            return None;
        }
        let index = 3 * (y * self.width + x);
        Some((
            *self.data.get(index)?,
            *self.data.get(index + 1)?,
            *self.data.get(index + 2)?,
        ))
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    pub fn read_line_lightness(&self, line: &Line) -> f32 {
        let start = line.start;
        let end = line.end;

        let mut total = 0.0;
        let mut total_weights = 0.0;

        for ((x, y), s) in XiaolinWu::<_, isize>::new(
            (start.0 * self.width as f32, start.1 * self.height as f32),
            (end.0 * self.width as f32, end.1 * self.height as f32),
        ) {
            if let Some((r, g, b)) = self.get(x as usize, y as usize) {
                total += rgb_lightness(r, g, b) * s;
                total_weights += s;
            }
        }
        total / total_weights
    }
}

impl From<ImageBuffer<Rgb<u8>, Vec<u8>>> for Image {
    fn from(value: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Self {
        Self {
            width: value.width() as usize,
            height: value.height() as usize,
            data: value.into_vec(),
            texture: None
        }
    }
}

impl Image {
    pub fn new(value: nokhwa::Buffer) -> Result<Self, NokhwaError> {
        Ok(value.decode_image::<RgbFormat>()?.into())
    }
}

pub const fn rgb_lightness(r: u8, g: u8, b: u8) -> f32 {
    (r as f32 + g as f32 + b as f32) / (255.0 * 3.0)
}
