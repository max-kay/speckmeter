use crate::app::Image;
use egui::Ui;
use image::Rgb;
use log::error;
use log::warn;
use nokhwa::nokhwa_check;
use nokhwa::Camera;
use nokhwa::CameraControl;
use nokhwa::CameraFormat;
use nokhwa::CameraInfo;
use nokhwa::FrameFormat;
use nokhwa::NokhwaError;
use std::borrow::Cow;
use std::vec::Vec;
use tracing_subscriber::fmt::format::Format;

struct CamInner {
    camera: Camera,
    controls: Option<Vec<CameraControl>>,
    format: FrameFormat,
}

impl CamInner {
    fn new(camera: Camera) -> Self {
        let format = camera.frame_format();
        Self {
            camera,
            controls: None,
            format,
        }
    }

    pub fn get_image(&mut self) -> Result<Image, NokhwaError> {
        let resolution = self.camera.resolution();
        let mut image = Image::new(
            resolution.width() as usize/2,
            resolution.height() as usize,
        );

        if !self.camera.is_stream_open() {
            self.camera.open_stream()?;
        }

        let frame = self.camera.frame_raw()?;

        let image_data =
            match image::ImageBuffer::from_raw(resolution.width()/2, resolution.height(), frame) {
                Some(image) => {
                    let image: image::ImageBuffer<Rgb<u8>, Cow<'_, [u8]>> = image;
                    image
                }
                None => {
                    return Err(NokhwaError::ReadFrameError(
                        "Frame Cow Too Small".to_string(),
                    ))
                }
            };
        let rgba_image: image::RgbaImage = image::buffer::ConvertBuffer::convert(&image_data);
        image.mut_buff().copy_from_slice(rgba_image.as_raw());

        Ok(image)
    }

    fn update_format(&mut self) {
        self.format = self.camera.frame_format();
    }

    fn update(&mut self, ui: &mut Ui) {
        self.update_format();
        ui.heading(self.camera.info().human_name());
        ui.label(format!(
            "{}, {} FPS\n{}",
            self.camera.resolution(),
            self.camera.frame_rate(),
            self.camera.frame_format()
        ));

        if ui
            .add(egui::RadioButton::new(
                self.format == FrameFormat::MJPEG,
                "MJPEG",
            ))
            .clicked()
        {
            self.camera.stop_stream();
            if let Err(err) = self.camera.set_frame_format(FrameFormat::MJPEG) {
                error!("{}", err)
            }
        }

        if ui
            .add(egui::RadioButton::new(
                self.format == FrameFormat::YUYV,
                "YUYV",
            ))
            .clicked()
        {
            self.camera.stop_stream();
            if let Err(err) = self.camera.set_frame_format(FrameFormat::YUYV) {
                error!("{}", err)
            }
        }

        match self.camera.compatible_list_by_resolution(self.format) {
            Ok(formats) => {
                let mut resolution = self.camera.resolution();
                egui::ComboBox::from_label("format")
                    .selected_text(format!("{}", resolution))
                    .show_ui(ui, |ui| {
                        for res in formats.keys() {
                            if ui
                                .selectable_value(&mut resolution, *res, format!("{}", res))
                                .clicked()
                            {
                                self.camera.stop_stream();
                                match self.camera.set_resolution(*res) {
                                    Ok(_) => (),
                                    Err(err) => {
                                        error!("could not set resolution to {}\n{}", res, err)
                                    }
                                }
                            };
                        }
                    });
                let mut framerate = self.camera.frame_rate();
                egui::ComboBox::from_label("framerate")
                    .selected_text(framerate.to_string())
                    .show_ui(ui, |ui| {
                        for fr in formats
                            .get(&resolution)
                            .expect("resolution should be valid")
                        {
                            if ui
                                .selectable_value(&mut framerate, *fr, fr.to_string())
                                .clicked()
                            {
                                self.camera.stop_stream();
                                if let Err(err) = self.camera.set_frame_rate(*fr) {
                                    error!("{}", err)
                                }
                            }
                        }
                    });
            }
            Err(err) => error!("could not query compatible camera formats\n{}", err),
        }
        if let Some(controls) = &mut self.controls {
            if ui.button("show current camera controls").clicked() {
                match self.camera.camera_controls() {
                    Ok(ctrls) => *controls = ctrls,
                    Err(err) => warn!("failed to set control\n{}", err),
                };
            }
            for ctrl in controls.iter_mut() {
                update_ctrl(ui, ctrl, &mut self.camera);
            }
        } else {
            match self.camera.camera_controls() {
                Ok(ctrls) => self.controls = Some(ctrls),
                Err(err) => error!("could not query camera controls\n{}", err),
            }
        }
    }
}

