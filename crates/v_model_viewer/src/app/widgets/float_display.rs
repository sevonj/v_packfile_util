use egui::Widget;

pub struct FloatDisplay {
    value: f32,
}

impl FloatDisplay {
    pub fn new(value: f32) -> Self {
        Self { value }
    }
}

impl Widget for FloatDisplay {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        ui.label(format!("{:.3}", self.value))
            .on_hover_text(self.value.to_string())
    }
}
