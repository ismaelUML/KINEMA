use super::KinemaGuiApp;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use kinema_adapter_ui::{UiTheme, UiViewModel};
use kinema_ports::{SimulationControl, SnapshotSink};

impl KinemaGuiApp {
    pub(super) fn render_graphs_view(&mut self, ui: &mut egui::Ui) {
        let (response, painter) = ui.allocate_painter(
            Vec2::new(ui.available_width(), ui.available_height().max(120.0)),
            egui::Sense::click_and_drag(),
        );

        let rect = response.rect;
        let theme = self.presenter.theme();
        let t_max = (self.presenter.model().duration_hint.max(10.0)) as f32;

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

        let model = self.presenter.model();
        let (y_min, y_max) = compute_y_bounds(model);

        draw_graph_background_and_grid(&painter, rect, t_max, y_min, y_max, theme);
        draw_graph_series(&painter, rect, model, t_max, y_min, y_max);
        draw_graph_event_markers(&painter, rect, model, t_max, y_min, y_max);
        draw_graph_time_cursor(&painter, rect, self.service.current_time() as f32, t_max);
        draw_graph_legend(&painter, rect, model);
    }
}

fn compute_y_bounds(model: &UiViewModel) -> (f32, f32) {
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for series in &model.graph_series {
        for (_, y) in &series.points {
            let y = *y as f32;
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }
    }
    if min_y >= max_y {
        (-10.0, 10.0)
    } else {
        let pad = ((max_y - min_y) * 0.12).max(1.0);
        (min_y - pad, max_y + pad)
    }
}

fn point_to_screen(rect: Rect, t: f32, y: f32, t_max: f32, y_min: f32, y_max: f32) -> Pos2 {
    let x = rect.min.x + (t / t_max).clamp(0.0, 1.0) * rect.width();
    let norm_y = ((y - y_min) / (y_max - y_min).max(0.001)).clamp(0.0, 1.0);
    let screen_y = rect.max.y - norm_y * rect.height();
    Pos2::new(x, screen_y)
}

fn draw_graph_background_and_grid(
    painter: &egui::Painter,
    rect: Rect,
    t_max: f32,
    y_min: f32,
    y_max: f32,
    theme: UiTheme,
) {
    let bg_color = match theme {
        UiTheme::Telemetry => Color32::from_rgb(11, 15, 23),
        UiTheme::Phosphor => Color32::from_rgb(6, 12, 6),
        UiTheme::Amber => Color32::from_rgb(14, 10, 4),
        UiTheme::Classic => Color32::from_rgb(18, 22, 30),
    };
    painter.rect_filled(rect, egui::CornerRadius::same(4), bg_color);
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(40, 55, 80, 80)),
        egui::StrokeKind::Inside,
    );

    // Horizontal grid & zero baseline
    let grid_color = Color32::from_rgba_unmultiplied(50, 70, 95, 50);
    for i in 1..4 {
        let frac = i as f32 / 4.0;
        let gy = rect.min.y + frac * rect.height();
        painter.line_segment(
            [Pos2::new(rect.min.x, gy), Pos2::new(rect.max.x, gy)],
            Stroke::new(1.0, grid_color),
        );
        let y_val = y_max - frac * (y_max - y_min);
        painter.text(
            Pos2::new(rect.min.x + 6.0, gy),
            egui::Align2::LEFT_CENTER,
            format!("{:.1}", y_val),
            egui::FontId::monospace(8.5),
            Color32::from_rgba_unmultiplied(120, 145, 175, 160),
        );
    }

    // Vertical time grid ticks
    for i in 1..5 {
        let frac = i as f32 / 5.0;
        let gx = rect.min.x + frac * rect.width();
        painter.line_segment(
            [Pos2::new(gx, rect.min.y), Pos2::new(gx, rect.max.y)],
            Stroke::new(1.0, grid_color),
        );
        painter.text(
            Pos2::new(gx, rect.max.y - 4.0),
            egui::Align2::CENTER_BOTTOM,
            format!("{:.1}s", frac * t_max),
            egui::FontId::monospace(8.5),
            Color32::from_rgba_unmultiplied(120, 145, 175, 160),
        );
    }
}

