use egui::{ComboBox, Context, DragValue, Slider, TextureHandle, Ui};
use itertools::CombinationsWithReplacement;
use log::{error, warn};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{
        CameraControl, CameraIndex, CameraInfo, ControlValueDescription, FrameFormat,
        RequestedFormat,
    },
    Camera, NokhwaError,
};
use std::{error::Error, fmt::Display};

pub mod my_image;

pub use my_image::Image;

use crate::app::draw_texture;

pub struct CameraModule {
    active_camera: Option<ActiveCamera>,
    devices: Vec<CameraInfo>,
}

impl CameraModule {
    pub fn display(&mut self, ctx: &Context, calibration_image: &mut Option<Image>) {
        egui::SidePanel::left("spectrograph_opts").show(ctx, |ui| self.side_panel(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_camera.is_none() {
                ui.heading("No active camera");
                return;
            }
            let cam = self.active_camera.as_mut().unwrap();
            match cam.get_img() {
                Err(err) => error!("could not get new image, Error: {}", err),
                Ok(mut image) => {
                    draw_texture(image.get_texture(ui.ctx()), ui);
                    if ui.button("take calibration image").clicked() {
                        *calibration_image = Some(image);
                    };
                    ui.ctx().request_repaint();
                }
            }
        });
    }
}

impl CameraModule {
    pub fn new() -> Self {
        Self {
            active_camera: None,
            devices: Vec::new(),
        }
    }

    pub fn query(&mut self) -> Result<(), NokhwaError> {
        self.devices = nokhwa::query(nokhwa::utils::ApiBackend::Auto)?;
        self.active_camera = None;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.devices = Vec::new();
        self.active_camera = None;
    }

    pub fn get_img(&mut self) -> Result<Image, Box<dyn Error>> {
        match self.active_camera.as_mut() {
            Some(cam) => Ok(cam.get_img()?),
            None => Err(CameraError::CameraNotActive)?,
        }
    }
}

impl CameraModule {
    pub fn side_panel(&mut self, ui: &mut Ui) {
        ui.heading("Camera Module");
        if ui.button("reset camera").clicked() {
            self.reset()
        }
        if let Some(cam) = self.active_camera.as_mut() {
            cam.side_panel(ui);
            return;
        }

        if !self.devices.is_empty() {
            for device in self.devices.iter() {
                ui.label(device.human_name());
                if ui.button("initialise").clicked() {
                    match ActiveCamera::new(device.index().clone()) {
                        Ok(inner) => self.active_camera = Some(inner),
                        Err(err) => error!("{}", err),
                    }
                }
            }
            return;
        }

        if ui.button("get cameras").clicked() {
            if let Err(err) = self.query() {
                error!("Querying failed: {}", err)
            }
        }
    }
}

impl Default for CameraModule {
    fn default() -> Self {
        Self::new()
    }
}

struct ActiveCamera {
    camera: Camera,
    controls: Vec<CameraControl>,
    show_controls: bool,
}

impl ActiveCamera {
    fn new(index: CameraIndex) -> Result<Self, NokhwaError> {
        let format = RequestedFormat::new::<RgbFormat>(nokhwa::utils::RequestedFormatType::None);
        let camera = Camera::new(index, format)?;
        let controls = camera.camera_controls()?;
        Ok(Self {
            camera,
            controls,
            show_controls: false,
        })
    }
}

impl ActiveCamera {
    fn side_panel(&mut self, ui: &mut Ui) {
        ui.heading("Camera Module");
        ui.strong(self.camera.info().human_name());

        match self.camera.refresh_camera_format() {
            Ok(_) => (),
            Err(err) => error!("could not refresh camera format, Error: {}", err),
        };
        let current_format = self.camera.camera_format();
        let mut current_resolution = current_format.resolution(); // mut because it is used to search for framerates after setting resolution
        let current_frame_rate = current_format.frame_rate();

        ui.label(format!(
            "{}, {} FPS",
            current_resolution, current_frame_rate
        ));

        match self.camera.compatible_fourcc() {
            Ok(compatible_formats) => {
                ComboBox::from_label(current_format.format().to_string()).show_ui(ui, |ui| {
                    for frame_format in compatible_formats {
                        if ui
                            .selectable_label(
                                current_format.format() == frame_format,
                                frame_format.to_string(),
                            )
                            .clicked()
                        {
                            match self.camera.set_frame_format(frame_format) {
                                Ok(_) => (),
                                Err(err) => {
                                    error!("could not set camera format Error: {}", err)
                                }
                            }
                        }
                    }
                });
            }
            Err(err) => {
                error!("could not querry compatible frame formats, Error: {}", err);
                ui.label("could not querry frame formats");
                return;
            }
        }

        match self
            .camera
            .compatible_list_by_resolution(current_format.format())
        {
            Ok(resolutions_map) => {
                let mut compatible_resolutions = Vec::from_iter(resolutions_map.keys());
                compatible_resolutions.sort();
                ComboBox::from_label(current_resolution.to_string()).show_ui(ui, |ui| {
                    for resolution in compatible_resolutions {
                        if ui
                            .selectable_label(
                                current_resolution == *resolution,
                                resolution.to_string(),
                            )
                            .clicked()
                        {
                            if let Err(err) = self.camera.set_resolution(*resolution) {
                                error!("{}", err)
                            }
                            current_resolution = *resolution
                        }
                    }
                });
                ComboBox::from_label(current_frame_rate.to_string()).show_ui(ui, |ui| {
                    for frame_rate in resolutions_map.get(&current_resolution).unwrap() {
                        if ui
                            .selectable_label(
                                current_frame_rate == *frame_rate,
                                frame_rate.to_string(),
                            )
                            .clicked()
                        {
                            if let Err(err) = self.camera.set_frame_rate(*frame_rate) {
                                error!("{}", err)
                            }
                        }
                    }
                });
            }
            Err(err) => {
                error!("could not querry compatible frame formats, Error: {}", err);
                ui.label("could not querry frame formats");
                return;
            }
        }

        ui.checkbox(&mut self.show_controls, "show controls");
        if self.show_controls {
            self.all_controls_ui(ui);
        }
    }

