use super::color::GOLD;
use egui::{CornerRadius, Stroke};

pub fn setup(ctx: &egui::Context) {
    ctx.style_mut(|style| {
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, GOLD);

        style.visuals.window_corner_radius = CornerRadius::ZERO;
        style.visuals.widgets.active.corner_radius = CornerRadius::ZERO;
        style.visuals.widgets.inactive.corner_radius = CornerRadius::ZERO;
        style.visuals.widgets.hovered.corner_radius = CornerRadius::ZERO;
    })
}
