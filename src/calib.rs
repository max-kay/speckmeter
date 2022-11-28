use eframe::emath::RectTransform;
use egui::{self, emath, Align2, Color32, Pos2, Response, Slider, Ui};
use itertools::Itertools;
use log::{error, warn};
use std::mem::swap;

use crate::{
    lin_reg::{lin_reg, Regression},
    LARGEST_WAVE_LENGTH, SMALLEST_WAVE_LENGTH,
};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Calibration {
    lines: Vec<(u16, Line)>,
    start: Option<(f32, f32)>,
    current_line: Option<Line>,
    current_text: String,
    horizontal_lines: bool,
    distance: f32,
    #[serde(skip)]
    spectral: Option<SpectralLines>,
    #[serde(skip)]
    show_generated: Option<u16>,
}

impl Calibration {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            start: None,
            current_line: None,
            current_text: String::new(),
            horizontal_lines: false,
            distance: 2.0,
            spectral: None,
            show_generated: None,
        }
    }

    pub fn start_line(&mut self, pos: Pos2) {
        self.start = Some((pos.x, pos.y))
    }

    pub fn end_line(&mut self, pos: Pos2) {
        match self.start {
            Some(start) => {
                self.start = None;
                self.current_line = Some(Line {
                    start,
                    end: (pos.x, pos.y),
                })
            }
            None => warn!("tried to end calibration line with out starting it!"),
        }
    }

    pub fn add_new_wave_length(&mut self, wave_length: u16) {
        match self.current_line {
            Some(line) => self.lines.push((wave_length, line)),
            None => warn!("tried to add wave length with no active line"),
        }
        self.current_line = None;
    }

    pub fn validate(&mut self) -> bool {
        if self.horizontal_lines {
            self.lines
                .iter_mut()
                .for_each(|(_, line)| line.make_left_to_right());
            self.lines.sort_by_key(|(wavelength, _)| *wavelength);
            self.lines.is_sorted_by_key(|(_, line)| line.start.1)
                && self.lines.is_sorted_by_key(|(_, line)| line.end.1)
        } else {
            self.lines
                .iter_mut()
                .for_each(|(_, line)| line.make_top_to_bottom());
            self.lines.sort_by_key(|(wavelength, _)| *wavelength);
            self.lines.is_sorted_by_key(|(_, line)| line.start.0)
                && self.lines.is_sorted_by_key(|(_, line)| line.end.0)
        }
    }

    fn generate_regression(&mut self) -> Option<()> {
        if self.validate() && self.lines.len() > 1 {
            let wavelengths: Vec<f32> = self
                .lines
                .iter()
                .map(|(wavelength, _)| *wavelength as f32)
                .collect();
            let coordinates = [
                self.lines
                    .iter()
                    .map(|(_, line)| line.start.0)
                    .collect_vec(),
                self.lines
                    .iter()
                    .map(|(_, line)| line.start.1)
                    .collect_vec(),
                self.lines.iter().map(|(_, line)| line.end.0).collect_vec(),
                self.lines.iter().map(|(_, line)| line.end.1).collect_vec(),
            ];
            self.spectral = Some(SpectralLines::from_lin_reg(lin_reg(
                wavelengths,
                &coordinates,
            )));
            Some(())
        } else {
            error!("calibration is invalid");
            None
        }
    }

    pub fn get_lines(&mut self) -> Option<&SpectralLines> {
        if self.spectral.is_none() {
            match self.generate_regression() {
                Some(_) => self.spectral.as_ref(),
                None => None,
            }
        } else {
            self.spectral.as_ref()
        }
    }
}

const ACTIVE_LINE_STROKE: (f32, Color32) = (5.0, Color32::WHITE);
const DRAWN_LINE_STROKE: (f32, Color32) = (5.0, Color32::RED);
const GEN_LINE_STROKE: (f32, Color32) = (2.0, Color32::RED);
const TEXT_COLOR: Color32 = Color32::WHITE;