    fn all_controls_ui(&mut self, ui: &mut Ui) {
        if ui.button("fetch controls").clicked() {
            if let Err(err) = self.fetch_controls() {
                ui.label("failed to fetch camera controls");
                error!("failed to fetch controls, Error: {}", err);
                return;
            }
        }
        for control in self.controls.iter_mut() {
            if control_ui(ui, control) {
                if let Err(err) = self
                    .camera
                    .set_camera_control(control.control(), control.value())
                {
                    error!(
                        "failed to set control: {}, Error: {}",
                        control.control(),
                        err
                    )
                }
            };
        }
    }

    fn fetch_controls(&mut self) -> Result<(), NokhwaError> {
        self.controls = self.camera.camera_controls()?;
        Ok(())
    }
}

fn control_ui(ui: &mut Ui, control: &mut CameraControl) -> bool {
    ui.strong(format!("{}", control.control()));
    let descr = control.description().clone();
    match descr {
        ControlValueDescription::Integer {
            mut value,
            default,
            step,
        } => {
            let response = ui.add(DragValue::new(&mut value));
            if response.changed() {
                let description = ControlValueDescription::Integer {
                    value,
                    default,
                    step,
                };
                *control = CameraControl::new(
                    control.control(),
                    control.name().to_string(),
                    description,
                    control.flag().to_vec(),
                    control.active(),
                )
            }
            if response.drag_released() {
                let val = (value / step) * step;
                let description = ControlValueDescription::Integer {
                    value: val,
                    default,
                    step,
                };
                *control = CameraControl::new(
                    control.control(),
                    control.name().to_string(),
                    description,
                    control.flag().to_vec(),
                    control.active(),
                );
                true
            } else {
                false
            }
        }
        ControlValueDescription::IntegerRange {
            min,
            max,
            mut value,
            step,
            default,
        } => {
            let response = ui.add(DragValue::new(&mut value));
            if response.changed() {
                let description = ControlValueDescription::Integer {
                    value,
                    default,
                    step,
                };
                *control = CameraControl::new(
                    control.control(),
                    control.name().to_string(),
                    description,
                    control.flag().to_vec(),
                    control.active(),
                )
            }
            if response.drag_released() {
                let mut val = ((value - min) / step) * step + min;
                if val >= max {
                    val = max - step
                }
                let description = ControlValueDescription::IntegerRange {
                    min,
                    max,
                    value: val,
                    default,
                    step,
                };
                *control = CameraControl::new(
                    control.control(),
                    control.name().to_string(),
                    description,
                    control.flag().to_vec(),
                    control.active(),
                );
                true
            } else {
                false
            }
        }
        ControlValueDescription::Boolean { mut value, default } => {
            if ui.checkbox(&mut value, "active").changed() {
                let description = ControlValueDescription::Boolean { value, default };
                *control = CameraControl::new(
                    control.control(),
                    control.name().to_string(),
                    description,
                    control.flag().to_vec(),
                    control.active(),
                );
                true
            } else {
                false
            }
        }
        // ControlValueDescription::Float {
        //     value,
        //     default,
        //     step,
        // } => todo!(),
        // ControlValueDescription::FloatRange {
        //     min,
        //     max,
        //     value,
        //     step,
        //     default,
        // } => todo!(),
        _ => {
            warn!("ignoring the control value: {}", control.value());
            false
        }
    }
}

impl ActiveCamera {
    fn get_img(&mut self) -> Result<Image, NokhwaError> {
        if !self.camera.is_stream_open() {
            self.camera.open_stream()?
        }
        let buf = self.camera.frame()?;
        Image::new(buf)
    }
}

#[derive(Debug)]
pub enum CameraError {
    CameraNotActive,
}

impl Display for CameraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CameraError::CameraNotActive => f.write_str("The camera was not active"),
        }
    }
}

impl Error for CameraError {}
