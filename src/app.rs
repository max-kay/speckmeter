use egui::{mutex::Mutex, ColorImage};
use image::{buffer::ConvertBuffer, ImageBuffer, Rgb, RgbaImage};
use log::error;
use once_cell::sync::Lazy;
use v4l::{io::traits::CaptureStream, prelude::*};

use crate::cam::CameraModule;

pub static CAMERA_STREAM: Lazy<Mutex<Option<MmapStream>>> = Lazy::new(Default::default);

pub struct Image {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

// impl Image {
//     pub fn new(width: usize, height: usize, buf: &[u8]) -> Self {
//         Self {
//             width,
//             height,
//             data: buf.into(),
//         }
//     }
//
//     pub fn mut_buff(&mut self) -> &mut [u8] {
//         &mut self.data
//     }
//
//     pub fn as_color_image(&self) -> egui::ColorImage {
//         egui::ColorImage::rg
//     }
//
//     fn draw(&mut self, ui: &mut Ui) {
//         let texture: egui::TextureHandle = ui.ctx().load_texture(
//             "camera view",
//             self.as_color_image(),
//             egui::TextureFilter::Linear,
//         );
//
//         ui.image(&texture, ui.ctx().used_size());
//     }
// }

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct SpeckApp {
    #[serde(skip)]
    camera_module: CameraModule,
    main_state: MainState,
    show_logs: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
enum MainState {
    #[default]
    Off,
    CameraView,
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

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.selectable_label(matches!(self.main_state, MainState::Off), "Off").clicked() {
                        self.main_state = MainState::Off;
                    }
                    if ui.selectable_label(matches!(self.main_state, MainState::CameraView), "Camera").clicked() {
                        self.main_state = MainState::CameraView;
                    }
                });
                egui::warn_if_debug_build(ui);
            });
        });

        egui::SidePanel::left("side_panel").show(ctx, |ui| {
            self.camera_module.update(ui);
        });

        egui::CentralPanel::default().show(ctx, |ui| match self.main_state {
            MainState::Off => {
                ui.strong("no view");
            }
            MainState::CameraView => {
                let stream_on = CAMERA_STREAM.lock().is_some();
                if stream_on {
                    match CAMERA_STREAM.lock().as_mut().unwrap().next() {
                        Ok((buf, meta)) => {
                            match ImageBuffer::from_raw(
                                self.camera_module.width(),
                                self.camera_module.height(),
                                buf,
                            ) {
                                Some(image) => {
                                    let image: RgbaImage =
                                        (image as ImageBuffer<Rgb<u8>, &[u8]>).convert();
                                    let image = ColorImage::from_rgba_unmultiplied(
                                        [
                                            self.camera_module.width() as usize,
                                            self.camera_module.height() as usize,
                                        ],
                                        &image,
                                    );
                                    let texture: egui::TextureHandle = ui.ctx().load_texture(
                                        format!("frame {}", meta.sequence),
                                        image,
                                        egui::TextureFilter::Linear,
                                    );
                                    ui.image(&texture, ui.ctx().used_size());
                                }
                                None => {
                                    error!("could not load image frame: {}, {} bytes\n    suposed to have {}", meta.sequence, buf.len(), self.camera_module.width()* self.camera_module.height()*3)
                                }
                            };
                        }
                        Err(_) => todo!(),
                    }
                } else if self.camera_module.has_camera() {
                    if let Err(err) = self.camera_module.make_stream() {
                        ui.label(format!("{}", err));
                    }
                } else {
                    ui.label("no active camera");
                }
            }
        });

        if self.show_logs {}
        if false {
            egui::Window::new("Window").show(ctx, |ui| {
                ui.label("Windows can be moved by dragging them.");
                ui.label("They are automatically sized based on contents.");
                ui.label("You can turn on resizing and scrolling if you like.");
                ui.label("You would normally chose either panels OR windows.");
            });
        }
    }
}
