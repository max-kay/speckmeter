use std::path::{Path, PathBuf};

use egui::{
    plot::{Plot, PlotPoints},
    Context, Ui,
};
use itertools::Itertools;
use log::{error, info, warn};
use native_dialog::FileDialog;

use crate::{
    calibration_module::CalibrationModule,
    camera_module::{CameraStream, Image},
    csv, LARGEST_WAVELENGTH, SMALLEST_WAVELENGTH,
};

pub struct SpectrographModule {
    take_average: usize,
    reference: Option<AbsSpectrograph>,
    current: Option<AbsSpectrograph>,
    spec_buf: Vec<AbsSpectrograph>,
    relative: bool,
    start: f32,
    stop: f32,
    step: f32,
    path: Option<PathBuf>,
    save_next: bool,
    filename: String,
    comment: String,
}

impl SpectrographModule {
    pub fn display(
        &mut self,
        ctx: &Context,
        width: u32,
        height: u32,
        calib: &mut CalibrationModule,
    ) {
        egui::SidePanel::right("spectrograph_opts").show(ctx, |ui| self.side_panel(ui));

        egui::CentralPanel::default().show(ctx, |ui| self.main_view(ui, width, height, calib));
    }
}

impl SpectrographModule {
    pub fn main_view(
        &mut self,
        ui: &mut Ui,
        width: u32,
        height: u32,
        calib: &mut CalibrationModule,
    ) {
        if let Some(img) = CameraStream::get_img(width, height) {
            if let Some(spec) =
                AbsSpectrograph::from_img(&img, calib, self.start, self.stop, self.step)
            {
                self.spec_buf.push(spec);
            } else {
                warn!("could not generate spectrograph")
            }
        }

        if self.spec_buf.len() >= self.take_average {
            self.current = Some(average_spectrograph(&self.spec_buf));
            self.spec_buf = Vec::new();
        }

        match self.current.as_ref() {
            Some(spec) => {
                if self.relative {
                    match self.reference.as_ref() {
                        Some(reference) => {
                            let spec = RelativeSpectrum::new(spec, reference);
                            spec.show(ui);
                            if self.save_next {
                                match self.path.as_ref() {
                                    Some(path) => match spec.write_to_csv(path, &self.comment) {
                                        Ok(_) => info!("saved file succesfully to {:?}", path),
                                        Err(err) => error!("failed to save file Error: {}", err),
                                    },
                                    None => warn!(
                                        "failed to save file, no path was set (shouldn't happen)"
                                    ),
                                }
                                self.save_next = false
                            }
                        }
                        None => {
                            ui.label("no reference available");
                        }
                    }
                } else {
                    if self.save_next {
                        match self.path.as_ref() {
                            Some(path) => match spec.write_to_csv(path, &self.comment) {
                                Ok(_) => info!("saved file succesfully to {:?}", path),
                                Err(err) => error!("failed to save file Error: {}", err),
                            },
                            None => {
                                warn!("failed to save file, no path was set (shouldn't happen)")
                            }
                        }
                        self.save_next = false
                    }
                    spec.show(ui)
                }
            }
            None => warn!("no current image available"),
        }
        ui.ctx().request_repaint()
    }

    pub fn side_panel(&mut self, ui: &mut Ui) {
        if ui.button("take reference").clicked() {
            match self.current.as_ref() {
                Some(spec) => {
                    self.reference = Some(spec.clone());
                    self.relative = true
                }
                None => warn!("failed to load reference"),
            }
        }

        if self.reference.is_some() {
            ui.checkbox(&mut self.relative, "relative");
        }

        ui.add(egui::Slider::new(&mut self.take_average, 0..=100));

        ui.label("Additional comment for csv");
        ui.text_edit_multiline(&mut self.comment);

        ui.label("filename:");
        ui.text_edit_singleline(&mut self.filename);

        if ui.button("save").clicked() {
            let dialog_result = match home::home_dir() {
                Some(home) => FileDialog::new()
                    .set_location(&home)
                    .set_filename(&self.filename)
                    .show_save_single_file(),
                None => FileDialog::new()
                    .set_filename(&self.filename)
                    .show_save_single_file(),
            };
            match dialog_result {
                Ok(opt) => match opt {
                    Some(buf) => {
                        self.path = Some(buf);
                        self.save_next = true;
                    }
                    None => warn!("no path was returned"),
                },
                Err(err) => error!("could not get location, Error: {}", err),
            }
        }
    }
}

impl Default for SpectrographModule {
    fn default() -> Self {
        Self {
            spec_buf: Vec::new(),
            take_average: 1,
            reference: None,
            comment: String::new(),
            current: None,
            relative: false,
            start: SMALLEST_WAVELENGTH as f32,
            stop: LARGEST_WAVELENGTH as f32,
            step: 1.0,
            save_next: false,
            path: home::home_dir(),
            filename: format!("{}.csv", chrono::Local::now().format("%Y_%m_%d_%H_%M")),
        }
    }
}

#[derive(Clone)]
pub struct AbsSpectrograph {
    start: f32,
    stop: f32,
    step: f32,
    values: Vec<f32>,
}

impl AbsSpectrograph {
    pub fn from_img(
        img: &Image,
        calib: &mut CalibrationModule,
        start: f32,
        stop: f32,
        step: f32,
    ) -> Option<Self> {
        let lines = calib.get_lines(start, stop, step)?;

        let mut values = Vec::with_capacity(lines.len());

        for line in lines.iter() {
            values.push(img.read_line_lightness(line));
        }
        Some(Self {
            start,
            stop,
            step,
            values,
        })
    }

    pub fn add(&mut self, other: &Self) {
        assert_eq!(self.start, other.start);
        assert_eq!(self.step, other.step);
        assert_eq!(self.stop, other.stop);
        self.values = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(x1, x2)| x1 + x2)
            .collect();
    }

    pub fn scale(&mut self, factor: f32) {
        self.values.iter_mut().for_each(|x| *x *= factor)
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

    pub fn write_to_csv(&self, path: impl AsRef<Path>, header: &str) -> std::io::Result<()> {
        let wavelengths = (0..self.values.len())
            .map(|x| x as f32 * self.step + self.start)
            .collect_vec();
        csv::write_f32_csv(
            path,
            vec!["wavelengths [nm]".to_string(), "intensity".to_string()],
            vec![wavelengths, self.values.clone()],
            header,
        )
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

    pub fn write_to_csv(&self, path: impl AsRef<Path>, header: &str) -> std::io::Result<()> {
        let wavelengths = (0..self.values.len())
            .map(|x| x as f32 * self.step + self.start)
            .collect_vec();
        csv::write_f32_csv(
            path,
            vec!["wavelengths [nm]".to_string(), "intensity".to_string()],
            vec![wavelengths, self.values.clone()],
            header,
        )
    }
}

fn average_spectrograph(graphs: &Vec<AbsSpectrograph>) -> AbsSpectrograph {
    let factor = 1.0 / graphs.len() as f32;
    let mut graph1 = graphs[0].clone();
    for graph in &graphs[1..] {
        graph1.add(graph)
    }
    graph1.scale(factor);
    graph1
}
