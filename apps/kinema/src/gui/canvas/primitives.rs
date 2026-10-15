use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

/// Draws a glowing laser vector with an arrowhead and floating value badge.
pub fn draw_laser_vector(
    painter: &egui::Painter,
    start: Pos2,
    end: Pos2,
    color: Color32,
    label: Option<&str>,
) {
    let dir = end - start;
    let len = dir.length();
    if len < 2.0 {
        return;
    }

    // Outer glow + bright core
    let glow_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 65);
    painter.line_segment([start, end], Stroke::new(5.0, glow_color));
    painter.line_segment([start, end], Stroke::new(2.0, color));

    // Arrowhead geometry
    let u = dir / len;
    let n = Vec2::new(-u.y, u.x);
    let head_size = (len * 0.25).clamp(5.0, 9.0);
    let p_tip = end;
    let p_left = end - u * head_size + n * (head_size * 0.45);
    let p_right = end - u * head_size - n * (head_size * 0.45);

    painter.add(egui::Shape::convex_polygon(
        vec![p_tip, p_left, p_right],
        color,
        Stroke::NONE,
    ));

    // Optional floating value pill
    if let Some(text) = label {
        draw_value_pill(painter, end + u * 4.0, text, color);
    }
}

/// Small pill badge floating near the tip of a vector.
fn draw_value_pill(painter: &egui::Painter, pos: Pos2, text: &str, accent: Color32) {
    let text_len = text.len() as f32;
    let pill_w = (text_len * 6.5).max(24.0) + 8.0;
    let pill_rect = Rect::from_center_size(pos, Vec2::new(pill_w, 14.0));

    painter.rect_filled(
        pill_rect,
        egui::CornerRadius::same(7),
        Color32::from_rgba_unmultiplied(10, 14, 22, 230),
    );
    painter.rect_stroke(
        pill_rect,
        egui::CornerRadius::same(7),
        Stroke::new(1.0, accent),
        egui::StrokeKind::Inside,
    );
    painter.text(
        pos,
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::monospace(9.0),
        Color32::from_rgb(230, 240, 255),
    );
}

/// Draws an interactive floating telemetry HUD card above an active entity.
pub fn draw_telemetry_hud(
    painter: &egui::Painter,
    anchor: Pos2,
    velocity: f64,
    acceleration: f64,
    state_label: &str,
    accent: Color32,
) {
    let card_center = Pos2::new(anchor.x, anchor.y - 28.0);
    let card_size = Vec2::new(82.0, 36.0);
    let card_rect = Rect::from_center_size(card_center, card_size);

    // Leader line from anchor to card bottom
    let leader_start = Pos2::new(anchor.x, anchor.y);
    let leader_end = Pos2::new(anchor.x, card_rect.max.y);
    painter.line_segment(
        [leader_start, leader_end],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(120, 160, 200, 120)),
    );

    // Glassmorphic dark card
    painter.rect_filled(
        card_rect,
        egui::CornerRadius::same(6),
        Color32::from_rgba_unmultiplied(14, 18, 27, 235),
    );
    painter.rect_stroke(
        card_rect,
        egui::CornerRadius::same(6),
        Stroke::new(1.0, accent),
        egui::StrokeKind::Inside,
    );

    // Velocity readout
    let speed_text = format!("{:.1} m/s", velocity.abs());
    painter.text(
        Pos2::new(card_rect.min.x + 8.0, card_rect.min.y + 11.0),
        egui::Align2::LEFT_CENTER,
        speed_text,
        egui::FontId::monospace(11.0),
        Color32::from_rgb(240, 245, 255),
    );

    // Small acceleration bar & state badge
    draw_hud_status_row(painter, card_rect, acceleration, state_label, accent);
}

fn draw_hud_status_row(
    painter: &egui::Painter,
    card_rect: Rect,
    accel: f64,
    state_label: &str,
    _accent: Color32,
) {
    let bar_y = card_rect.max.y - 10.0;
    let bar_start = Pos2::new(card_rect.min.x + 8.0, bar_y);
    let bar_len = 22.0f32;
    let bar_end = Pos2::new(bar_start.x + bar_len, bar_y);

    // Background track
    painter.line_segment(
        [bar_start, bar_end],
        Stroke::new(2.5, Color32::from_rgb(35, 45, 60)),
    );

    // Active fill
    let ratio = (accel as f32 / 5.0).clamp(-1.0, 1.0);
    let fill_color = if ratio < -0.05 {
        Color32::from_rgb(255, 120, 30) // Braking amber
    } else {
        Color32::from_rgb(16, 185, 129) // Cruising / accel green
    };
    let active_end = Pos2::new(bar_start.x + (ratio.abs() * bar_len), bar_y);
    if ratio.abs() > 0.05 {
        painter.line_segment([bar_start, active_end], Stroke::new(2.5, fill_color));
    }

    // State chip text
    painter.text(
        Pos2::new(card_rect.max.x - 6.0, bar_y),
        egui::Align2::RIGHT_CENTER,
        state_label,
        egui::FontId::monospace(8.0),
        fill_color,
    );
}

/// Draws dynamic friction spark particles at sliding contact interfaces.
pub fn draw_friction_sparks(painter: &egui::Painter, base_x: f32, ground_y: f32, seed: f32) {
    let spark_color = Color32::from_rgb(255, 176, 32);
    for i in 0..4 {
        let phase = seed * 15.0 + i as f32 * 1.57;
        let dx = phase.sin() * 12.0 - 8.0;
        let dy = -(phase.cos().abs() * 6.0);
        let p_spark = Pos2::new(base_x + dx, ground_y + dy);
        painter.circle_filled(p_spark, 1.5, spark_color);
    }
}

/// Draws interactive position gizmo handle for moving bodies in canvas.
pub fn draw_position_gizmo(
    painter: &egui::Painter,
    center: Pos2,
    is_hovered: bool,
    is_dragging: bool,
) {
    let accent = if is_dragging {
        Color32::from_rgb(255, 61, 203)
    } else if is_hovered {
        Color32::from_rgb(0, 240, 255)
    } else {
        Color32::from_rgb(0, 190, 220)
    };

    let rect = Rect::from_center_size(center, Vec2::new(56.0, 18.0));
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(5),
        Color32::from_rgba_unmultiplied(10, 15, 24, 220),
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(5),
        Stroke::new(1.2, accent),
        egui::StrokeKind::Inside,
    );
    painter.text(
        center,
        egui::Align2::CENTER_CENTER,
        "↔ MOVE X",
        egui::FontId::monospace(9.0),
        accent,
    );
}

/// Draws interactive velocity gizmo handle at the tip of the velocity vector.
pub fn draw_velocity_gizmo(
    painter: &egui::Painter,
    center: Pos2,
    is_hovered: bool,
    is_dragging: bool,
) {
    let accent = if is_dragging {
        Color32::from_rgb(255, 220, 60)
    } else if is_hovered {
        Color32::from_rgb(255, 176, 32)
    } else {
        Color32::from_rgb(210, 140, 20)
    };

    painter.circle_filled(
        center,
        6.0,
        Color32::from_rgba_unmultiplied(15, 20, 30, 230),
    );
    painter.circle_stroke(center, 6.0, Stroke::new(1.5, accent));
    painter.circle_filled(center, 2.5, accent);
}
