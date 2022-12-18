use eframe::emath::RectTransform;
use egui::{self, emath, Align2, Color32, Context, Frame, Pos2, Rect, Response, Slider, Ui};
use itertools::Itertools;
use log::{error, warn};
use std::{f32::consts::PI, mem::swap};

use crate::{
    app::{draw_texture, State},
    camera_module::{CameraStream, Image},
    fitting::{self, Cost, Gradient},
    LARGEST_WAVELENGTH, SMALLEST_WAVELENGTH,
};

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct CalibrationModule {
    lines: Vec<(u16, Line)>,
    #[serde(skip)]
    start: Option<(f32, f32)>,
    #[serde(skip)]
    current_line: Option<Line>,
    #[serde(skip)]
    current_text: String,
    grating_const: f32,
    angle: f32,
    distance_to_sensor: f32,
    sensor_width: f32,
    #[serde(skip)]
    show_generated: Option<u16>,
    #[serde(skip)]
    spectral: Option<SpectralLines>,
}

impl CalibrationModule {
    pub fn display(
        &mut self,
        ctx: &Context,
        main_state: &mut State,
        calibration_image: &mut Option<Image>,
        width: u32,
        height: u32,
    ) {
        egui::SidePanel::right("spectrograph_opts").show(ctx, |ui| self.side_panel(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                if ui.button("take calibration image").clicked() {
                    if let Some(img) = CameraStream::get_img(width, height) {
                        *calibration_image = Some(img);
                    } else {
                        *calibration_image = None;
                        error!("could not take calibration image")
                    }
                }
            });
            match calibration_image.as_mut() {
                None => {
                    ui.vertical_centered(|ui| {
                        ui.horizontal_centered(|ui| {
                            ui.strong("there is no calibration image");
                            if ui.button("go to camera").clicked() {
                                *main_state = State::CameraView;
                            }
                            if ui.button("take calibration image").clicked() {
                                if let Some(img) = CameraStream::get_img(width, height) {
                                    *calibration_image = Some(img);
                                } else {
                                    error!("could not take calibration image")
                                }
                            }
                        })
                    });
                }
                Some(img) => {
                    let aspect_ratio = img.aspect_ratio();
                    let texture = img.get_texture(ui);
                    ui.vertical_centered(|ui| {
                        let style = ui.style();
                        Frame::canvas(style).show(ui, |ui| {
                            let (to_screen, response) = draw_texture(texture, ui);
                            self.main_view(ui, to_screen, aspect_ratio, response);
                        });
                    });
                }
            }
        });
    }
}

impl CalibrationModule {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            start: None,
            current_line: None,
            current_text: String::new(),
            grating_const: 500.0,
            show_generated: None,
            spectral: None,
            angle: 17.5,
            distance_to_sensor: 1.0,
            sensor_width: 0.5,
        }
    }

    fn start_line(&mut self, pos: Pos2) {
        self.start = Some((pos.x, pos.y))
    }

    fn end_line(&mut self, pos: Pos2) {
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

    fn add_new_wavelength(&mut self, wavelength: u16) {
        match self.current_line {
            Some(line) => self.lines.push((wavelength, line)),
            None => warn!("tried to add wave length with no active line"),
        }
        self.current_line = None;
    }

    // TODO left to right or right to left
    fn validate(&mut self) -> bool {
        self.lines
            .iter_mut()
            .for_each(|(_, line)| line.make_top_to_bottom());
        self.lines.sort_by_key(|(wavelength, _)| *wavelength);
        self.lines.is_sorted_by_key(|(_, line)| -line.start.0)
            && self.lines.is_sorted_by_key(|(_, line)| -line.end.0)
    }

    fn generate_regression(&mut self) -> Option<()> {
        dbg!(self.lines.clone());
        if self.validate() && self.lines.len() > 1 {
            self.spectral = SpectralLines::new(
                dbg!(self.lines.clone()),
                self.grating_const,
                self.angle,
                self.distance_to_sensor,
                self.sensor_width,
            );
            Some(())
        } else {
            error!("calibration is invalid");
            None
        }
    }

    pub fn get_lines(&mut self, start: f32, stop: f32, step: f32) -> Option<Vec<Line>> {
        if self.spectral.is_none() {
            self.generate_regression()?
        }
        let mut current_wl = start;
        let mut lines = Vec::with_capacity(((stop - start) / step) as usize);
        while current_wl < stop {
            lines.push(
                self.spectral
                    .as_ref()
                    .unwrap()
                    .line_with_wavelength(current_wl),
            );
            current_wl += step;
        }
        Some(lines)
    }

    pub fn get_line(&mut self, wavelength: f32) -> Option<Line> {
        if self.spectral.is_none() {
            self.generate_regression()?
        }
        Some(
            self.spectral
                .as_ref()
                .unwrap()
                .line_with_wavelength(wavelength),
        )
    }
}

