use super::KinemaGuiApp;
use eframe::egui::{self, Color32};
use kinema_adapter_ui::UiTheme;

impl KinemaGuiApp {
    pub(super) fn apply_theme_styling(&self, ctx: &egui::Context) {
        let mut visuals = match self.presenter.theme() {
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
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
        visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
        visuals.window_corner_radius = egui::CornerRadius::ZERO;
        ctx.set_visuals(visuals);
    }
}
