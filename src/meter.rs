use egui::{
    plot::{Plot, PlotPoints},
    Ui,
};
use line_drawing::XiaolinWu;
use log::{error, warn};
use v4l::io::traits::CaptureStream;

use crate::{
    app::{make_img_buf, Image, CAMERA_STREAM},
    calib::SpectralLines, SMALLEST_WAVE_LENGTH, LARGEST_WAVE_LENGTH,
};

pub const fn rgb_lightness(r: u8, g: u8, b: u8) -> f32 {
    (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
}

#[derive(Clone)]
pub struct AbsSpectrograph {
    start: f32,
    stop: f32,
    step: f32,
    values: Vec<f32>,
}

impl AbsSpectrograph {
    pub fn from_img(img: &Image, lines: &SpectralLines, start: f32, stop: f32, step: f32) -> Self {
        let width = img.width as f32;
        let height = img.height as f32;

        let mut values = Vec::with_capacity(((stop - start) / step) as usize);

        let mut current_wl = start;
        while current_wl <= stop {
            let line = lines.line_with_wavelength(current_wl);
            let start = line.start;
            let end = line.end;

            let mut total = 0.0;
            let mut total_weights = 0.0;

            for ((x, y), s) in XiaolinWu::<_, isize>::new(
                (start.0 * width, start.1 * height),
                (end.0 * width, end.1 * height),
            ) {
                let (r, g, b) = img.get(x as usize, y as usize);
                total += rgb_lightness(r, g, b) * s;
                total_weights += s;
            }
            values.push(total / total_weights);
            current_wl += step;
        }
        Self {
            start,
            stop,
            step,
            values,
        }
    }

    pub fn compare(&self, other: &Self) -> bool {
        self.start == other.start && self.stop == other.stop && self.step == other.step
    }

    pub fn show(&self, ui: &mut Ui) {
        let points: PlotPoints = self
            .values
            .iter()
            .enumerate()
            .map(|(i, val)| [self.start as f64 + i as f64 * self.step as f64, *val as f64])
            .collect();

        Plot::new("absolute spectrograph")
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .include_y(0.0)
            .include_y(1.0)
            .show(ui, |plot_ui| plot_ui.line(egui::plot::Line::new(points)));
    }
}

pub struct RelativeSpectrum {
    start: f32,
    step: f32,
    values: Vec<f32>,
}

impl RelativeSpectrum {
    pub fn new(values: &AbsSpectrograph, reference: &AbsSpectrograph) -> Self {
        assert!(values.compare(reference));
        Self {
            start: values.start,
            step: values.step,
            values: values
                .values
                .iter()
                .zip(reference.values.iter())
                .map(|(val, refer)| val / refer)
                .collect(),
        }
    }

    pub fn show(&self, ui: &mut Ui) {
        let points: PlotPoints = self
            .values
            .iter()
            .enumerate()
            .map(|(i, val)| [self.start as f64 + i as f64 * self.step as f64, *val as f64])
            .collect();

        Plot::new("absolute spectrograph")
            .allow_boxed_zoom(false)
            .allow_drag(false)
            .allow_scroll(false)
            .allow_zoom(false)
            .include_y(0.0)
            .include_y(1.0)
            .show(ui, |plot_ui| plot_ui.line(egui::plot::Line::new(points)));
    }
}

pub struct Meter {
    reference: Option<AbsSpectrograph>,
    current: Option<AbsSpectrograph>,
    relative: bool,
    start: f32,
    stop: f32,
    step: f32,
}

impl Meter {
    pub fn main(
        &mut self,
        ui: &mut Ui,
        current_width: u32,
        current_height: u32,
        lines: &SpectralLines,
    ) {
        match CAMERA_STREAM.lock().as_mut() {
            Some(stream) => match stream.next() {
                Ok((buf, _)) => {
                    let img: Image = make_img_buf(buf, current_width, current_height)
                        .expect("image should be ok")
                        .into();

                    self.current = Some(AbsSpectrograph::from_img(
                        &img, lines, self.start, self.stop, self.step,
                    ));
                }
                Err(err) => error!("could not load image: {}", err),
            },
            None => error!("not camera stream exists"),
        }
        match self.current.as_ref() {
            Some(spec) => {
                if self.relative {
                    match self.current.as_ref() {
                        Some(reference) => RelativeSpectrum::new(spec, reference).show(ui),
                        None => {
                            ui.label("no reference available");
                        }
                    }
                } else {
                    spec.show(ui)
                }
            }
            None => warn!("no current image available"),
        }
    }

    pub fn side_panel(&mut self, ui: &mut Ui) {
        if self.reference.is_some() {
            ui.label("you have a reference");
        } else {
            ui.label("you have no reference");
        }
        if ui.button("get reference").clicked() {
            match self.current.as_ref() {
                Some(spec) => self.reference = Some(spec.clone()),
                None => warn!("failed to load reference"),
            }
        }
    }
}

impl Default for Meter {
    fn default() -> Self {
        Self {
            reference: None,
            current: None,
            relative: false,
            start: SMALLEST_WAVE_LENGTH as f32,
            stop: LARGEST_WAVE_LENGTH as f32,
            step: 1.0,
        }
    }
}