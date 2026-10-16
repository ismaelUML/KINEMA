mod entities;
pub mod primitives;
mod ropes;

use super::KinemaGuiApp;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use kinema_adapter_ui::{GraphKind, UiBodyView, UiTheme};
use kinema_domain::EntityKind;
use kinema_ports::{SceneEditing, SnapshotSink};

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

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                self.selected_body_id = check_entity_click(
                    pos,
                    &self.presenter.model().bodies,
                    origin_x,
                    ground_y,
                    self.canvas_zoom,
                );
            }
        }

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
        // Draw interactive gizmos for selected entity
        self.render_active_gizmos(ui, &painter, origin_x, ground_y);
        // Draw guided challenge HUD overlay if a challenge is active
        self.render_challenge_hud_overlay(ui, rect);
    }

    fn render_active_gizmos(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        origin_x: f32,
        ground_y: f32,
    ) {
        let Some(sel_id) = self.selected_body_id.clone() else {
            return;
        };
        let bodies = self.presenter.model().bodies.clone();
        let Some(body) = bodies.iter().find(|b| b.id == sel_id) else {
            return;
        };

        let lane_offset = match body.kind {
            EntityKind::Vehicle { lane } => lane as f32 * 32.0,
            _ => 0.0,
        };
        let px = origin_x + (body.current_position as f32 * self.canvas_zoom);
        let py = ground_y - 22.0 - lane_offset;

        self.interact_position_gizmo(ui, painter, body, px, ground_y);
        self.interact_velocity_gizmo(ui, painter, body, px, py);
    }

    fn interact_position_gizmo(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        body: &UiBodyView,
        px: f32,
        ground_y: f32,
    ) {
        let center = Pos2::new(px + 21.0, ground_y + 20.0);
        let rect = Rect::from_center_size(center, Vec2::new(56.0, 18.0));
        let id = ui.make_persistent_id("body_gizmo_pos");
        let resp = ui.interact(rect, id, egui::Sense::drag());

        if resp.dragged() {
            let delta_m = (resp.drag_delta().x / self.canvas_zoom) as f64;
            let param_name = if body.parameters.iter().any(|(k, _)| *k == "s0") {
                "s0"
            } else if body.parameters.iter().any(|(k, _)| *k == "y0") {
                "y0"
            } else {
                "x0"
            };
            let curr = body
                .parameters
                .iter()
                .find(|(k, _)| *k == param_name)
                .map(|(_, v)| *v)
                .unwrap_or(body.current_position);
            let new_val = curr + delta_m;
            let _ = self.service.edit_parameter(&body.id, param_name, new_val);
            self.presenter
                .consume_snapshot(self.service.scene(), self.service.current_time());
            self.sync_active_challenge();
            self.status_text =
                format!("Moved {} to {} = {:.2}m", body.name, param_name, new_val);
        }
        primitives::draw_position_gizmo(painter, center, resp.hovered(), resp.dragged());
    }

    fn interact_velocity_gizmo(
        &mut self,
        ui: &mut egui::Ui,
        painter: &egui::Painter,
        body: &UiBodyView,
        px: f32,
        py: f32,
    ) {
        let v_len = (body.current_velocity as f32 * 2.5).clamp(-65.0, 65.0);
        let center = Pos2::new(px + 21.0 + v_len, py + 12.0);
        let rect = Rect::from_center_size(center, Vec2::new(16.0, 16.0));
        let id = ui.make_persistent_id("body_gizmo_vel");
        let resp = ui.interact(rect, id, egui::Sense::drag());

        if resp.dragged() {
            let delta_v = (resp.drag_delta().x / 2.5) as f64;
            let param_name = if body.parameters.iter().any(|(k, _)| *k == "v0") {
                "v0"
            } else {
                "v"
            };
            let curr = body
                .parameters
                .iter()
                .find(|(k, _)| *k == param_name)
                .map(|(_, v)| *v)
                .unwrap_or(body.current_velocity);
            let new_val = curr + delta_v;
            let _ = self.service.edit_parameter(&body.id, param_name, new_val);
            self.presenter
                .consume_snapshot(self.service.scene(), self.service.current_time());
            self.sync_active_challenge();
            self.status_text = format!(
                "Adjusted {} to {} = {:.2}m/s",
                body.name, param_name, new_val
            );
        }
        primitives::draw_velocity_gizmo(painter, center, resp.hovered(), resp.dragged());
    }

    fn render_challenge_hud_overlay(&mut self, ui: &mut egui::Ui, canvas_rect: egui::Rect) {
        let Some(ref ch) = self.presenter.model().active_challenge_view else {
            return;
        };
        let hud_width = 390.0;
        let hud_rect = egui::Rect::from_min_size(
            egui::pos2(canvas_rect.min.x + 14.0, canvas_rect.min.y + 14.0),
            egui::vec2(hud_width, 130.0),
        );
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(hud_rect));
        let border_color = if ch.is_solved {
            egui::Color32::from_rgb(80, 240, 120)
        } else {
            egui::Color32::from_rgb(45, 140, 240)
        };
        egui::Frame::window(child_ui.style())
            .fill(egui::Color32::from_rgba_premultiplied(12, 16, 24, 235))
            .stroke(egui::Stroke::new(1.5, border_color))
            .corner_radius(egui::CornerRadius::same(6))
            .show(&mut child_ui, |ui| {
                render_challenge_hud_content(ui, ch);
            });
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

fn check_entity_click(
    pointer_pos: Pos2,
    bodies: &[UiBodyView],
    origin_x: f32,
    ground_y: f32,
    zoom: f32,
) -> Option<String> {
    for b in bodies {
        let lane_offset = match b.kind {
            EntityKind::Vehicle { lane } => lane as f32 * 32.0,
            _ => 0.0,
        };
        let px = origin_x + (b.current_position as f32 * zoom);
        let py = ground_y - 22.0 - lane_offset;
        let bounds = Rect::from_center_size(Pos2::new(px + 21.0, py), Vec2::new(60.0, 40.0));
        if bounds.contains(pointer_pos) {
            return Some(b.id.clone());
        }
    }
    None
}

fn render_challenge_hud_content(ui: &mut egui::Ui, ch: &kinema_adapter_ui::UiChallengeView) {
    ui.horizontal(|ui| {
        let title_badge = if ch.is_solved {
            "🏆 ¡DESAFÍO COMPLETADO!"
        } else {
            "🎯 DESAFÍO ACTIVO"
        };
        let color = if ch.is_solved {
            egui::Color32::from_rgb(80, 240, 120)
        } else {
            egui::Color32::from_rgb(70, 180, 255)
        };
        ui.label(egui::RichText::new(title_badge).color(color).strong());
        if ch.is_solved {
            ui.label(egui::RichText::new("⭐⭐⭐").color(egui::Color32::from_rgb(255, 215, 0)));
        }
    });
    ui.label(egui::RichText::new(&ch.title).strong().size(13.0));
    ui.label(egui::RichText::new(&ch.objective).small());
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Estado:").small().strong());
        ui.label(egui::RichText::new(&ch.status_text).small());
    });
    let progress = if ch.is_solved { 1.0 } else { ch.progress_ratio };
    ui.add(egui::ProgressBar::new(progress).show_percentage());
}