impl Calibration {
    pub fn main_view(&mut self, ui: &mut Ui, to_screen: emath::RectTransform, response: Response) {
        let to_picture = to_screen.inverse();
        if let Some(line_count) = self.show_generated.as_ref() {
            if let Some(spectral) = self.spectral.as_ref() {
                let step =
                    (LARGEST_WAVE_LENGTH - SMALLEST_WAVE_LENGTH) as f32 / (*line_count - 1) as f32;
                for i in 0..*line_count {
                    let wavelength = SMALLEST_WAVE_LENGTH as f32 + (i as f32 * step);
                    let screen_points = spectral
                        .line_with_wavelength(wavelength)
                        .to_points(to_screen);
                    ui.painter().line_segment(screen_points, GEN_LINE_STROKE);
                    ui.painter().text(
                        screen_points[1],
                        Align2::RIGHT_CENTER,
                        wavelength.to_string(),
                        Default::default(),
                        TEXT_COLOR,
                    );
                }
                return;
            }
        }
        for (wave_length, line) in self.lines.iter() {
            let points = line.to_points(to_screen);
            ui.painter().line_segment(points, DRAWN_LINE_STROKE);
            ui.painter().text(
                points[0],
                Align2::RIGHT_CENTER,
                wave_length.to_string(),
                Default::default(),
                TEXT_COLOR,
            );
        }
        match self.current_line {
            None => {
                if !self.current_text.is_empty() {
                    self.current_text = String::new()
                }
                if response.drag_started() {
                    self.start_line(
                        to_picture
                            * response
                                .interact_pointer_pos()
                                .expect("a drag has started so interaction should exist"),
                    )
                } else if response.dragged() {
                    let screen_start =
                        to_screen * self.start.expect("there should be an active line").into();
                    ui.painter().line_segment(
                        [
                            screen_start,
                            response
                                .interact_pointer_pos()
                                .expect("pointer is draged so there should be an interaction"),
                        ],
                        ACTIVE_LINE_STROKE,
                    )
                } else if response.drag_released() {
                    self.end_line(
                        to_picture
                            * response
                                .interact_pointer_pos()
                                .expect("drag ended so there should be an interaction"),
                    )
                }
            }
            Some(line) => {
                ui.painter()
                    .line_segment(line.to_points(to_screen), DRAWN_LINE_STROKE);
                egui::Window::new("Add Wave length to last line").show(ui.ctx(), |ui| {
                    ui.text_edit_singleline(&mut self.current_text);
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() {
                                match self.current_text.parse::<u16>() {
                                    Ok(val) => self.add_new_wave_length(val),
                                    Err(_) => {
                                        self.current_text =
                                            "this has to be a valid integer".to_string()
                                    }
                                }
                            }
                            if ui.button("Discard Line").clicked() {
                                self.current_line = None;
                            }
                        })
                    });
                });
            }
        }
    }

    pub fn side_view(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Lines with the same wavelength are");
            if ui.radio(self.horizontal_lines, "horizontal").clicked() {
                self.horizontal_lines = true;
            }
            if ui.radio(!self.horizontal_lines, "vertical").clicked() {
                self.horizontal_lines = false;
            }
        });
        ui.label(format!("There are {} lines.", self.lines.len()));
        ui.horizontal(|ui| {
            ui.label("distance between diffrection lines in mm");
            ui.add(egui::Slider::new(&mut self.distance, 0.0..=5.0));
        });
        if self.spectral.is_some() {
            match self.show_generated.as_mut() {
                Some(line_count) => {
                    if ui.button("delete regression").clicked() {
                        self.spectral = None;
                    }
                    ui.add(Slider::new(line_count, 3..=60));
                }
                None => {
                    if ui.button("show generated lines").clicked() {
                        self.show_generated = Some(10);
                    }
                }
            }
        } else {
            self.show_generated = None;
            if ui.button("generate regression").clicked() {
                self.generate_regression();
                self.show_generated = Some(10);
            }
        }
        if ui.button("discard all lines").clicked() {
            self.lines = Vec::new();
            self.start = None;
            self.current_line = None;
            self.current_text = String::new();
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct Line {
    pub start: (f32, f32),
    pub end: (f32, f32),
}

impl Line {
    pub fn to_points(self, to_screen: RectTransform) -> [Pos2; 2] {
        [to_screen * self.start.into(), to_screen * self.end.into()]
    }

    pub fn make_left_to_right(&mut self) {
        if self.start.0 > self.end.0 {
            swap(&mut self.start, &mut self.end)
        }
    }

    pub fn make_top_to_bottom(&mut self) {
        if self.start.1 > self.end.1 {
            swap(&mut self.start, &mut self.end)
        }
    }
}

pub struct SpectralLines {
    start_x: Box<dyn Fn(f32) -> f32>,
    start_y: Box<dyn Fn(f32) -> f32>,
    end_x: Box<dyn Fn(f32) -> f32>,
    end_y: Box<dyn Fn(f32) -> f32>,
}

impl SpectralLines {
    pub fn from_lin_reg(reg: Regression) -> Self {
        let Regression { slopes, y_offsets } = reg;

        let slopes0 = slopes[0];
        let y_offsets0 = y_offsets[0];
        let slopes1 = slopes[1];
        let y_offsets1 = y_offsets[1];
        let slopes2 = slopes[2];
        let y_offsets2 = y_offsets[2];
        let slopes3 = slopes[3];
        let y_offsets3 = y_offsets[3];
        Self {
            start_x: Box::new(move |lambda| slopes0 * lambda + y_offsets0),
            start_y: Box::new(move |lambda| slopes1 * lambda + y_offsets1),
            end_x: Box::new(move |lambda| slopes2 * lambda + y_offsets2),
            end_y: Box::new(move |lambda| slopes3 * lambda + y_offsets3),
        }
    }

    pub fn line_with_wavelength(&self, wavelength: f32) -> Line {
        Line {
            start: (
                self.start_x.call((wavelength,)),
                self.start_y.call((wavelength,)),
            ),
            end: (
                self.end_x.call((wavelength,)),
                self.end_y.call((wavelength,)),
            ),
        }
    }
}
