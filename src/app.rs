use egui::{emath, Color32, Mesh, Pos2, Rect, TextureHandle, Ui, Vec2};
use log::warn;

use crate::{
    calibration_module::CalibrationModule,
    camera_module::{CameraModule, Image},
    spectrum_module::SpectrographModule,
    tracer_module::TracerModule,
};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SpeckApp {
    #[serde(skip)]
    camera_module: CameraModule,
    #[serde(skip)]
    calibration_img: Option<Image>,
    calibration_module: CalibrationModule,
    #[serde(skip)]
    spectrograph_module: SpectrographModule,
    #[serde(skip)]
    tracer_module: TracerModule,
    state: State,
    show_logs: bool,
}

impl Default for SpeckApp {
    fn default() -> Self {
        Self {
            camera_module: Default::default(),
            calibration_img: None,
            calibration_module: CalibrationModule::new(),
            state: Default::default(),
            spectrograph_module: Default::default(),
            tracer_module: Default::default(),
            show_logs: true,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    CameraView,
    Calibration,
    GraphView,
    TracerView,
}

impl SpeckApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customized the look at feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        let mut app = Self::default();
        if let Some(storage) = cc.storage {
            app.calibration_module = eframe::get_value(storage, "calibration").unwrap_or_default();
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
        eframe::set_value(storage, "calibration", &self.calibration_module);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.menu(ui));
        match self.state {
            State::CameraView => {
                self.camera_module
                    .display(ctx, &mut self.calibration_img, &mut self.state)
            }
            State::Calibration => self.calibration_module.display(
                ctx,
                &mut self.state,
                &mut self.calibration_img,
                self.camera_module.width(),
                self.camera_module.height(),
            ),
            State::GraphView => self.spectrograph_module.display(
                ctx,
                self.camera_module.width(),
                self.camera_module.height(),
                &mut self.calibration_module,
            ),
            State::TracerView => self.tracer_module.display(
                ctx,
                &mut self.calibration_module,
                self.camera_module.width(),
                self.camera_module.height(),
            ),
        }

        if self.show_logs {}
    }
}

impl SpeckApp {
    fn menu(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.selectable_value(&mut self.state, State::CameraView, "📷 Camera");
                ui.selectable_value(&mut self.state, State::Calibration, "⭕ Calibration");
                ui.selectable_value(&mut self.state, State::GraphView, "Spectrograph");
                ui.selectable_value(&mut self.state, State::TracerView, "Tracer");
            });
        });
    }
}

pub fn draw_texture(
    texture: &TextureHandle,
    ui: &mut Ui,
) -> (emath::RectTransform, egui::Response) {
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
