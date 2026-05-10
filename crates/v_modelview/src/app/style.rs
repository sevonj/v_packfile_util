use egui::Color32;
use egui::CornerRadius;
use egui::Frame;
use egui::Margin;
use egui::Shadow;
use egui::Stroke;

pub const OSD_BG_FILL: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 127);
#[allow(dead_code)]
pub const OSD_FRAME: Frame = Frame {
    inner_margin: Margin::same(4),
    fill: OSD_BG_FILL,
    stroke: Stroke::NONE,
    corner_radius: CornerRadius::same(6),
    outer_margin: Margin::ZERO,
    shadow: Shadow::NONE,
};
pub const OSD_PANEL_FRAME: Frame = Frame {
    inner_margin: Margin::same(8),
    fill: OSD_BG_FILL,
    stroke: Stroke::NONE,
    corner_radius: CornerRadius::ZERO,
    outer_margin: Margin::ZERO,
    shadow: Shadow::NONE,
};
