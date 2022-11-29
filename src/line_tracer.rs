use egui::{DragValue, Ui};

pub struct LineTracer {
    lines_to_trace: Vec<f32>,
    references: Vec<f32>,
    seconds_from_start: f32,
    start_inst: Option<std::time::Instant>,
    abs_values: Vec<Vec<f32>>,
}

impl LineTracer {
    pub fn main(&mut self, ui: &mut Ui) {}

    pub fn side_panel(&mut self, ui: &mut Ui) {
        ui.label("trace wavelengths");
        for val in &mut self.lines_to_trace {
            ui.add(DragValue::new(val));
        }
        if ui.button("add new wavelength").clicked() {
            self.lines_to_trace.push(500.0)
        }
    }
}

impl Default for LineTracer {
    fn default() -> Self {
        Self {
            lines_to_trace: vec![500.0],
            references: Default::default(),
            seconds_from_start: Default::default(),
            start_inst: Default::default(),
            abs_values: Default::default(),
        }
    }
}
