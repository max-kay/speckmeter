use egui::{emath, Color32, Frame, Mesh, Pos2, Rect, TextureHandle, Ui, Vec2};
use log::{error, warn};

use crate::{
    calibration_module::CalibrationModule,
    camera_module::{CameraModule, CameraStream, Image},
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
    calibration: CalibrationModule,
    #[serde(skip)]
    meter: SpectrographModule,
    #[serde(skip)]
    tracer: TracerModule,
    main_state: MainState,
    show_camera_opts: bool,
    show_calibration: bool,
    show_meter_opts: bool,
    show_tracer_opts: bool,
    show_logs: bool,
}

impl Default for SpeckApp {
    fn default() -> Self {
        Self {
            camera_module: Default::default(),
            calibration_img: None,
            calibration: CalibrationModule::new(),
            main_state: Default::default(),
            meter: Default::default(),
            tracer: Default::default(),
            show_camera_opts: true,
            show_calibration: false,
            show_meter_opts: false,
            show_tracer_opts: false,
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
    TracerView,
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
        egui::TopBottomPanel::top("menu").show(ctx, |ui| self.side_panel_menu(ui));

        if self.show_camera_opts {
            egui::SidePanel::left("side_panel").show(ctx, |ui| {
                self.camera_module.update(ui);
            });
        }

        if self.show_calibration {
            egui::SidePanel::right("calibration").show(ctx, |ui| {
                self.calibration.side_panel(ui);
                if ui.button("new image").clicked() {
                    self.new_calibration_img()
                }
            });
        }

        if self.show_meter_opts {
            egui::SidePanel::right("meter").show(ctx, |ui| self.meter.side_panel(ui));
        }

        if self.show_tracer_opts {
            egui::SidePanel::right("tracer").show(ctx, |ui| self.tracer.side_panel(ui));
        }

        if self.show_logs {}

        egui::CentralPanel::default().show(ctx, |ui| {
            self.main_view(ui);
        });
    }
}

impl SpeckApp {
    fn side_panel_menu(&mut self, ui: &mut Ui) {
        egui::menu::bar(ui, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Sidepanels", |ui| {
                    ui.checkbox(&mut self.show_camera_opts, "Camera Module");
                    ui.checkbox(&mut self.show_logs, "Log window");
                    ui.checkbox(&mut self.show_calibration, "Calibration window");
                    ui.checkbox(&mut self.show_meter_opts, "Meter Options")
                });
                ui.horizontal_centered(|ui| {
                    self.menu(ui);
                })
            })
        });
    }

    fn menu(&mut self, ui: &mut Ui) {
        if ui
            .selectable_value(&mut self.main_state, MainState::CameraView, "📷 Camera")
            .clicked()
        {
            self.close_side_panels();
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
            self.close_side_panels();
            self.show_calibration = true;
        };
        if ui
            .selectable_value(&mut self.main_state, MainState::GraphView, "Spectrograph")
            .clicked()
        {
            self.close_side_panels();
            self.show_meter_opts = true;
        }
        if ui
            .selectable_value(&mut self.main_state, MainState::TracerView, "Tracer")
            .clicked()
        {
            self.close_side_panels();
            self.show_tracer_opts = true;
        }
    }

    fn main_view(&mut self, ui: &mut Ui) {
        match self.main_state {
            MainState::CameraView => {
                self.camera_view(ui);
            }
            MainState::Calibration => {
                self.calibration_view(ui);
            }
            MainState::GraphView => {
                self.graph_view(ui);
            }
            MainState::TracerView => {
                self.tracer_view(ui);
            }
        }
    }

    fn tracer_view(&mut self, ui: &mut Ui) {
        self.tracer.main(
            ui,
            &mut self.calibration,
            self.camera_module.width(),
            self.camera_module.height(),
        )
    }

    fn graph_view(&mut self, ui: &mut Ui) {
        self.camera_module.make_stream();
        self.meter.main(
            ui,
            self.camera_module.width(),
            self.camera_module.height(),
            &mut self.calibration,
        )
    }

    fn calibration_view(&mut self, ui: &mut Ui) {
        CameraStream::close();
        match self.calibration_img.as_mut() {
            None => {
                ui.horizontal_centered(|ui| {
                    ui.strong("there is no calibration image");
                    if ui.button("go to camera").clicked() {
                        self.close_side_panels();
                        self.show_camera_opts = true;
                        self.main_state = MainState::CameraView;
                    }
                    if ui.button("take calibration image").clicked() {
                        self.new_calibration_img()
                    }
                });
            }
            Some(img) => {
                let aspect_ratio = img.aspect_ratio();
                let texture = img.get_texture(ui);
                ui.vertical_centered(|ui| {
                    let style = ui.style();
                    Frame::canvas(style).show(ui, |ui| {
                        let (to_screen, response) = draw_texture(texture, ui);
                        self.calibration
                            .main_view(ui, to_screen, aspect_ratio, response);
                    });
                });
            }
        }
    }

    fn camera_view(&mut self, ui: &mut Ui) {
        if CameraStream::is_open() {
            ui.vertical_centered(|ui| {
                if ui.button("take calibration image").clicked() {
                    self.new_calibration_img();
                }
                if let Some(texture) = CameraStream::get_img_as_texture(
                    ui.ctx(),
                    self.camera_module.width(),
                    self.camera_module.height(),
                ) {
                    egui::Frame::canvas(ui.style()).show(ui, |ui| {
                        draw_texture(&texture, ui);
                    });
                    ui.ctx().request_repaint()
                }
            });
        } else if self.camera_module.has_camera() {
            self.camera_module.make_stream()
        } else {
            ui.label("no active camera");
        }
    }

    fn close_side_panels(&mut self) {
        self.show_calibration = false;
        self.show_camera_opts = false;
        self.show_meter_opts = false;
        self.show_tracer_opts = false;
    }

    fn new_calibration_img(&mut self) {
        self.camera_module.make_stream();

        if let Some(img) =
            CameraStream::get_img(self.camera_module.width(), self.camera_module.height())
        {
            self.calibration_img = Some(img);
            self.main_state = MainState::Calibration;
            self.show_calibration = true;
            self.show_camera_opts = false;
            self.show_meter_opts = false;
            self.show_tracer_opts = false;
        } else {
            error!("could not take calibration image")
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