fn draw_graph_series(
    painter: &egui::Painter,
    rect: Rect,
    model: &UiViewModel,
    t_max: f32,
    y_min: f32,
    y_max: f32,
) {
    for (i, series) in model.graph_series.iter().enumerate() {
        let curve_color = match i % 3 {
            0 => Color32::from_rgb(0, 229, 255),  // Laser cyan
            1 => Color32::from_rgb(255, 176, 32), // High-vis amber
            _ => Color32::from_rgb(255, 61, 203), // Telemetry magenta
        };
        let glow_color = Color32::from_rgba_unmultiplied(
            curve_color.r(),
            curve_color.g(),
            curve_color.b(),
            60,
        );

        for window in series.points.windows(2) {
            let p0 = point_to_screen(
                rect,
                window[0].0 as f32,
                window[0].1 as f32,
                t_max,
                y_min,
                y_max,
            );
            let p1 = point_to_screen(
                rect,
                window[1].0 as f32,
                window[1].1 as f32,
                t_max,
                y_min,
                y_max,
            );

            // Under-glow + sharp core
            painter.line_segment([p0, p1], Stroke::new(4.0, glow_color));
            painter.line_segment([p0, p1], Stroke::new(1.5, curve_color));
        }
    }
}

fn draw_graph_event_markers(
    painter: &egui::Painter,
    rect: Rect,
    model: &UiViewModel,
    t_max: f32,
    y_min: f32,
    y_max: f32,
) {
    for m in &model.meeting_markers {
        let t = m.time as f32;
        if t >= 0.0 && t <= t_max {
            let px = rect.min.x + (t / t_max) * rect.width();
            painter.line_segment(
                [Pos2::new(px, rect.min.y), Pos2::new(px, rect.max.y)],
                Stroke::new(1.0, Color32::from_rgb(255, 61, 203)),
            );

            let pt = point_to_screen(rect, t, m.position as f32, t_max, y_min, y_max);
            painter.circle_filled(pt, 4.0, Color32::from_rgb(255, 61, 203));
            painter.circle_stroke(pt, 5.5, Stroke::new(1.0, Color32::WHITE));
            painter.text(
                Pos2::new(px + 4.0, rect.min.y + 12.0),
                egui::Align2::LEFT_CENTER,
                format!("MEET @ {:.2}s", t),
                egui::FontId::monospace(8.5),
                Color32::from_rgb(255, 120, 220),
            );
        }
    }
}

fn draw_graph_time_cursor(painter: &egui::Painter, rect: Rect, current_time: f32, t_max: f32) {
    let cursor_x = rect.min.x + (current_time / t_max).clamp(0.0, 1.0) * rect.width();
    let cursor_color = Color32::from_rgb(0, 229, 255);

    painter.line_segment(
        [
            Pos2::new(cursor_x, rect.min.y),
            Pos2::new(cursor_x, rect.max.y),
        ],
        Stroke::new(1.5, cursor_color),
    );

    // Floating cursor badge
    let badge_rect = Rect::from_center_size(Pos2::new(cursor_x, rect.min.y + 9.0), Vec2::new(56.0, 14.0));
    painter.rect_filled(
        badge_rect,
        egui::CornerRadius::same(7),
        Color32::from_rgba_unmultiplied(10, 15, 24, 230),
    );
    painter.rect_stroke(
        badge_rect,
        egui::CornerRadius::same(7),
        Stroke::new(1.0, cursor_color),
        egui::StrokeKind::Inside,
    );
    painter.text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.2}s", current_time),
        egui::FontId::monospace(9.0),
        Color32::from_rgb(230, 245, 255),
    );
}

fn draw_graph_legend(painter: &egui::Painter, rect: Rect, model: &UiViewModel) {
    if model.graph_series.is_empty() {
        return;
    }
    let mut offset_x = rect.max.x - 12.0;
    for (i, series) in model.graph_series.iter().enumerate().rev() {
        let curve_color = match i % 3 {
            0 => Color32::from_rgb(0, 229, 255),
            1 => Color32::from_rgb(255, 176, 32),
            _ => Color32::from_rgb(255, 61, 203),
        };
        let label = &series.body_name;
        let text_w = label.len() as f32 * 6.0 + 16.0;
        let pill_rect = Rect::from_min_size(
            Pos2::new(offset_x - text_w, rect.min.y + 6.0),
            Vec2::new(text_w, 15.0),
        );
        painter.rect_filled(
            pill_rect,
            egui::CornerRadius::same(4),
            Color32::from_rgba_unmultiplied(15, 20, 30, 200),
        );
        painter.circle_filled(
            Pos2::new(pill_rect.min.x + 6.0, pill_rect.center().y),
            3.0,
            curve_color,
        );
        painter.text(
            Pos2::new(pill_rect.min.x + 13.0, pill_rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::monospace(8.5),
            Color32::from_rgb(210, 225, 245),
        );
        offset_x -= text_w + 6.0;
    }
}
