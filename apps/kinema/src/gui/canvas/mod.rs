mod entities;
mod ropes;

use super::KinemaGuiApp;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use kinema_adapter_ui::{GraphKind, UiTheme};
use kinema_domain::EntityKind;

struct PaintContext<'a> {
    painter: &'a egui::Painter,
    rect: Rect,
    origin_x: f32,
    ground_y: f32,
    zoom: f32,
}

#[derive(Clone, Copy)]
struct FrictionParams {
    mass: f64,
    mu_s: f64,
    _mu_k: f64,
    f_app: f64,
}

impl KinemaGuiApp {
    pub(super) fn render_central_workspace(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            let available_height = ui.available_height();
            let canvas_height = (available_height * 0.58).max(200.0);

            ui.heading("SCENE VIEW");
            self.render_interactive_canvas(ui, canvas_height);

            ui.separator();
            ui.horizontal(|ui| {
                ui.heading("GRAPHS");
                ui.selectable_value(&mut self.active_graph, GraphKind::PositionTime, "x-t");
                ui.selectable_value(&mut self.active_graph, GraphKind::VelocityTime, "v-t");
                ui.selectable_value(&mut self.active_graph, GraphKind::AccelerationTime, "a-t");
            });

            self.render_graphs_view(ui);
        });
    }

    fn render_interactive_canvas(&mut self, ui: &mut egui::Ui, height: f32) {
        let (response, painter) =
            ui.allocate_painter(Vec2::new(ui.available_width(), height), egui::Sense::drag());

        let rect = response.rect;
        let bg_color = match self.presenter.theme() {
            UiTheme::Classic => Color32::from_rgb(16, 20, 28),
            UiTheme::Phosphor => Color32::from_rgb(8, 14, 8),
            UiTheme::Amber => Color32::from_rgb(18, 12, 4),
        };
        painter.rect_filled(rect, egui::CornerRadius::ZERO, bg_color);

        handle_canvas_pan_and_zoom(
            &response,
            ui,
            &mut self.canvas_pan_x,
            &mut self.canvas_zoom,
            rect,
        );

        let ground_y = rect.max.y - 45.0;
        let origin_x = rect.min.x + self.canvas_pan_x;

        draw_metric_track(&painter, rect, origin_x, ground_y, self.canvas_zoom);

        // Draw bodies
        let ctx = PaintContext {
            painter: &painter,
            rect,
            origin_x,
            ground_y,
            zoom: self.canvas_zoom,
        };
        self.paint_bodies(&ctx);
        // Draw ropes
        self.paint_ropes(&ctx);
    }

    fn paint_bodies(&self, ctx: &PaintContext<'_>) {
        let model = self.presenter.model();
        for (i, body) in model.bodies.iter().enumerate() {
            self.paint_entity(ctx, i, body);
        }
        self.paint_meeting_markers(ctx);
    }

    fn paint_meeting_markers(&self, ctx: &PaintContext<'_>) {
        let model = self.presenter.model();
        for m in &model.meeting_markers {
            let mx = ctx.origin_x + (m.position as f32 * ctx.zoom);
            if mx >= ctx.rect.min.x && mx <= ctx.rect.max.x {
                ctx.painter.text(
                    Pos2::new(mx, ctx.ground_y - 40.0),
                    egui::Align2::CENTER_CENTER,
                    "X (MEET)",
                    egui::FontId::monospace(12.0),
                    Color32::from_rgb(255, 60, 60),
                );
            }
        }
    }

    fn paint_entity(
        &self,
        ctx: &PaintContext<'_>,
        index: usize,
        body: &kinema_adapter_ui::UiBodyView,
    ) {
        match &body.kind {
            EntityKind::Vehicle { lane } => {
                self.paint_vehicle(ctx, index, body, *lane);
            }
            EntityKind::FreeFall { initial_height } => {
                self.paint_free_fall(ctx, body, *initial_height);
            }
            EntityKind::VerticalProjectile { v0 } => {
                self.paint_vertical_projectile(ctx, body, *v0);
            }
            EntityKind::FeatherAndHammer { is_feather } => {
                self.paint_lunar_drop(ctx, body, *is_feather);
            }
            EntityKind::FrictionBlock {
                mass,
                mu_s,
                mu_k,
                f_app,
            } => {
                let params = FrictionParams {
                    mass: *mass,
                    mu_s: *mu_s,
                    _mu_k: *mu_k,
                    f_app: *f_app,
                };
                self.paint_friction_crate(ctx, body, params);
            }
            EntityKind::InclineBlock {
                angle_rad,
                incline_length,
            } => {
                self.paint_incline_plane(ctx, body, *angle_rad, *incline_length);
            }
            EntityKind::AtwoodSystem { m1, m2 } => {
                self.paint_atwood_machine(ctx, body, *m1, *m2);
            }
            EntityKind::TablePulleySystem { m1, m2 } => {
                self.paint_table_pulley(ctx, body, *m1, *m2);
            }
            EntityKind::Generic => {
                self.paint_vehicle(ctx, index, body, 0);
            }
        }
    }
}

fn parse_hex_color(hex: &str) -> Color32 {
    let clean = hex.trim_start_matches('#');
    if clean.len() != 6 {
        return Color32::WHITE;
    }
    let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255);
    Color32::from_rgb(r, g, b)
}

fn handle_canvas_pan_and_zoom(
    response: &egui::Response,
    ui: &egui::Ui,
    pan_x: &mut f32,
    zoom: &mut f32,
    rect: Rect,
) {
    if response.dragged() {
        *pan_x += response.drag_delta().x;
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.1 {
            let mult = (scroll * 0.002).exp();
            let mouse_x = ui
                .input(|i| i.pointer.hover_pos())
                .map(|p| p.x)
                .unwrap_or(rect.center().x);
            let rel_x = mouse_x - (rect.min.x + *pan_x);
            *pan_x -= rel_x * (mult - 1.0);
            *zoom = (*zoom * mult).clamp(0.5, 40.0);
        }
    }
}

fn draw_metric_track(
    painter: &egui::Painter,
    rect: Rect,
    origin_x: f32,
    ground_y: f32,
    zoom: f32,
) {
    let track_color = Color32::from_rgb(80, 95, 115);
    painter.line_segment(
        [
            Pos2::new(rect.min.x, ground_y),
            Pos2::new(rect.max.x, ground_y),
        ],
        Stroke::new(2.0, track_color),
    );

    let tick_spacing = 25.0 * zoom;
    for i in -20..60 {
        let tx = origin_x + (i as f32 * tick_spacing);
        if tx >= rect.min.x && tx <= rect.max.x {
            painter.line_segment(
                [Pos2::new(tx, ground_y - 6.0), Pos2::new(tx, ground_y + 6.0)],
                Stroke::new(1.0, Color32::from_rgb(90, 105, 130)),
            );
            painter.text(
                Pos2::new(tx, ground_y + 12.0),
                egui::Align2::CENTER_TOP,
                format!("{}m", i * 25),
                egui::FontId::monospace(10.0),
                Color32::from_rgb(160, 175, 195),
            );
        }
    }
}
