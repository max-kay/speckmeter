use egui::{Context, DragValue, Slider, TextureHandle, Ui};
use log::{error, warn};
use nokhwa::{
    pixel_format::RgbFormat,
    utils::{
        CameraControl, CameraIndex, CameraInfo, ControlValueSetter, FrameFormat, RequestedFormat,
    },
    Camera, NokhwaError,
};
use std::{any::Any, error::Error, fmt::Display};

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
    }
}

impl CameraModule {
    pub fn new() -> Self {
        Self {
            active_camera: None,
            devices: Vec::new(),
        }
    }

    // #[cfg(target_os = "linux")]
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
                    match ActiveCamera::new(*device.index()) {
                        Ok(inner) => self.active_camera = Some(inner),
                        Err(err) => error!("{}", err),
                    }
                }
            }
            return;
        }

        if ui.button("get cameras").clicked() {
            match self.query() {
                Ok(_) => todo!(),
                Err(err) => error!("Querying failed: {}", err),
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
        let format: RequestedFormat<'_> = RequestedFormat::new::<RgbFormat>(
            nokhwa::utils::RequestedFormatType::HighestResolutionAbs,
        );
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

        self.camera.refresh_camera_format();
        let current_format = self.camera.camera_format();
        let mut current_resolution = current_format.resolution(); // mut because it is used to search for framerates after setting resolution
        let current_frame_rate = current_format.frame_rate();

        if current_format.format() != FrameFormat::RAWRGB {
            if let Err(err) = self.camera.set_frame_format(FrameFormat::RAWRGB) {
                error!("failed to set frame format, Error: {}", err);
            }
        }

        ui.label(format!(
            "{}, {} FPS",
            current_resolution, current_frame_rate
        ));

        match self.camera.compatible_fourcc() {
            Ok(compatible_formats) => {
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
                            Err(err) => error!("could not set camera format Error: {}", err),
                        }
                    }
                }
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
                for resolution in compatible_resolutions {
                    if ui
                        .selectable_label(current_resolution == *resolution, resolution.to_string())
                        .clicked()
                    {
                        if let Err(err) = self.camera.set_resolution(*resolution) {
                            error!("{}", err)
                        }
                        current_resolution = *resolution
                    }
                }

                for frame_rate in resolutions_map.get(&current_resolution).unwrap() {
                    if ui
                        .selectable_label(current_frame_rate == *frame_rate, frame_rate.to_string())
                        .clicked()
                    {
                        if let Err(err) = self.camera.set_frame_rate(*frame_rate) {
                            error!("{}", err)
                        }
                    }
                }
            }
            Err(err) => {
                error!("could not querry compatible frame formats, Error: {}", err);
                ui.label("could not querry frame formats");
                return;
            }
        }

        ui.checkbox(&mut self.show_controls, "show controls");
        if self.show_controls {
            self.control_ui(ui);
        }
    }

    fn control_ui(&mut self, ui: &mut Ui) {
        if let Err(err) = self.fetch_controls() {
            ui.label("failed to fetch camera controls");
            error!("failed to fetch controls, Error: {}", err);
            return;
        }
        for control in self.controls.iter_mut() {
            ui.label(format!("{}", control.control()));
            ui.label(format!("flags: {:?}", control.flag()));

            // let mut active = control.active();
            // if ui.checkbox(&mut active, "active").clicked() {
            //     println!("{} and {}", active, control.active());
            //     control.set_active(active);
            //     if let Err(err) = self.camera. {
            //         error!(
            //             "could not set control value for {}, Error: {}",
            //             control.control(),
            //             err
            //         )
            //     }
            // }

            match control.value() {
                ControlValueSetter::Integer(val) => {]
                    if 
                },
                ControlValueSetter::Float(_) => todo!(),
                ControlValueSetter::Boolean(mut val) => {
                    if ui.checkbox(&mut val, "").clicked() {
                        if let Err(err) = self.camera
                            .set_camera_control(control.control(), ControlValueSetter::Boolean(val)) {
                                error!("could not set control value {}", err)
                            }
                    }
                }
                _ => warn!("ignoring the control value: {}", control.value()),
            };
            if ui
                .add(Slider::new(
                    &mut value,
                    control.minimum_value()..=control.maximum_value(),
                ))
                .changed()
            {
                if let Err(err) = control.set_value(value) {
                    error!("failed to set value of {}, Error: {}", control, err)
                }
                if let Err(err) = self.camera.set_camera_control(*control) {
                    error!(
                        "could not set control value for {}, Error: {}",
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

impl ActiveCamera {
    fn get_img(&mut self) -> Result<Image, NokhwaError> {
        if !self.camera.is_stream_open() {
            self.camera.open_stream()?
        }
        let buf = self.camera.frame()?;
        Ok(buf.into())
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
