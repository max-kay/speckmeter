use egui::{self, ColorImage, TextureHandle, Ui};
use image::{buffer::ConvertBuffer, ImageBuffer, Rgb, RgbaImage};
use std::vec::Vec;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Image {
    pub width: usize,
    pub height: usize,
    pub(crate) data: Vec<u8>,
    #[serde(skip)]
    pub(crate) texture: Option<TextureHandle>,
}

impl Image {
    pub fn get_texture(&mut self, ui: &mut Ui) -> &egui::TextureHandle {
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
        self.texture = Some(ui.ctx().load_texture(
            "calibration img",
            image,
            egui::TextureFilter::Linear,
        ));
        self.texture.as_ref().unwrap()
    }

    pub fn get(&self, x: usize, y: usize) -> Option<(u8, u8, u8)> {
        if self.width < x {
            // TODO check this logic
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
}

impl From<ImageBuffer<Rgb<u8>, &[u8]>> for Image {
    fn from(value: ImageBuffer<Rgb<u8>, &[u8]>) -> Self {
        Self {
            width: value.width() as usize,
            height: value.height() as usize,
            data: value.to_vec(),
            texture: None,
        }
    }
}
