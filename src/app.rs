use egui::{
    emath, mutex::Mutex, Color32, ColorImage, Frame, Mesh, Pos2, Rect, TextureHandle, Ui, Vec2,
};
use image::{buffer::ConvertBuffer, ImageBuffer, Rgb, RgbaImage};
use log::{error, warn};
use once_cell::sync::Lazy;
use v4l::{io::traits::CaptureStream, prelude::*};

use crate::{calib, cam::CameraModule, meter::Meter};

pub static CAMERA_STREAM: Lazy<Mutex<Option<MmapStream>>> = Lazy::new(Default::default);

pub fn make_img_buf(buf: &[u8], width: u32, height: u32) -> Option<ImageBuffer<Rgb<u8>, &[u8]>> {
    let image = ImageBuffer::from_raw(width, height, buf)?;
    Some(image as ImageBuffer<Rgb<u8>, &[u8]>)
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Image {
    pub width: usize,
    pub height: usize,
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

    pub fn get(&self, x: usize, y: usize) -> (u8, u8, u8) {
        assert!(x < self.width);
        assert!(y < self.height);
        let index = 3 * (y * self.width + x);
        (self.data[index], self.data[index + 1], self.data[index + 2])
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

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SpeckApp {
    #[serde(skip)]
    camera_module: CameraModule,
    #[serde(skip)]
    calibration_img: Option<Image>,
    calibration: calib::Calibration,
    #[serde(skip)]
    meter: Meter,
    main_state: MainState,
    show_camera_opts: bool,
    show_calibration: bool,
    show_meter_opts: bool,
    show_logs: bool,
}

impl Default for SpeckApp {
    fn default() -> Self {
        Self {
            camera_module: Default::default(),
            calibration_img: None,
            calibration: calib::Calibration::new(),
            main_state: Default::default(),
            meter: Default::default(),
            show_camera_opts: true,
            show_calibration: false,
            show_meter_opts: false,
            show_logs: true,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, PartialEq, Eq)]
enum MainState {
    #[default]
    CameraView,
    Calibration,
    GraphView,
}

impl SpeckApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let mut app = Self::default();
        if let Some(storage) = cc.storage {
            app.calibration = eframe::get_value(storage, "calibration").unwrap_or_default();
        }
        if app.camera_module.query().is_err() {
            warn!("could not initialise cameras")
        };
        app
    }
}

impl eframe::App for SpeckApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, "calibration", &self.calibration);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu(ui));

        if self.show_camera_opts {
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                self.camera_module.update(ui);
            });
        }

        if self.show_calibration {
            egui::SidePanel::right("calibration").show(ctx, |ui| {
                self.calibration.side_view(ui);
                if ui.button("new image").clicked() {
                    self.new_calibration_img()
                }
            });
        }

        if self.show_meter_opts {
            egui::SidePanel::right("meter").show(ctx, |ui| self.meter.side_panel(ui));
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
                    ui.checkbox(&mut self.show_logs, "Log window");
                    ui.checkbox(&mut self.show_calibration, "Calibration window");
                    ui.checkbox(&mut self.show_meter_opts, "Meter Options")
                });
                ui.horizontal_centered(|ui| {
                    if ui
                        .selectable_value(&mut self.main_state, MainState::CameraView, "📷 Camera")
                        .clicked()
                    {
                        self.show_none_but_logs();
                        self.show_camera_opts = true;
                    };
                    if ui
                        .selectable_value(
                            &mut self.main_state,
                            MainState::Calibration,
                            "⭕ Calibration",
                        )
                        .clicked()
                    {
                        self.show_none_but_logs();
                        self.show_calibration = true;
                    };
                    if ui
                        .selectable_value(
                            &mut self.main_state,
                            MainState::GraphView,
                            "Spectrograph",
                        )
                        .clicked()
                    {
                        self.show_none_but_logs();
                        self.show_meter_opts = true;
                    }
                })
            })
        });
    }

    fn main_view(&mut self, ui: &mut Ui) {
        match self.main_state {
            MainState::CameraView => {
                let stream_on = CAMERA_STREAM.lock().is_some();
                if stream_on {
                    ui.vertical_centered(|ui| {
                        if ui.button("take calibration image").clicked() {
                            self.new_calibration_img();
                        }
                        if let Some(texture) = self.get_current_texture(ui) {
                            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                                draw_texture(&texture, ui);
                            });
                            ui.ctx().request_repaint()
                        }
                    });
                } else if self.camera_module.has_camera() {
                    if let Err(err) = self.camera_module.make_stream() {
                        ui.label(format!("{}", err));
                    }
                } else {
                    ui.label("no active camera");
                }
            }
            MainState::Calibration => {
                *CAMERA_STREAM.lock() = None;
                match self.calibration_img.as_mut() {
                    None => {
                        ui.strong("there is no calibration image");
                        if ui.button("go to camera").clicked() {
                            self.main_state = MainState::CameraView;
                            self.show_calibration = true;
                            self.show_camera_opts = false;
                        }
                    }
                    Some(img) => {
                        let texture = img.get_texture(ui);
                        ui.vertical_centered(|ui| {
                            let style = ui.style();
                            Frame::canvas(style).show(ui, |ui| {
                                let (to_screen, response) = draw_texture(texture, ui);
                                self.calibration.main_view(ui, to_screen, response);
                            });
                        });
                    }
                }
            }
            MainState::GraphView => {
                if CAMERA_STREAM.lock().is_none() {
                    if let Err(err) = self.camera_module.make_stream() {
                        error!("failed to open camera stream graph view: {}", err)
                    }
                }

                match self.calibration.get_lines() {
                    Some(lines) => self.meter.main(
                        ui,
                        self.camera_module.width(),
                        self.camera_module.height(),
                        lines,
                    ),
                    None => {
                        ui.label("no calibration lines found");
                        warn!("tried to get calibration lines without generating them")
                    }
                }
            }
        }
    }

    fn show_none_but_logs(&mut self) {
        self.show_calibration = false;
        self.show_camera_opts = false;
        self.show_meter_opts = false;
    }

    fn new_calibration_img(&mut self) {
        if CAMERA_STREAM.lock().as_ref().is_none() {
            match self.camera_module.make_stream() {
                Ok(_) => (),
                Err(err) => {
                    error!("could not make camera stream: {}", err);
                    return;
                }
            }
        }
        match CAMERA_STREAM.lock().as_mut().unwrap().next() {
            Ok((buf, meta)) => {
                match make_img_buf(buf, self.camera_module.width(), self.camera_module.height()) {
                    Some(img) => {
                        self.calibration_img = Some(img.into());
                        self.main_state = MainState::Calibration;
                        self.show_calibration = true;
                        self.show_camera_opts = false;
                        self.show_meter_opts = false;
                    }
                    None => error!(
                        "could not load image frame: {}, {} bytes received",
                        meta.sequence, meta.bytesused
                    ),
                }
            }
            Err(err) => error!("could not get frame: {}", err),
        }
    }
}

impl SpeckApp {
    fn get_current_texture(&mut self, ui: &mut Ui) -> Option<egui::TextureHandle> {
        match CAMERA_STREAM.lock().as_mut()?.next() {
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
                            "could not load image frame: {},   {} bytes received",
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
    let rect = ui.available_rect_before_wrap();
    let aspect_ratio = texture.aspect_ratio();

    let (response, painter) = if rect.aspect_ratio() > aspect_ratio {
        let size = Vec2::new(rect.height() * aspect_ratio, rect.height());
        ui.allocate_painter(size, egui::Sense::click_and_drag())
    } else {
        let size = Vec2::new(rect.width(), rect.width() / aspect_ratio);
        ui.allocate_painter(size, egui::Sense::click_and_drag())
    };

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
