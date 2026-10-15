use super::KinemaGuiApp;
use eframe::egui::{self, Color32};
use kinema_adapter_ui::UiTheme;

impl KinemaGuiApp {
    pub(super) fn apply_theme_styling(&self, ctx: &egui::Context) {
        let is_telemetry = self.presenter.theme() == UiTheme::Telemetry;
        let mut visuals = match self.presenter.theme() {
            UiTheme::Telemetry => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = Color32::from_rgb(17, 21, 30);
                v.window_fill = Color32::from_rgb(23, 28, 39);
                v.faint_bg_color = Color32::from_rgb(14, 18, 25);
                v.extreme_bg_color = Color32::from_rgb(10, 13, 20);
                v.widgets.noninteractive.bg_fill = Color32::from_rgb(23, 28, 39);
                v.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(230, 237, 247);
                v.widgets.inactive.bg_fill = Color32::from_rgb(28, 35, 48);
                v.widgets.inactive.fg_stroke.color = Color32::from_rgb(180, 195, 215);
                v.widgets.hovered.bg_fill = Color32::from_rgb(38, 48, 68);
                v.widgets.hovered.fg_stroke.color = Color32::from_rgb(0, 229, 255);
                v.widgets.active.bg_fill = Color32::from_rgb(20, 60, 80);
                v.widgets.active.fg_stroke.color = Color32::from_rgb(0, 229, 255);
                v.selection.bg_fill = Color32::from_rgba_unmultiplied(0, 229, 255, 60);
                v.selection.stroke = egui::Stroke::new(1.0, Color32::from_rgb(0, 229, 255));
                v
            }
            UiTheme::Classic => {
                let mut v = egui::Visuals::light();
                v.panel_fill = Color32::from_rgb(192, 192, 192);
                v.window_fill = Color32::from_rgb(192, 192, 192);
                v.widgets.noninteractive.bg_fill = Color32::from_rgb(192, 192, 192);
                v.widgets.inactive.bg_fill = Color32::from_rgb(212, 212, 212);
                v.widgets.hovered.bg_fill = Color32::from_rgb(230, 230, 230);
                v.widgets.active.bg_fill = Color32::from_rgb(170, 170, 170);
                v
            }
            UiTheme::Phosphor => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = Color32::from_rgb(10, 14, 10);
                v.window_fill = Color32::from_rgb(10, 14, 10);
                v.override_text_color = Some(Color32::from_rgb(51, 255, 51));
                v
            }
            UiTheme::Amber => {
                let mut v = egui::Visuals::dark();
                v.panel_fill = Color32::from_rgb(18, 13, 4);
                v.window_fill = Color32::from_rgb(18, 13, 4);
                v.override_text_color = Some(Color32::from_rgb(255, 176, 0));
                v
            }
        };

        if is_telemetry {
            visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(5);
            visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(5);
            visuals.widgets.active.corner_radius = egui::CornerRadius::same(5);
            visuals.window_corner_radius = egui::CornerRadius::same(8);
        } else {
            visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
            visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
            visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
            visuals.window_corner_radius = egui::CornerRadius::ZERO;
        }

        ctx.set_visuals(visuals);
    }
}
