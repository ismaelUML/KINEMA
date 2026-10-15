use super::{FrictionParams, PaintContext};
use crate::gui::KinemaGuiApp;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

impl KinemaGuiApp {
    pub(super) fn paint_vehicle(
        &self,
        ctx: &PaintContext<'_>,
        index: usize,
        body: &kinema_adapter_ui::UiBodyView,
        lane: usize,
    ) {
        let lane_offset_y = lane as f32 * 32.0;
        let car_base_y = ctx.ground_y - lane_offset_y;
        let px = ctx.origin_x + (body.current_position as f32 * ctx.zoom);
        let py = car_base_y - 22.0;

        if px < ctx.rect.min.x - 60.0 || px > ctx.rect.max.x + 60.0 {
            return;
        }

        if lane > 0 {
            let dy = ctx.ground_y - lane_offset_y + 8.0;
            ctx.painter.line_segment(
                [Pos2::new(ctx.rect.min.x, dy), Pos2::new(ctx.rect.max.x, dy)],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(180, 190, 200, 60)),
            );
        }

        let cart_color = match index % 3 {
            0 => Color32::from_rgb(64, 200, 255),
            1 => Color32::from_rgb(255, 180, 50),
            _ => Color32::from_rgb(100, 240, 140),
        };

        let chassis_rect = Rect::from_min_size(Pos2::new(px, py + 6.0), Vec2::new(42.0, 14.0));
        ctx.painter.rect(
            chassis_rect,
            egui::CornerRadius::same(3),
            cart_color,
            Stroke::new(1.5, Color32::BLACK),
            egui::StrokeKind::Inside,
        );

        let cabin_rect = Rect::from_min_size(Pos2::new(px + 10.0, py), Vec2::new(20.0, 8.0));
        ctx.painter.rect(
            cabin_rect,
            egui::CornerRadius::same(2),
            Color32::from_rgb(30, 45, 60),
            Stroke::new(1.0, Color32::BLACK),
            egui::StrokeKind::Inside,
        );

        self.paint_wheel(ctx.painter, px + 8.0, car_base_y - 2.0);
        self.paint_wheel(ctx.painter, px + 34.0, car_base_y - 2.0);

        ctx.painter.text(
            Pos2::new(px + 21.0, py - 6.0),
            egui::Align2::CENTER_BOTTOM,
            &body.name,
            egui::FontId::monospace(11.0),
            Color32::WHITE,
        );

