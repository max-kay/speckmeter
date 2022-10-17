use std::vec::Vec;

use egui::Ui;

use crate::cam::CameraModule;

pub struct Image {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl Image {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            data: vec![0; width * height * 4],
        }
    }

    pub fn mut_buff(&mut self) -> &mut [u8] {
        &mut self.data
    }

    pub fn as_color_image(&self) -> egui::ColorImage {
        egui::ColorImage::from_rgba_unmultiplied([self.width, self.height], &self.data)
    }

    fn draw(&mut self, ui: &mut Ui) {
        let texture: egui::TextureHandle = ui.ctx().load_texture(
            "camera view",
            self.as_color_image(),
            egui::TextureFilter::Linear,
        );

        ui.image(&texture, ui.ctx().used_size());
    }
}

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
    Off,
    #[default]
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
        #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.close();
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
                if self.camera_module.active() {
                    match self.camera_module.get_image() {
                        Ok(mut image) => image.draw(ui),
                        Err(err) => {
                            ui.label(format!("failed to load image\n{}", err));
                        }
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