pub fn update_ctrl(ui: &mut Ui, ctrl: &mut CameraControl, camera: &mut Camera) {
    ui.push_id(ctrl.control(), |ui| {
        ui.strong(format!("{}", ctrl.control()));
        let mut active = ctrl.active();
        if ui.toggle_value(&mut active, "active").changed() {
            camera.stop_stream();
            ctrl.set_active(active); // TODO
            match camera.set_camera_control(*ctrl) {
                Ok(_) => (),
                Err(err) => warn!("couldn't set {} to {}\n{}", ctrl.control(), active, err),
            }
        }
        let mut current_val = ctrl.value();
        let val_str = format!("{}", ctrl.value());
        egui::ComboBox::from_id_source(1)
            .selected_text(val_str)
            .show_ui(ui, |ui| {
                for val in ctrl.valid_values() {
                    if ui
                        .selectable_value(&mut current_val, val, val.to_string())
                        .clicked()
                    {
                        if let Err(err) = ctrl.set_value(current_val) {
                            warn!("invalid control value for {}\n{}", ctrl.control(), err);
                            break;
                        };
                        camera.stop_stream();
                        if let Err(err) = camera.set_camera_control(*ctrl) {
                            warn!("failed to send control value\n{}", err)
                        }
                    };
                }
            });
    });
}

pub struct CameraModule {
    inner: Option<CamInner>,
    infos: Vec<CameraInfo>,
    state: CameraState,
}

#[derive(Debug, Default)]
pub enum CameraState {
    #[default]
    Dead,
    Infos,
    Initialized,
}

impl CameraModule {
    pub fn new() -> Self {
        Self {
            inner: None,
            infos: Vec::new(),
            state: Default::default(),
        }
    }

    pub fn query(&mut self) -> Result<(), nokhwa::NokhwaError> {
        self.infos = nokhwa::query()?;
        self.state = CameraState::Infos;
        self.inner = None;
        Ok(())
    }

    pub fn active(&self) -> bool {
        matches!(self.state, CameraState::Initialized)
    }

    pub fn get_image(&mut self) -> Result<Image, NokhwaError> {
        self.inner
            .as_mut()
            .expect("module should be initialised")
            .get_image()
    }

    pub fn reset(&mut self) {
        self.infos = Vec::new();
        self.inner = None;
        self.state = CameraState::Dead;
    }

    pub fn init(&mut self, info: &CameraInfo) -> Result<(), NokhwaError> {
        let mut camera = Camera::new(info.index(), None)?;
        camera.open_stream()?;
        self.state = CameraState::Initialized;
        self.inner = Some(CamInner::new(camera));
        Ok(())
    }

    pub fn update(&mut self, ui: &mut Ui) {
        match self.state {
            CameraState::Dead => {
                ui.heading("Camera Module");
                if ui.button("get cameras").clicked() {
                    match self.query() {
                        Ok(_) => (),
                        Err(err) => error!("Querying failed: {}", err),
                    }
                }
            }
            CameraState::Infos => {
                ui.heading("Camera Module");
                for info in self.infos.clone() {
                    ui.label(format!("{}\n{}", info.human_name(), info.description()));
                    if ui.button("initialise").clicked() {
                        if let Err(err) = self.init(&info) {
                            self.reset();
                            error!("initializing failed {}", err)
                        }
                    }
                }
            }
            CameraState::Initialized => {
                self.inner
                    .as_mut()
                    .expect("camera should be initialised")
                    .update(ui);
            }
        }
        if ui.button("reset camera").clicked() {
            self.reset()
        }
        if nokhwa_check() {
            ui.label("nokhwa is ready");
        }
    }
}

impl Default for CameraModule {
    fn default() -> Self {
        Self::new()
    }
}