        if body.current_velocity.abs() > 0.05 {
            let arrow_len = (body.current_velocity as f32 * 2.0).clamp(-50.0, 50.0);
            let p_start = Pos2::new(px + 21.0, py + 12.0);
            let p_end = Pos2::new(px + 21.0 + arrow_len, py + 12.0);
            ctx.painter
                .line_segment([p_start, p_end], Stroke::new(2.0, Color32::YELLOW));
        }
    }

    pub(super) fn paint_wheel(&self, painter: &egui::Painter, cx: f32, cy: f32) {
        painter.circle_filled(Pos2::new(cx, cy), 5.0, Color32::BLACK);
        painter.circle_filled(Pos2::new(cx, cy), 2.5, Color32::LIGHT_GRAY);
    }

    pub(super) fn paint_free_fall(
        &self,
        ctx: &PaintContext<'_>,
        body: &kinema_adapter_ui::UiBodyView,
        initial_height: f64,
    ) {
        let tower_x = (ctx.origin_x + 120.0).clamp(ctx.rect.min.x + 80.0, ctx.rect.max.x - 80.0);
        let max_h_pixels = (ctx.rect.height() - 80.0).max(120.0);
        let h_scale = (max_h_pixels / initial_height.max(1.0) as f32).min(18.0);

        let tower_top_y = ctx.ground_y - (initial_height as f32 * h_scale);
        ctx.painter.line_segment(
            [
                Pos2::new(tower_x - 15.0, ctx.ground_y),
                Pos2::new(tower_x - 15.0, tower_top_y - 10.0),
            ],
            Stroke::new(3.0, Color32::from_rgb(70, 85, 105)),
        );

        for h in (0..=(initial_height as i32)).step_by(5) {
            let ty = ctx.ground_y - (h as f32 * h_scale);
            ctx.painter.line_segment(
                [Pos2::new(tower_x - 20.0, ty), Pos2::new(tower_x - 10.0, ty)],
                Stroke::new(1.0, Color32::from_rgb(140, 160, 185)),
            );
            ctx.painter.text(
                Pos2::new(tower_x - 24.0, ty),
                egui::Align2::RIGHT_CENTER,
                format!("{}m", h),
                egui::FontId::monospace(9.0),
                Color32::from_rgb(180, 200, 220),
            );
        }

        ctx.painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(tower_x - 18.0, tower_top_y - 12.0),
                Vec2::new(30.0, 8.0),
            ),
            egui::CornerRadius::same(2),
            Color32::from_rgb(190, 80, 60),
        );

        let cur_y = body.current_position.max(0.0) as f32;
        let ball_y = ctx.ground_y - (cur_y * h_scale) - 10.0;
        ctx.painter.circle_filled(
            Pos2::new(tower_x, ball_y),
            10.0,
            Color32::from_rgb(80, 200, 255),
        );
        ctx.painter.circle_stroke(
            Pos2::new(tower_x, ball_y),
            10.0,
            Stroke::new(1.5, Color32::WHITE),
        );

        ctx.painter.text(
            Pos2::new(tower_x + 18.0, ball_y),
            egui::Align2::LEFT_CENTER,
            format!("y = {:.2} m", cur_y),
            egui::FontId::monospace(11.0),
            Color32::WHITE,
        );

        ctx.painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(tower_x - 20.0, ctx.ground_y - 4.0),
                Vec2::new(40.0, 6.0),
            ),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(200, 100, 60),
        );
    }

    pub(super) fn paint_lunar_drop(
        &self,
        ctx: &PaintContext<'_>,
        body: &kinema_adapter_ui::UiBodyView,
        is_feather: bool,
    ) {
        let base_x = if is_feather {
            ctx.origin_x + 220.0
        } else {
            ctx.origin_x + 90.0
        };
        let h_scale = 80.0f32;
        let cur_y = body.current_position.max(0.0) as f32;
        let py = ctx.ground_y - (cur_y * h_scale) - 14.0;

        ctx.painter.text(
            Pos2::new(ctx.rect.center().x, ctx.ground_y + 18.0),
            egui::Align2::CENTER_TOP,
            "APOLLO 15 LUNAR VACUUM: g = 1.62 m/s² | ρ = 0 kg/m³",
            egui::FontId::monospace(11.0),
            Color32::from_rgb(180, 210, 240),
        );

        if is_feather {
            let p_top = Pos2::new(base_x, py - 18.0);
            let p_bot = Pos2::new(base_x, py + 12.0);
            ctx.painter
                .line_segment([p_top, p_bot], Stroke::new(2.0, Color32::WHITE));
            for k in 0..6 {
                let vy = py - 14.0 + (k as f32 * 4.0);
                ctx.painter.line_segment(
                    [Pos2::new(base_x, vy), Pos2::new(base_x - 8.0, vy - 4.0)],
                    Stroke::new(1.2, Color32::from_rgb(220, 230, 240)),
                );
                ctx.painter.line_segment(
                    [Pos2::new(base_x, vy), Pos2::new(base_x + 8.0, vy - 4.0)],
                    Stroke::new(1.2, Color32::from_rgb(220, 230, 240)),
                );
            }
            ctx.painter.text(
                Pos2::new(base_x, py - 24.0),
                egui::Align2::CENTER_BOTTOM,
                "Falcon Feather",
                egui::FontId::monospace(10.0),
                Color32::from_rgb(200, 220, 240),
            );
        } else {
            let head_rect =
                Rect::from_min_size(Pos2::new(base_x - 14.0, py - 16.0), Vec2::new(28.0, 8.0));
            ctx.painter.rect_filled(
                head_rect,
                egui::CornerRadius::same(2),
                Color32::from_rgb(170, 185, 200),
            );
            ctx.painter.line_segment(
                [Pos2::new(base_x, py - 12.0), Pos2::new(base_x, py + 14.0)],
                Stroke::new(3.0, Color32::from_rgb(160, 110, 60)),
            );
            ctx.painter.text(
                Pos2::new(base_x, py - 24.0),
                egui::Align2::CENTER_BOTTOM,
                "Geological Hammer",
                egui::FontId::monospace(10.0),
                Color32::from_rgb(200, 220, 240),
            );
        }
    }

    pub(super) fn paint_vertical_projectile(
        &self,
        ctx: &PaintContext<'_>,
        body: &kinema_adapter_ui::UiBodyView,
        v0: f64,
    ) {
        let launch_x = (ctx.origin_x + 100.0).clamp(ctx.rect.min.x + 60.0, ctx.rect.max.x - 60.0);
        let max_height = (v0 * v0 / (2.0 * 9.81)) as f32;
        let h_scale = (ctx.rect.height() * 0.65 / max_height.max(1.0)).min(12.0);

        ctx.painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(launch_x - 12.0, ctx.ground_y - 12.0),
                Vec2::new(24.0, 12.0),
            ),
            egui::CornerRadius::same(2),
            Color32::from_rgb(60, 70, 85),
        );

        let apex_y = ctx.ground_y - (max_height * h_scale);
        ctx.painter.line_segment(
            [
                Pos2::new(launch_x - 40.0, apex_y),
                Pos2::new(launch_x + 40.0, apex_y),
            ],
            Stroke::new(1.0, Color32::from_rgb(255, 80, 80)),
        );
        ctx.painter.text(
            Pos2::new(launch_x + 45.0, apex_y),
            egui::Align2::LEFT_CENTER,
            format!("APEX (h_max = {:.1} m)", max_height),
            egui::FontId::monospace(10.0),
            Color32::from_rgb(255, 120, 120),
        );

        let cur_y = body.current_position.max(0.0) as f32;
        let py = ctx.ground_y - (cur_y * h_scale) - 10.0;
        ctx.painter.circle_filled(
            Pos2::new(launch_x, py),
            8.0,
            Color32::from_rgb(255, 140, 40),
        );

        if body.current_velocity.abs() > 0.1 {
            let v_len = -(body.current_velocity as f32 * 1.5).clamp(-40.0, 40.0);
            ctx.painter.line_segment(
                [Pos2::new(launch_x, py), Pos2::new(launch_x, py + v_len)],
                Stroke::new(2.5, Color32::YELLOW),
            );
        }

        ctx.painter.text(
            Pos2::new(launch_x, py - 14.0),
            egui::Align2::CENTER_BOTTOM,
            format!("y = {:.2} m | v = {:.1} m/s", cur_y, body.current_velocity),
            egui::FontId::monospace(10.0),
            Color32::WHITE,
        );
    }

    pub(super) fn paint_friction_crate(
        &self,
        ctx: &PaintContext<'_>,
        body: &kinema_adapter_ui::UiBodyView,
        params: FrictionParams,
    ) {
        let px = ctx.origin_x + (body.current_position as f32 * ctx.zoom);
        let crate_w = 54.0f32;
        let crate_h = 40.0f32;
        let py = ctx.ground_y - crate_h;

        if px < ctx.rect.min.x - 70.0 || px > ctx.rect.max.x + 70.0 {
            return;
        }

        let crate_rect = Rect::from_min_size(Pos2::new(px, py), Vec2::new(crate_w, crate_h));
        ctx.painter.rect_filled(
            crate_rect,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(160, 110, 55),
        );
        ctx.painter.line_segment(
            [Pos2::new(px, py), Pos2::new(px + crate_w, py + crate_h)],
            Stroke::new(2.0, Color32::from_rgb(120, 80, 40)),
        );
        ctx.painter.line_segment(
            [Pos2::new(px, py + crate_h), Pos2::new(px + crate_w, py)],
            Stroke::new(2.0, Color32::from_rgb(120, 80, 40)),
        );
        ctx.painter.rect_stroke(
            crate_rect,
            egui::CornerRadius::ZERO,
            Stroke::new(2.0, Color32::from_rgb(60, 45, 30)),
            egui::StrokeKind::Inside,
        );

        let center_x = px + crate_w * 0.5;
        let center_y = py + crate_h * 0.5;

        // Weight W = mg
        ctx.painter.line_segment(
            [
                Pos2::new(center_x, center_y),
                Pos2::new(center_x, center_y + 35.0),
            ],
            Stroke::new(2.0, Color32::from_rgb(255, 60, 60)),
        );
        ctx.painter.text(
            Pos2::new(center_x, center_y + 37.0),
            egui::Align2::CENTER_TOP,
            "W",
            egui::FontId::monospace(9.0),
            Color32::from_rgb(255, 80, 80),
        );

        // Normal force N
        ctx.painter.line_segment(
            [
                Pos2::new(center_x, center_y),
                Pos2::new(center_x, center_y - 35.0),
            ],
            Stroke::new(2.0, Color32::from_rgb(60, 180, 255)),
        );
        ctx.painter.text(
            Pos2::new(center_x, center_y - 37.0),
            egui::Align2::CENTER_BOTTOM,
            "N",
            egui::FontId::monospace(9.0),
            Color32::from_rgb(80, 200, 255),
        );

        // Applied push force F_ext
        if params.f_app.abs() > 0.1 {
            let f_len = (params.f_app as f32 * 0.15).clamp(10.0, 50.0);
            ctx.painter.line_segment(
                [
                    Pos2::new(px + crate_w, center_y),
                    Pos2::new(px + crate_w + f_len, center_y),
                ],
                Stroke::new(2.5, Color32::from_rgb(60, 230, 80)),
            );
            ctx.painter.text(
                Pos2::new(px + crate_w + f_len + 4.0, center_y),
                egui::Align2::LEFT_CENTER,
                format!("{:.0}N", params.f_app),
                egui::FontId::monospace(9.0),
                Color32::from_rgb(80, 240, 100),
            );
        }

        // Friction force f_r
        if body.current_velocity.abs() > 0.05 || params.f_app > 0.0 {
            ctx.painter.line_segment(
                [
                    Pos2::new(px, ctx.ground_y - 2.0),
                    Pos2::new(px - 25.0, ctx.ground_y - 2.0),
                ],
                Stroke::new(2.0, Color32::from_rgb(255, 160, 40)),
            );
            ctx.painter.text(
                Pos2::new(px - 28.0, ctx.ground_y - 2.0),
                egui::Align2::RIGHT_CENTER,
                "fr",
                egui::FontId::monospace(9.0),
                Color32::from_rgb(255, 180, 60),
            );
        }

        let fs_max = params.mu_s * params.mass * 9.81;
        let is_static = params.f_app <= fs_max;
        let status_text = if is_static {
            "STATIC EQUILIBRIUM (F_app <= fs_max | a = 0)"
        } else {
            "KINETIC SLIDING (F_app > fs_max | fk = μk·N)"
        };
        let status_color = if is_static {
            Color32::from_rgb(80, 220, 100)
        } else {
            Color32::from_rgb(255, 140, 60)
        };

        ctx.painter.text(
            Pos2::new(center_x, py - 46.0),
            egui::Align2::CENTER_BOTTOM,
            status_text,
            egui::FontId::monospace(10.0),
            status_color,
        );
    }

    pub(super) fn paint_incline_plane(
        &self,
        ctx: &PaintContext<'_>,
        body: &kinema_adapter_ui::UiBodyView,
        angle_rad: f64,
        _incline_length: f64,
    ) {
        let base_x = (ctx.origin_x + 40.0).clamp(ctx.rect.min.x + 20.0, ctx.rect.max.x - 300.0);
        let ramp_width = 240.0f32;
        let ramp_height = ramp_width * angle_rad.tan() as f32;

        let p_bottom_left = Pos2::new(base_x, ctx.ground_y);
        let p_top_left = Pos2::new(base_x, ctx.ground_y - ramp_height);
        let p_bottom_right = Pos2::new(base_x + ramp_width, ctx.ground_y);

        let wedge_points = vec![p_bottom_left, p_top_left, p_bottom_right];
        ctx.painter.add(egui::Shape::convex_polygon(
            wedge_points,
            Color32::from_rgb(45, 60, 80),
            Stroke::new(2.0, Color32::from_rgb(100, 130, 165)),
        ));

        ctx.painter.text(
            Pos2::new(p_bottom_right.x - 35.0, ctx.ground_y - 6.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("θ = {:.0}°", angle_rad.to_degrees()),
            egui::FontId::monospace(11.0),
            Color32::YELLOW,
        );

        let hyp_len = (ramp_width * ramp_width + ramp_height * ramp_height).sqrt();
        let ux = ramp_width / hyp_len;
        let uy = ramp_height / hyp_len;
        let nx = uy;
        let ny = -ux;

        let s_pix = (body.current_position as f32 * 8.0).clamp(0.0, hyp_len - 60.0);
        let p_center = Pos2::new(
            p_top_left.x + ux * (35.0 + s_pix),
            p_top_left.y + uy * (35.0 + s_pix),
        );

        let half_w = 15.0f32;
        let h = 16.0f32;
        let p1 = Pos2::new(p_center.x - ux * half_w, p_center.y - uy * half_w);
        let p2 = Pos2::new(p_center.x + ux * half_w, p_center.y + uy * half_w);
        let p3 = Pos2::new(p2.x + nx * h, p2.y + ny * h);
        let p4 = Pos2::new(p1.x + nx * h, p1.y + ny * h);

        ctx.painter.add(egui::Shape::convex_polygon(
            vec![p1, p2, p3, p4],
            Color32::from_rgb(255, 170, 50),
            Stroke::new(1.5, Color32::BLACK),
        ));

        let label_pos = Pos2::new(p_center.x + nx * (h + 10.0), p_center.y + ny * (h + 10.0));
        ctx.painter.text(
            label_pos,
            egui::Align2::CENTER_BOTTOM,
            &body.name,
            egui::FontId::monospace(9.0),
            Color32::WHITE,
        );
    }

    pub(super) fn paint_atwood_machine(
        &self,
        ctx: &PaintContext<'_>,
        body: &kinema_adapter_ui::UiBodyView,
        m1: f64,
        m2: f64,
    ) {
        let pulley_cx = (ctx.origin_x + 140.0).clamp(ctx.rect.min.x + 80.0, ctx.rect.max.x - 80.0);
        let pulley_cy = ctx.ground_y - 180.0;
        let r = 18.0f32;

        ctx.painter.line_segment(
            [
                Pos2::new(pulley_cx - 40.0, pulley_cy - 20.0),
                Pos2::new(pulley_cx + 40.0, pulley_cy - 20.0),
            ],
            Stroke::new(4.0, Color32::from_rgb(70, 80, 95)),
        );
        ctx.painter.line_segment(
            [
                Pos2::new(pulley_cx, pulley_cy - 20.0),
                Pos2::new(pulley_cx, pulley_cy),
            ],
            Stroke::new(2.5, Color32::from_rgb(90, 105, 125)),
        );

        ctx.painter.circle_filled(
            Pos2::new(pulley_cx, pulley_cy),
            r,
            Color32::from_rgb(140, 155, 175),
        );
        ctx.painter.circle_stroke(
            Pos2::new(pulley_cx, pulley_cy),
            r,
            Stroke::new(2.0, Color32::BLACK),
        );
        ctx.painter
            .circle_filled(Pos2::new(pulley_cx, pulley_cy), 3.0, Color32::BLACK);

        let s = body.current_position as f32 * 12.0;
        let left_y = (pulley_cy + 60.0 - s).clamp(pulley_cy + 25.0, ctx.ground_y - 20.0);
        let right_y = (pulley_cy + 60.0 + s).clamp(pulley_cy + 25.0, ctx.ground_y - 20.0);

        ctx.painter.line_segment(
            [
                Pos2::new(pulley_cx - r, pulley_cy),
                Pos2::new(pulley_cx - r, left_y),
            ],
            Stroke::new(1.8, Color32::WHITE),
        );
        ctx.painter.line_segment(
            [
                Pos2::new(pulley_cx + r, pulley_cy),
                Pos2::new(pulley_cx + r, right_y),
            ],
            Stroke::new(1.8, Color32::WHITE),
        );

        let m1_rect = Rect::from_center_size(
            Pos2::new(pulley_cx - r, left_y + 12.0),
            Vec2::new(26.0, 22.0),
        );
        ctx.painter.rect_filled(
            m1_rect,
            egui::CornerRadius::same(2),
            Color32::from_rgb(64, 180, 255),
        );
        ctx.painter.text(
            Pos2::new(pulley_cx - r, left_y + 12.0),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}kg", m1),
            egui::FontId::monospace(9.0),
            Color32::BLACK,
        );

        let m2_rect = Rect::from_center_size(
            Pos2::new(pulley_cx + r, right_y + 14.0),
            Vec2::new(30.0, 26.0),
        );
        ctx.painter.rect_filled(
            m2_rect,
            egui::CornerRadius::same(2),
            Color32::from_rgb(255, 160, 40),
        );
        ctx.painter.text(
            Pos2::new(pulley_cx + r, right_y + 14.0),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}kg", m2),
            egui::FontId::monospace(9.0),
            Color32::BLACK,
        );
    }

    pub(super) fn paint_table_pulley(
        &self,
        ctx: &PaintContext<'_>,
        body: &kinema_adapter_ui::UiBodyView,
        m1: f64,
        m2: f64,
    ) {
        let table_start_x =
            (ctx.origin_x + 10.0).clamp(ctx.rect.min.x + 10.0, ctx.rect.max.x - 200.0);
        let table_w = 160.0f32;
        let table_h = 65.0f32;
        let table_top_y = ctx.ground_y - table_h;

        let table_rect = Rect::from_min_size(
            Pos2::new(table_start_x, table_top_y),
            Vec2::new(table_w, 8.0),
        );
        ctx.painter.rect_filled(
            table_rect,
            egui::CornerRadius::ZERO,
            Color32::from_rgb(130, 90, 50),
        );
        ctx.painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(table_start_x + 6.0, table_top_y + 8.0),
                Vec2::new(8.0, table_h - 8.0),
            ),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(90, 60, 35),
        );
        ctx.painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(table_start_x + table_w - 14.0, table_top_y + 8.0),
                Vec2::new(8.0, table_h - 8.0),
            ),
            egui::CornerRadius::ZERO,
            Color32::from_rgb(90, 60, 35),
        );

        let pulley_pos = Pos2::new(table_start_x + table_w, table_top_y);
        ctx.painter
            .circle_filled(pulley_pos, 7.0, Color32::from_rgb(160, 175, 190));
        ctx.painter
            .circle_stroke(pulley_pos, 7.0, Stroke::new(1.0, Color32::BLACK));

        let s = (body.current_position as f32 * 10.0).clamp(0.0, table_w - 35.0);
        let m1_x = table_start_x + 20.0 + s;
        let m1_rect =
            Rect::from_min_size(Pos2::new(m1_x, table_top_y - 20.0), Vec2::new(26.0, 20.0));
        ctx.painter.rect_filled(
            m1_rect,
            egui::CornerRadius::same(2),
            Color32::from_rgb(60, 180, 240),
        );
        ctx.painter.text(
            Pos2::new(m1_x + 13.0, table_top_y - 10.0),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}kg", m1),
            egui::FontId::monospace(9.0),
            Color32::BLACK,
        );

        ctx.painter.line_segment(
            [
                Pos2::new(m1_x + 26.0, table_top_y - 7.0),
                Pos2::new(pulley_pos.x, table_top_y - 7.0),
            ],
            Stroke::new(1.5, Color32::WHITE),
        );
        let hanging_y = (table_top_y + 15.0 + s).clamp(table_top_y + 10.0, ctx.ground_y - 15.0);
        ctx.painter.line_segment(
            [
                Pos2::new(pulley_pos.x + 7.0, table_top_y),
                Pos2::new(pulley_pos.x + 7.0, hanging_y),
            ],
            Stroke::new(1.5, Color32::WHITE),
        );

        let m2_rect = Rect::from_center_size(
            Pos2::new(pulley_pos.x + 7.0, hanging_y + 10.0),
            Vec2::new(20.0, 20.0),
        );
        ctx.painter.rect_filled(
            m2_rect,
            egui::CornerRadius::same(2),
            Color32::from_rgb(255, 150, 40),
        );
        ctx.painter.text(
            Pos2::new(pulley_pos.x + 7.0, hanging_y + 10.0),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}kg", m2),
            egui::FontId::monospace(8.0),
            Color32::BLACK,
        );
    }
}
