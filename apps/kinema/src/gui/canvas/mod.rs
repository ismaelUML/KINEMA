mod entities;
pub mod primitives;
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
        let theme = self.presenter.theme();
        let bg_color = match theme {
            UiTheme::Telemetry => Color32::from_rgb(10, 13, 20),
            UiTheme::Classic => Color32::from_rgb(16, 20, 28),
            UiTheme::Phosphor => Color32::from_rgb(8, 14, 8),
            UiTheme::Amber => Color32::from_rgb(18, 12, 4),
        };
        painter.rect_filled(rect, egui::CornerRadius::ZERO, bg_color);

        handle_canvas_pan_and_zoom(
            &response,
            ui,
            &mut self.canvas_pan_x,
            &mut self.canvas_pan_y,
            &mut self.canvas_zoom,
            rect,
        );

        let ground_y = rect.max.y - 45.0 + self.canvas_pan_y;
        let origin_x = rect.min.x + self.canvas_pan_x;

        if theme == UiTheme::Telemetry {
            draw_telemetry_optical_grid(&painter, rect, origin_x, ground_y, self.canvas_zoom);
        }
        draw_metric_track(&painter, rect, origin_x, ground_y, self.canvas_zoom, theme);

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
            if mx >= ctx.rect.min.x - 40.0 && mx <= ctx.rect.max.x + 40.0 {
                // Dashed vertical leader line
                let leader_top = ctx.ground_y - 48.0;
                let leader_bottom = ctx.ground_y;
                let mut curr_y = leader_top;
                while curr_y < leader_bottom {
                    let next_y = (curr_y + 4.0).min(leader_bottom);
                    ctx.painter.line_segment(
                        [Pos2::new(mx, curr_y), Pos2::new(mx, next_y)],
                        Stroke::new(1.0, Color32::from_rgb(255, 61, 203)),
                    );
                    curr_y += 8.0;
                }

                // Glowing diamond pin
                let d_size = 6.0f32;
                let d_center = Pos2::new(mx, leader_top);
                let p1 = Pos2::new(d_center.x, d_center.y - d_size);
                let p2 = Pos2::new(d_center.x + d_size, d_center.y);
                let p3 = Pos2::new(d_center.x, d_center.y + d_size);
                let p4 = Pos2::new(d_center.x - d_size, d_center.y);
                ctx.painter.add(egui::Shape::convex_polygon(
                    vec![p1, p2, p3, p4],
                    Color32::from_rgb(255, 61, 203),
                    Stroke::new(1.0, Color32::WHITE),
                ));

                // Value readout badge
                ctx.painter.text(
                    Pos2::new(mx, leader_top - 8.0),
                    egui::Align2::CENTER_BOTTOM,
                    format!("MEET {:.1}m ({:.1}s)", m.position, m.time),
                    egui::FontId::monospace(9.5),
                    Color32::from_rgb(255, 120, 220),
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
    pan_y: &mut f32,
    zoom: &mut f32,
    rect: Rect,
) {
    if response.dragged() {
        *pan_x += response.drag_delta().x;
        *pan_y += response.drag_delta().y;
    }
    apply_canvas_scroll(ui, pan_x, pan_y, zoom, rect, response.hovered());
}

fn apply_canvas_scroll(
    ui: &egui::Ui,
    pan_x: &mut f32,
    pan_y: &mut f32,
    zoom: &mut f32,
    rect: Rect,
    is_hovered: bool,
) {
    if !is_hovered {
        return;
    }
    let scroll = ui.input(|i| i.smooth_scroll_delta.y);
    if scroll.abs() <= 0.1 {
        return;
    }
    let mult = (scroll * 0.002).exp();
    let mouse = ui
        .input(|i| i.pointer.hover_pos())
        .unwrap_or_else(|| rect.center());
    let rel_x = mouse.x - (rect.min.x + *pan_x);
    let rel_y = mouse.y - (rect.max.y - 45.0 + *pan_y);
    *pan_x -= rel_x * (mult - 1.0);
    *pan_y -= rel_y * (mult - 1.0);
    *zoom = (*zoom * mult).clamp(0.5, 40.0);
}

fn draw_telemetry_optical_grid(
    painter: &egui::Painter,
    rect: Rect,
    origin_x: f32,
    ground_y: f32,
    zoom: f32,
) {
    // Horizontal height grid lines
    let y_step = 25.0 * zoom;
    if y_step > 15.0 {
        let mut y = ground_y - y_step;
        while y > rect.min.y {
            painter.line_segment(
                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                Stroke::new(0.5, Color32::from_rgba_unmultiplied(35, 48, 68, 50)),
            );
            y -= y_step;
        }
    }

    // Origin Reticle (0, 0)
    let reticle_color = Color32::from_rgb(0, 229, 255);
    painter.circle_stroke(
        Pos2::new(origin_x, ground_y),
        4.0,
        Stroke::new(1.0, reticle_color),
    );
}

fn draw_metric_track(
    painter: &egui::Painter,
    rect: Rect,
    origin_x: f32,
    ground_y: f32,
    zoom: f32,
    theme: UiTheme,
) {
    let (track_color, tick_color, label_color) = match theme {
        UiTheme::Telemetry => (
            Color32::from_rgb(32, 45, 65),
            Color32::from_rgb(0, 229, 255),
            Color32::from_rgb(140, 165, 195),
        ),
        UiTheme::Phosphor => (
            Color32::from_rgb(20, 60, 20),
            Color32::from_rgb(0, 255, 65),
            Color32::from_rgb(0, 200, 50),
        ),
        UiTheme::Amber => (
            Color32::from_rgb(60, 40, 10),
            Color32::from_rgb(255, 176, 0),
            Color32::from_rgb(220, 150, 0),
        ),
        UiTheme::Classic => (
            Color32::from_rgb(80, 95, 115),
            Color32::from_rgb(90, 105, 130),
            Color32::from_rgb(160, 175, 195),
        ),
    };

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
                Stroke::new(1.0, tick_color),
            );
            painter.text(
                Pos2::new(tx, ground_y + 12.0),
                egui::Align2::CENTER_TOP,
                format!("{}m", i * 25),
                egui::FontId::monospace(10.0),
                label_color,
            );
        }
    }
}
