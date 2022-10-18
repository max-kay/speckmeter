use crate::app::CAMERA_STREAM;
use egui::Ui;
use log::{error, warn};
use std::{io::Result, vec::Vec};
use v4l::{
    buffer,
    context::Node,
    control,
    frameinterval::FrameIntervalEnum,
    prelude::*,
    video::{capture::Parameters, Capture},
    Format, FourCC, Fraction,
};

struct CamInner {
    camera: Device,
    // caps: Capabilities,
    controls: Vec<control::Description>,
    fourcc: FourCC,
    width: u32,
    height: u32,
    interval: Fraction,
}

impl CamInner {
    fn new(index: usize) -> Result<Self> {
        let camera = Device::new(index)?;
        // let caps = camera.query_caps()?;
        let controls = camera.query_controls()?;
        let format = camera.format()?;
        let param = camera.params()?;
        Ok(Self {
            camera,
            controls,
            fourcc: format.fourcc,
            width: format.width,
            height: format.height,
            interval: param.interval,
        })
    }

    pub fn make_stream(&mut self) -> Result<()> {
        *CAMERA_STREAM.lock() = Some(MmapStream::with_buffers(
            &self.camera,
            buffer::Type::VideoCapture,
            5,
        )?);
        Ok(())
    }
}

impl CamInner {
    fn update(&mut self, ui: &mut Ui) {
        // ui.heading(self.camera.info().human_name());
        ui.label(format!(
            "{}x{}\n{}",
            self.width,
            self.height,
            self.fourcc
                .str()
                .expect("FourCC not representable as string"),
        ));

        egui::ComboBox::from_label("format")
            .selected_text(self.fourcc.str().expect("FourCC not utf-8"))
            .show_ui(ui, |ui| match self.camera.enum_formats() {
                Ok(formats) => {
                    for f in formats {
                        if ui
                            .selectable_label(
                                self.fourcc == f.fourcc,
                                f.fourcc.str().expect("FourCC not utf-8"),
                            )
                            .clicked()
                        {
                            match self.camera.set_format(&Format::new(
                                self.width,
                                self.height,
                                f.fourcc,
                            )) {
                                Ok(format) => {
                                    self.width = format.width;
                                    self.height = format.height;
                                    self.fourcc = format.fourcc;
                                }
                                Err(err) => error!("{}", err),
                            };
                        }
                    }
                }
                Err(err) => error!("{}", err),
            });

        egui::ComboBox::from_label("size")
            .selected_text(format!("{}x{}", self.width, self.height))
            .show_ui(ui, |ui| match self.camera.enum_framesizes(self.fourcc) {
                Ok(sizes) => {
                    for s in sizes {
                        for size in s.size.to_discrete() {
                            let width = size.width;
                            let height = size.height;
                            if ui
                                .selectable_label(
                                    self.width == width && self.height == height,
                                    format!("{}x{}", width, height),
                                )
                                .clicked()
                            {
                                match self.camera.set_format(&Format::new(
                                    width,
                                    height,
                                    self.fourcc,
                                )) {
                                    Ok(format) => {
                                        self.width = format.width;
                                        self.height = format.height;
                                        self.fourcc = format.fourcc;
                                    }
                                    Err(err) => error!("{}", err),
                                }
                            };
                        }
                    }
                }
                Err(err) => error!("{}", err),
            });

        egui::ComboBox::from_label("FPS")
        .selected_text((self.interval.denominator as f32 / self.interval.numerator as f32).to_string())
        .show_ui(ui, |ui| {
            match self
                .camera
                .enum_frameintervals(self.fourcc, self.width, self.height)
            {
                Ok(stuff) => {
                    for elem in stuff {
                        match elem.interval {
                            FrameIntervalEnum::Discrete(interval) => {
                                if ui
                                    .selectable_label(
                                        self.interval.numerator == interval.numerator
                                            && self.interval.denominator == interval.denominator,
                                        (interval.denominator as f32 / interval.numerator as f32)
                                            .to_string(),
                                    )
                                    .clicked()
                                {
                                    match self.camera.set_params(&Parameters::new(interval)) {
                                        Ok(para) => {
                                            self.interval = para.interval;
                                        }
                                        Err(err) => error!("{}", err),
                                    }
                                }
                            }
                            FrameIntervalEnum::Stepwise(_) =>{
                                error!("if this error shows up you'll have some pain integrating this :)");
                                todo!()
                            },
                        }
                    }
                }
                Err(err) => error!("{}", err),
            }
        });

        // if ui.button("show current camera controls").clicked() {
        //     match self.camera.camera_controls() {
        //         Ok(ctrls) => *controls = ctrls,
        //         Err(err) => warn!("failed to set control\n{}", err),
        //     };
        // }
        // for ctrl in controls.iter_mut() {
        //     update_ctrl(ui, ctrl, &mut self.camera);
        // }
    }
}

pub struct CameraModule {
    inner: Option<CamInner>,
    nodes: Vec<Node>,
}

impl CameraModule {
    pub fn new() -> Self {
        Self {
            inner: None,
            nodes: Vec::new(),
        }
    }

    pub fn query(&mut self) -> Result<()> {
        self.nodes = v4l::context::enum_devices();
        self.inner = None;
        Ok(())
    }

    pub fn make_stream(&mut self) -> Result<()> {
        self.inner
            .as_mut()
            .expect("module should be initialised")
            .make_stream()
    }

    pub fn reset(&mut self) {
        self.nodes = Vec::new();
        self.inner = None;
        *CAMERA_STREAM.lock() = None;
    }

    pub fn has_camera(&self) -> bool {
        self.inner.is_some()
    }

    pub fn width(&self) -> u32 {
        self.inner
            .as_ref()
            .expect("inner should be initialised")
            .width
    }

    pub fn height(&self) -> u32 {
        self.inner
            .as_ref()
            .expect("inner should be initialised")
            .height
    }
}
impl CameraModule {
    pub fn update(&mut self, ui: &mut Ui) {
        ui.heading("Camera Module");
        match (!self.nodes.is_empty(), self.inner.is_some()) {
            (false, false) => {
                if ui.button("get cameras").clicked() {
                    match self.query() {
                        Ok(_) => (),
                        Err(err) => error!("Querying failed: {}", err),
                    }
                }
            }
            (true, false) => {
                for node in self.nodes.iter() {
                    match node.name() {
                        Some(name) => {
                            ui.label(name);
                            if ui.button("initialise").clicked() {
                                match CamInner::new(node.index()) {
                                    Ok(inner) => self.inner = Some(inner),
                                    Err(err) => error!("{}", err),
                                }
                            }
                        }
                        None => warn!("could not read camera name at idx: {}", node.index()),
                    }
                }
            }
            (true, true) => {
                self.inner
                    .as_mut()
                    .expect("camera should be initialised")
                    .update(ui);
            }
            (false, true) => {
                unreachable!()
            }
        }
        if ui.button("reset camera").clicked() {
            self.reset()
        }
    }
}

impl Default for CameraModule {
    fn default() -> Self {
        Self::new()
    }
}
