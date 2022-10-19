use crate::app::CAMERA_STREAM;
use core::panic;
use egui::{Slider, Ui};
use log::{error, warn};
use std::{io::Result, vec::Vec};
use v4l::{
    buffer,
    context::Node,
    control,
    format::Colorspace,
    frameinterval::FrameIntervalEnum,
    prelude::*,
    video::{capture::Parameters, Capture},
    Control, Format, FourCC, Fraction,
};

struct CamInner {
    camera: Device,
    controls: Vec<(control::Description, Control)>,
    color_space: Colorspace,
    fourcc: FourCC,
    width: u32,
    height: u32,
    interval: Fraction,
    show_controls: bool,
}

impl CamInner {
    fn new(index: usize) -> Result<Self> {
        let camera = Device::new(index)?;
        // let caps = camera.query_caps()?;

        let mut formats = camera.enum_formats()?;
        formats.retain(|f| f.fourcc == FourCC::new(b"RGB3"));
        let mut format = camera.format()?;
        if !formats.is_empty() {
            format.fourcc = formats[0].fourcc;
            match camera.set_format(&format) {
                Ok(f) => format = f,
                Err(err) => return Err(err),
            };
        }

        let controls = fetch_controls(&camera)?;

        let param = camera.params()?;
        Ok(Self {
            camera,
            controls,
            color_space: format.colorspace,
            fourcc: format.fourcc,
            width: format.width,
            height: format.height,
            interval: param.interval,
            show_controls: false,
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

fn fetch_controls(camera: &Device) -> Result<Vec<(control::Description, Control)>> {
    let ctrl_description = camera.query_controls()?;
    let mut controls = Vec::new();
    for d in ctrl_description {
        match camera.control(d.id) {
            Ok(control) => controls.push((d, control)),
            Err(err) => warn!(
                "failed to load value for {} disregarding it. Err:{}",
                d.name, err
            ),
        }
    }
    Ok(controls)
}

impl CamInner {
    fn update(&mut self, ui: &mut Ui) {
        // ui.heading(self.camera.info().human_name());
        ui.label(format!(
            "{}x{}\n{} - {}",
            self.width,
            self.height,
            self.fourcc
                .str()
                .expect("FourCC not representable as string"),
            self.color_space,
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
                            *CAMERA_STREAM.lock() = None;
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
                                *CAMERA_STREAM.lock() = None;
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
                                    *CAMERA_STREAM.lock() = None;
                                    match self.camera.set_params(&Parameters::new(interval)) {
                                        Ok(para) => {
                                            self.interval = para.interval;
                                        }
                                        Err(err) => error!("{}", err),
                                    }
                                }
                            }
                            FrameIntervalEnum::Stepwise(_) =>{
                                error!("if this error shows up you'll have some pain implementing this :)");
                                todo!()
                            },
                        }
                    }
                }
                Err(err) => error!("{}", err),
            }
        });

        ui.checkbox(&mut self.show_controls, "show controls");
        if self.show_controls {
            if ui.button("refetch controls").clicked() {
                match fetch_controls(&self.camera) {
                    Ok(vec) => self.controls = vec,
                    Err(err) => error!("could not fetch controls {}", err),
                }
            }
            for (description, control) in self.controls.iter_mut() {
                update_ctrl(ui, description, control, &self.camera);
            }
        }
    }
}

fn update_ctrl(
    ui: &mut Ui,
    description: &mut control::Description,
    control: &mut Control,
    cam: &Device,
) {
    let control::Description {
        id,
        typ,
        name,
        minimum,
        maximum,
        step,
        default,
        flags,
        items: _,
    } = description;
    ui.strong(name.clone());
    if !flags.is_empty() {
        ui.label(format!("{}", flags));
    }
    match typ {
        control::Type::Integer => {
            if let control::Value::Integer(mut val) = control.value {
                if *step == 1 {
                    if ui.add(Slider::new(&mut val, *minimum..=*maximum)).changed() {
                        let new = Control {
                            id: *id,
                            value: control::Value::Integer(val),
                        };
                        match set_control(cam, new) {
                            Ok(_) => control.value = control::Value::Integer(val),
                            Err(err) => error!("could not set control {}", err),
                        }
                    }
                } else {
                    egui::ComboBox::from_id_source(name)
                        .selected_text(val.to_string())
                        .show_ui(ui, |ui| {
                            let mut iter_val = *minimum;
                            let step = *step as i64;
                            while iter_val <= *maximum {
                                if ui
                                    .selectable_label(val == iter_val, iter_val.to_string())
                                    .clicked()
                                {
                                    let new = Control {
                                        id: *id,
                                        value: control::Value::Integer(iter_val),
                                    };
                                    match set_control(cam, new) {
                                        Ok(_) => control.value = control::Value::Integer(iter_val),
                                        Err(err) => error!("could not set control {}", err),
                                    }
                                    iter_val += step;
                                }
                            }
                        });
                }
            } else {
                error!(
                    "control description with interger type was: {:?}",
                    control.value
                );
                panic!()
            };
        }
        control::Type::Boolean => {
            if let control::Value::Boolean(mut b) = control.value {
                if ui.checkbox(&mut b, "").clicked() {
                    let new = Control {
                        id: *id,
                        value: control::Value::Boolean(b),
                    };
                    match set_control(cam, new) {
                        Ok(_) => control.value = control::Value::Boolean(b),
                        Err(err) => error!("could not set control {}", err),
                    }
                }
            } else {
                error!(
                    "control description with boolean type was {:?}",
                    control.value
                );
                panic!()
            }
        }
        _ => {
            ui.label(format!("not implemented, because it has type: {}", typ));
        }
    }
}

fn set_control(cam: &Device, ctrl: Control) -> Result<()> {
    *CAMERA_STREAM.lock() = None;
    cam.set_control(ctrl)
}

// fn flag_ui(flags: &mut control::Flags, modify: control::Flags, ui: &mut Ui) -> bool {
//     if ui.checkbox(&mut flags.contains(modify), format!("{}", modify)).clicked() {
//         flags.toggle(modify);
//         true
//     } else {
//         false
//     }
// }

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
