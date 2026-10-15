use super::{parse_hex_color, PaintContext};
use crate::gui::KinemaGuiApp;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

impl KinemaGuiApp {
    pub(super) fn paint_ropes(&self, ctx: &PaintContext<'_>) {
        let model = self.presenter.model();
        let rope_scale = 80.0f32;
        let base_x = (ctx.origin_x + 100.0).clamp(ctx.rect.min.x + 40.0, ctx.rect.max.x - 240.0);

        for rope in &model.rope_views {
            if let (Some(first), Some(last)) = (rope.segments.first(), rope.segments.last()) {
                let p0 = Pos2::new(
                    base_x + (first.p0[0] as f32 * rope_scale),
                    ctx.ground_y - (first.p0[1] as f32 * rope_scale),
                );
                let p1 = Pos2::new(
                    base_x + (last.p1[0] as f32 * rope_scale),
                    ctx.ground_y - (last.p1[1] as f32 * rope_scale),
                );

                // Overhead mounting beam
                ctx.painter.line_segment(
                    [
                        Pos2::new(p0.x - 20.0, p0.y - 10.0),
                        Pos2::new(p1.x + 20.0, p1.y - 10.0),
                    ],
                    Stroke::new(6.0, Color32::from_rgb(60, 70, 85)),
                );

                // Brackets and pin studs
                ctx.painter.rect_filled(
                    Rect::from_center_size(p0, Vec2::new(14.0, 14.0)),
                    egui::CornerRadius::same(2),
                    Color32::from_rgb(130, 145, 165),
                );
                ctx.painter.circle_filled(p0, 3.0, Color32::BLACK);

                ctx.painter.rect_filled(
                    Rect::from_center_size(p1, Vec2::new(14.0, 14.0)),
                    egui::CornerRadius::same(2),
                    Color32::from_rgb(130, 145, 165),
                );
                ctx.painter.circle_filled(p1, 3.0, Color32::BLACK);
            }

            for seg in &rope.segments {
                let p0 = Pos2::new(
                    base_x + (seg.p0[0] as f32 * rope_scale),
                    ctx.ground_y - (seg.p0[1] as f32 * rope_scale),
                );
                let p1 = Pos2::new(
                    base_x + (seg.p1[0] as f32 * rope_scale),
                    ctx.ground_y - (seg.p1[1] as f32 * rope_scale),
                );
                let color = parse_hex_color(&seg.color_hex);
                ctx.painter.line_segment([p0, p1], Stroke::new(3.5, color));
            }
        }
    }
}
