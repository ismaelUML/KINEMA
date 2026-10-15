use super::KinemaGuiApp;
use eframe::egui;
use kinema_adapter_storage::PngCanvasExporter;
use kinema_adapter_ui::HelpTopic;
use kinema_ports::{CancellationToken, ImageExporter};

impl KinemaGuiApp {
    pub(super) fn render_dialogs(&mut self, ctx: &egui::Context) {
        if self.help_open {
            egui::Window::new("KINEMA Help Contents (F1)")
                .open(&mut self.help_open)
                .show(ctx, |ui| {
                    ui.monospace(HelpTopic::Contents.content());
                });
        }
        if self.eq_ref_open {
            egui::Window::new("Equation Reference")
                .open(&mut self.eq_ref_open)
                .show(ctx, |ui| {
                    ui.monospace(HelpTopic::EquationReference.content());
                });
        }
        if self.about_open {
            egui::Window::new("About KINEMA")
                .open(&mut self.about_open)
                .show(ctx, |ui| {
                    ui.monospace(HelpTopic::About.content());
                });
        }
    }

    pub(super) fn export_current_canvas(&mut self) {
        let exporter = PngCanvasExporter::new(640, 480);
        let token = CancellationToken::new();
        let filename = "kinema_canvas_export.png";

        match exporter.export_png(
            self.service.scene(),
            self.service.current_time(),
            filename,
            &token,
        ) {
            Ok(()) => self.status_text = format!("Canvas exported as '{}'", filename),
            Err(e) => self.status_text = format!("Export failed: {}", e),
        }
    }
}
