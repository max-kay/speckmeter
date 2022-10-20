use egui::{emath, mutex::Mutex, Color32, ColorImage, Frame, Mesh, Pos2, Rect, TextureHandle, Ui};
use image::{buffer::ConvertBuffer, ImageBuffer, Rgb, RgbaImage};
use log::error;
use once_cell::sync::Lazy;
use v4l::{io::traits::CaptureStream, prelude::*};

use crate::cam::CameraModule;

pub static CAMERA_STREAM: Lazy<Mutex<Option<MmapStream>>> = Lazy::new(Default::default);

pub fn make_img_buf(buf: &[u8], width: u32, height: u32) -> Option<ImageBuffer<Rgb<u8>, &[u8]>> {
    let image = ImageBuffer::from_raw(width, height, buf)?;
    Some(image as ImageBuffer<Rgb<u8>, &[u8]>)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Image {
    width: usize,
    height: usize,
    data: Vec<u8>,
    #[serde(skip)]
    texture: Option<TextureHandle>,
}

impl Image {
    fn get_texture(&mut self, ui: &mut Ui) -> &egui::TextureHandle {
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

#[derive(serde::Serialize, serde::Deserialize)]
struct Calibration {
    lines: Vec<(f32, Line)>,
}

impl Calibration {
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    pub fn add_line(&mut self, wave_length: f32, line: Line) {
        self.lines.push((wave_length, line))
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Line {
    start: (f32, f32),
    end: (f32, f32),
}

impl Line {
    pub fn new(start: (f32, f32), end: (f32, f32)) -> Self {
        Self { start, end }
    }
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SpeckApp {
    #[serde(skip)]
    camera_module: CameraModule,
    main_state: MainState,
    show_logs: bool,
    show_camera_opts: bool,
    #[serde(skip)]
    calibration_img: Option<Image>,
    #[serde(skip)]
    calibration: Option<Calibration>,
}

impl Default for SpeckApp {
    fn default() -> Self {
        Self {
            camera_module: Default::default(),
            main_state: Default::default(),
            show_logs: true,
            show_camera_opts: true,
            calibration_img: None,
            calibration: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, PartialEq, Eq)]
enum MainState {
    #[default]
    Off,
    CameraView,
    Calibration,
}

impl SpeckApp {
    /// Called once before the first frame.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        // if let Some(storage) = cc.storage {
        //     return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        // }

        Default::default()
    }
}

impl eframe::App for SpeckApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu(ui));

        if self.show_camera_opts {
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                self.camera_module.update(ui);
            });
        }

        if self.show_logs {}

        egui::CentralPanel::default().show(ctx, |ui| {
            self.main_view(ui);
        });
    }
}

impl SpeckApp {
    fn menu(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Sidepanels", |ui| {
                    ui.checkbox(&mut self.show_camera_opts, "Camera Module");
                    ui.checkbox(&mut self.show_logs, "Log window")
                });
                ui.horizontal_centered(|ui| {
                    ui.selectable_value(&mut self.main_state, MainState::Off, "Off");
                    ui.selectable_value(&mut self.main_state, MainState::CameraView, "📷 Camera");
                    ui.selectable_value(
                        &mut self.main_state,
                        MainState::Calibration,
                        "⭕ Calibration",
                    )
                })
            })
        });
    }

    fn main_view(&mut self, ui: &mut Ui) {
        match self.main_state {
            MainState::Off => {
                ui.strong("no view");
            }
            MainState::CameraView => {
                let stream_on = CAMERA_STREAM.lock().is_some();
                if stream_on {
                    ui.vertical_centered(|ui| {
                        if ui.button("take calibration image").clicked() {
                            match CAMERA_STREAM.lock().as_mut().unwrap().next() {
                                Ok((buf, meta)) => match make_img_buf(
                                    buf,
                                    self.camera_module.width(),
                                    self.camera_module.height(),
                                ) {
                                    Some(img) => {
                                        self.calibration_img = Some(img.into());
                                        self.main_state = MainState::Calibration
                                    }
                                    None => error!(
                                        "could not load image frame: {}, {} bytes received",
                                        meta.sequence, meta.bytesused
                                    ),
                                },
                                Err(err) => error!("could not get frame: {}", err),
                            }
                        }
                    });
                    if let Some(texture) = self.get_current_texture(ui) {
                        egui::Frame::canvas(ui.style()).show(ui, |ui| {
                            draw_texture(&texture, ui);
                        });
                        ui.ctx()
                            .request_repaint_after(std::time::Duration::from_millis(10))
                    }
                } else if self.camera_module.has_camera() {
                    if let Err(err) = self.camera_module.make_stream() {
                        ui.label(format!("{}", err));
                    }
                } else {
                    ui.label("no active camera");
                }
            }
            MainState::Calibration => match self.calibration_img.as_mut() {
                None => {
                    ui.strong("there is no calibration image");
                    if ui.button("go to camera").clicked() {
                        self.main_state = MainState::CameraView;
                    }
                }
                Some(img) => {
                    let texture = img.get_texture(ui);
                    Frame::canvas(ui.style()).show(ui, |ui| {
                        let (to_screen, respone) = draw_texture(texture, ui);
                        let to_picture = to_screen.inverse();
                    });
                }
            },
        }
    }
}

impl SpeckApp {
    fn get_current_texture(&mut self, ui: &mut Ui) -> Option<egui::TextureHandle> {
        match CAMERA_STREAM.lock().as_mut().unwrap().next() {
            Ok((buf, meta)) => {
                match make_img_buf(buf, self.camera_module.width(), self.camera_module.height()) {
                    Some(image) => {
                        let image: RgbaImage = image.convert();
                        let image = ColorImage::from_rgba_unmultiplied(
                            [
                                self.camera_module.width() as usize,
                                self.camera_module.height() as usize,
                            ],
                            &image,
                        );
                        Some(ui.ctx().load_texture(
                            format!("frame {}", meta.sequence),
                            image,
                            egui::TextureFilter::Linear,
                        ))
                    }
                    None => {
                        error!(
                            "could not load image frame: {}, {} bytes received",
                            meta.sequence, meta.bytesused
                        );
                        None
                    }
                }
            }
            Err(err) => {
                error!("failed to read frame: {}", err);
                None
            }
        }
    }
}

fn draw_texture(texture: &TextureHandle, ui: &mut Ui) -> (emath::RectTransform, egui::Response) {
    let (response, painter) = ui.allocate_painter(
        ui.available_size_before_wrap(),
        egui::Sense::click_and_drag(),
    );

    let to_screen = emath::RectTransform::from_to(
        Rect::from_min_size(Pos2::ZERO, response.rect.square_proportions()),
        response.rect,
    );

    let mut shape = Mesh::with_texture(texture.into());

    shape.add_rect_with_uv(
        Rect {
            min: to_screen * Pos2::ZERO,
            max: to_screen
                * Pos2 {
                    y: 1.0,
                    x: texture.aspect_ratio(),
                },
        },
        Rect {
            min: Pos2::ZERO,
            max: Pos2 { y: 1.0, x: 1.0 },
        },
        Color32::WHITE,
    );
    painter.add(shape);
    (to_screen, response)
}
