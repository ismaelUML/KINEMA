use super::KinemaGuiApp;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use kinema_ports::{SimulationControl, SnapshotSink};

impl KinemaGuiApp {
    pub(super) fn render_graphs_view(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), ui.available_height().max(120.0)),
            egui::Sense::click_and_drag(),
        );

        let rect = response.rect;
        painter.rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(20, 24, 32),
        );

        let current_time = self.service.current_time();
        let t_max = 10.0f32;

        // Interactive time scrubbing on graph click/drag
        if response.clicked() || response.dragged() {
            if let Some(mouse_pos) = response.interact_pointer_pos() {
                let frac = ((mouse_pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0);
                let scrub_t = (frac * t_max) as f64;
                self.service.seek(scrub_t);
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
        }

        // Draw curves
        let model = self.presenter.model();
        for (i, series) in model.graph_series.iter().enumerate() {
            let curve_color = match i % 3 {
                0 => Color32::from_rgb(64, 200, 255),
                1 => Color32::from_rgb(255, 180, 50),
                _ => Color32::from_rgb(100, 240, 140),
            };

            for window in series.points.windows(2) {
                let (t0, y0) = (window[0].0 as f32, window[0].1 as f32);
                let (t1, y1) = (window[1].0 as f32, window[1].1 as f32);

                let x0 = rect.min.x + (t0 / t_max) * rect.width();
                let x1 = rect.min.x + (t1 / t_max) * rect.width();
                let graph_mid_y = rect.center().y;
                let py0 =
                    graph_mid_y - (y0 * 2.0).clamp(-rect.height() * 0.45, rect.height() * 0.45);
                let py1 =
                    graph_mid_y - (y1 * 2.0).clamp(-rect.height() * 0.45, rect.height() * 0.45);

                painter.line_segment(
                    [Pos2::new(x0, py0), Pos2::new(x1, py1)],
                    Stroke::new(1.5, curve_color),
                );
            }
        }

        // Red cursor line for current simulation time
        let cursor_x = rect.min.x + (current_time as f32 / t_max) * rect.width();
        if cursor_x >= rect.min.x && cursor_x <= rect.max.x {
            painter.line_segment(
                [
                    Pos2::new(cursor_x, rect.min.y),
                    Pos2::new(cursor_x, rect.max.y),
                ],
                Stroke::new(2.0, Color32::RED),
            );
        }
    }
}