const ACTIVE_LINE_STROKE: (f32, Color32) = (5.0, Color32::WHITE);
const DRAWN_LINE_STROKE: (f32, Color32) = (5.0, Color32::RED);
const GEN_LINE_STROKE: (f32, Color32) = (2.0, Color32::BLACK);
const TEXT_COLOR: Color32 = Color32::BLACK;

impl CalibrationModule {
    pub fn main_view(
        &mut self,
        ui: &mut Ui,
        to_screen: emath::RectTransform,
        aspect_ratio: f32,
        response: Response,
    ) {
        let top_left_screen = to_screen * Pos2 { x: 0.0, y: 0.0 };
        let bottom_right_screen = to_screen
            * Pos2 {
                x: aspect_ratio,
                y: 1.0,
            };
        // this allows me to work in normalised coordiantes, [0, 1]x[0, 1]
        let to_screen = emath::RectTransform::from_to(
            Rect::from_x_y_ranges(0.0..=1.0, 0.0..=1.0),
            Rect::from_min_max(top_left_screen, bottom_right_screen),
        );
        let to_picture = to_screen.inverse();
        // Show generated lines if they exist and line_count is set and then skip the rest of this fn
        if let Some(line_count) = self.show_generated.as_ref() {
            if let Some(spectral) = self.spectral.as_ref() {
                let step =
                    (LARGEST_WAVELENGTH - SMALLEST_WAVELENGTH) as f32 / (*line_count - 1) as f32;
                for i in 0..*line_count {
                    let wavelength = SMALLEST_WAVELENGTH as f32 + (i as f32 * step);
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
            }
        }
        // paint lines drawn by the user and its corresponding wavelength
        for (wavelength, line) in self.lines.iter() {
            let points = line.to_points(to_screen);
            ui.painter().line_segment(points, DRAWN_LINE_STROKE);
            ui.painter().text(
                points[0],
                Align2::RIGHT_CENTER,
                wavelength.to_string(),
                Default::default(),
                TEXT_COLOR,
            );
        }
        // line saveing
        match self.current_line {
            None => {
                // if there is no line being worked on:
                if !self.current_text.is_empty() {
                    self.current_text = String::new()
                }
                if response.drag_started() {
                    // if a line is started to be drawn save the starting point
                    self.start_line(
                        to_picture
                            * response
                                .interact_pointer_pos()
                                .expect("a drag has started so interaction should exist"),
                    )
                } else if response.dragged() {
                    // paint the line currently being draged
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
                    // save the end point of the line
                    self.end_line(
                        to_picture
                            * response
                                .interact_pointer_pos()
                                .expect("drag ended so there should be an interaction"),
                    )
                }
            }
            Some(line) => {
                // if the line has finnished drawing open a window to enter the corresponding wavelength
                ui.painter()
                    .line_segment(line.to_points(to_screen), DRAWN_LINE_STROKE);
                egui::Window::new("Add Wave length to last line").show(ui.ctx(), |ui| {
                    ui.text_edit_singleline(&mut self.current_text);
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() {
                                match self.current_text.parse::<u16>() {
                                    Ok(val) => self.add_new_wavelength(val),
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

    pub fn side_panel(&mut self, ui: &mut Ui) {
        ui.label(format!("There are {} lines.", self.lines.len()));
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

        ui.strong("Spectrometer settings");
        ui.label("Angle in degrees");
        ui.add(Slider::new(&mut self.angle, -90.0..=90.0));
        ui.label("Distance to sensor in mm");
        ui.add(Slider::new(&mut self.distance_to_sensor, 0.0..=100.0));
        ui.label("Sensor width in mm");
        ui.add(Slider::new(&mut self.sensor_width, 0.0..=10.0));
        ui.label("Grating constant in lines per mm");
        ui.add(Slider::new(&mut self.grating_const, 0.0..=1000.0));
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Copy)]
pub struct Line {
    pub start: (f32, f32),
    pub end: (f32, f32),
}

impl Line {
    pub fn to_points(self, to_screen: RectTransform) -> [Pos2; 2] {
        [to_screen * self.start.into(), to_screen * self.end.into()]
    }

    pub fn make_top_to_bottom(&mut self) {
        if self.start.1 > self.end.1 {
            swap(&mut self.start, &mut self.end)
        }
    }

    pub fn cut_with_horizontal(&self, y: f32) -> f32 {
        self.start.0
            + (y - self.start.1) / (self.end.1 - self.start.1) * (self.end.0 - self.start.0)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SpectralLines {
    grating_const: f32,
    top_param: Vec<f32>,
    bottom_param: Vec<f32>,
}

impl SpectralLines {
    pub fn new(
        measure: Vec<(u16, Line)>,
        grating_const: f32,
        angle: f32,
        dist: f32,
        sensor_width: f32,
    ) -> Option<Self> {
        let a = (angle * PI / 360.0).tan();
        let b = dist / sensor_width;
        let c = 0.5;

        let init_params = vec![a, b, c];

        let rs = measure
            .iter()
            .map(|(wl, _)| *wl as f32 * grating_const / 1_000_000.0)
            .collect_vec();

        let lines = measure.iter().map(|(_, line)| line).collect_vec();

        let (top_param, bottom_param) = gen_param(lines, &rs, init_params);
        Some(Self {
            top_param,
            bottom_param,
            grating_const,
        })
    }

    pub fn line_with_wavelength(&self, lambda: f32) -> Line {
        let top_normed_x = normed_x(lambda * self.grating_const / 1_000_000.0, &self.top_param);
        let bottom_normed_x = normed_x(
            lambda * self.grating_const / 1_000_000.0,
            &self.bottom_param,
        );
        Line {
            start: (top_normed_x, 0.0),
            end: (bottom_normed_x, 1.0),
        }
    }
}

pub fn normed_x(lambda_times_grating_const: f32, parameters: &[f32]) -> f32 {
    let a = parameters[0];
    let b = parameters[1];
    let c = parameters[2];
    let root = (1.0 - lambda_times_grating_const * lambda_times_grating_const).sqrt();
    b * ((a * root - lambda_times_grating_const) / (root + a * lambda_times_grating_const)) + c
}

fn gen_param(lines: Vec<&Line>, rs: &[f32], init_param: Vec<f32>) -> (Vec<f32>, Vec<f32>) {
    let top_xs = lines.iter().map(|line| line.cut_with_horizontal(0.0));
    let top_problem = FittingProblem {
        data: top_xs.into_iter().zip(rs.iter().cloned()).collect(),
    };
    let top_param = fitting::search_minimum(top_problem, init_param.clone(), 1_000_000, 0.1);

    let bottom_xs = lines.iter().map(|line| line.cut_with_horizontal(1.0));
    let bottom_problem = FittingProblem {
        data: bottom_xs.into_iter().zip(rs.iter().cloned()).collect(),
    };
    let bottom_param = fitting::search_minimum(bottom_problem, init_param, 1_000_000, 0.1);

    (top_param, bottom_param)
}

struct FittingProblem {
    data: Vec<(f32, f32)>, // (xs, ratios) ratio = (lambda / d), where d = distance between lines on grating
}

impl Cost for FittingProblem {
    fn cost(&self, parameters: Vec<f32>) -> f32 {
        self.data.iter().fold(0.0, |acc, (x, r)| {
            acc + (normed_x(*r, &parameters) - x).powi(2)
        }) / self.data.len() as f32
    }
}

impl Gradient for FittingProblem {
    fn gradient(&self, parameters: Vec<f32>) -> Vec<f32> {
        let a = parameters[0];
        let b = parameters[1];
        let _c = parameters[2];
        let grad = self
            .data
            .iter()
            .fold([0.0, 0.0, 0.0], |acc, (x, r)| {
                let [mut da, mut db, mut dc] = acc;
                let prefactor = 2.0 * (normed_x(*r, &parameters) - x);
                let root = (1.0 - r * r).sqrt();

                da += prefactor * b * (root * (root + a * r) - (a * root - r) * r)
                    / (root + a * r).powi(2);

                db += prefactor * (a * root - r) / (root + a * r);

                dc += prefactor;

                [da, db, dc]
            })
            .into();
        fitting::scale(grad, 1.0 / self.data.len() as f32)
    }
}
