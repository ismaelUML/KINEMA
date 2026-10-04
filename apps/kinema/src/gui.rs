// Why custom canvas painting instead of egui_plot for the scene?
// Standard plot widgets don't give us custom cart wheels, Atwood pulleys, or tension-colored
// catenary ropes without wrestling their coordinate transforms all night.
// Doing raw painter lines and rects is 100x simpler, deterministically fast, and looks
// unmistakably like a 1995 physics simulation textbook CD-ROM.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use kinema_adapter_storage::PngCanvasExporter;
use kinema_adapter_ui::{GraphKind, HelpTopic, UiPresenter, UiTheme};
use kinema_app::SimulationService;
use kinema_domain::Scene;
use kinema_ports::{
    CancellationToken, ImageExporter, ScenarioCatalog, SceneEditing, SimulationControl,
    SnapshotSink,
};
use std::time::Instant;

pub struct KinemaGuiApp {
    service: SimulationService,
    presenter: UiPresenter,
    is_playing: bool,
    speed: f64,
    last_tick: Instant,
    active_graph: GraphKind,
    status_text: String,
    help_open: bool,
    eq_ref_open: bool,
    about_open: bool,
    canvas_pan_x: f32,
    canvas_zoom: f32,
}

impl Default for KinemaGuiApp {
    fn default() -> Self {
        let mut service = SimulationService::new(Scene::default());
        let _ = service.load_scenario("two_cars_mru");
        let mut presenter = UiPresenter::new();
        presenter.consume_snapshot(service.scene(), service.current_time());

        Self {
            service,
            presenter,
            is_playing: false,
            speed: 1.0,
            last_tick: Instant::now(),
            active_graph: GraphKind::PositionTime,
            status_text: "Ready. Press Space to Play, F1 for Help.".to_string(),
            help_open: false,
            eq_ref_open: false,
            about_open: false,
            canvas_pan_x: 60.0,
            canvas_zoom: 6.0,
        }
    }
}

impl KinemaGuiApp {
    pub fn new() -> Self {
        Self::default()
    }

    fn advance_simulation(&mut self, dt_wall: f64) {
        if !self.is_playing {
            return;
        }
        let sim_dt = dt_wall * self.speed;
        let steps = (sim_dt * 60.0).round() as usize;
        let clamped_steps = steps.clamp(1, 10);
        for _ in 0..clamped_steps {
            self.service.step_forward();
        }
        self.presenter
            .consume_snapshot(self.service.scene(), self.service.current_time());
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.is_playing = !self.is_playing;
            self.status_text = if self.is_playing {
                "Simulation running.".to_string()
            } else {
                "Simulation paused.".to_string()
            };
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowRight)) {
            self.service.step_forward();
            self.presenter
                .consume_snapshot(self.service.scene(), self.service.current_time());
        }
        if ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft)) {
            self.service.step_backward();
            self.presenter
                .consume_snapshot(self.service.scene(), self.service.current_time());
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Home)) {
            self.service.seek(0.0);
            self.presenter
                .consume_snapshot(self.service.scene(), self.service.current_time());
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Z)) && self.service.undo() {
            self.status_text = "Undo successful.".to_string();
            self.presenter
                .consume_snapshot(self.service.scene(), self.service.current_time());
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Y)) && self.service.redo() {
            self.status_text = "Redo successful.".to_string();
            self.presenter
                .consume_snapshot(self.service.scene(), self.service.current_time());
        }
        if ctx.input(|i| i.key_pressed(egui::Key::F1)) {
            self.help_open = !self.help_open;
        }
    }

    fn apply_theme_styling(&self, ctx: &egui::Context) {
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

impl eframe::App for KinemaGuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt_wall = (now - self.last_tick).as_secs_f64().min(0.1);
        self.last_tick = now;

        self.apply_theme_styling(ui.ctx());
        self.handle_shortcuts(ui.ctx());
        self.advance_simulation(dt_wall);

        // Request continuous repaint while playing simulation
        if self.is_playing {
            ui.ctx().request_repaint();
        }

        self.render_menu_and_toolbar(ui);
        self.render_status_bar(ui);
        self.render_side_panel(ui);
        self.render_central_workspace(ui);
        self.render_dialogs(ui.ctx());
    }
}

impl KinemaGuiApp {
    fn render_menu_and_toolbar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("top_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                self.render_file_menu(ui);
                self.render_edit_menu(ui);
                self.render_view_menu(ui);
                self.render_simulate_menu(ui);
                self.render_scene_catalog_menu(ui);
                self.render_help_menu(ui);
            });
            ui.separator();
            self.render_toolbar(ui);
        });
    }

    fn render_file_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("Export Canvas to PNG... (Ctrl+E)").clicked() {
                self.export_current_canvas();
            }
            ui.separator();
            if ui.button("Exit").clicked() {
                std::process::exit(0);
            }
        });
    }

    fn render_edit_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Edit", |ui| {
            if ui.button("Undo (Ctrl+Z)").clicked() && self.service.undo() {
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
            if ui.button("Redo (Ctrl+Y)").clicked() && self.service.redo() {
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
        });
    }

    fn render_view_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("View", |ui| {
            ui.label("Theme:");
            if ui.button("Classic (Win95)").clicked() {
                self.presenter.set_theme(UiTheme::Classic);
            }
            if ui.button("Phosphor Green (CRT)").clicked() {
                self.presenter.set_theme(UiTheme::Phosphor);
            }
            if ui.button("Amber Phosphor (CRT)").clicked() {
                self.presenter.set_theme(UiTheme::Amber);
            }
            ui.separator();
            ui.label("Graph Kind:");
            if ui.button("Position-Time [x-t]").clicked() {
                self.active_graph = GraphKind::PositionTime;
                self.presenter.set_graph_kind(GraphKind::PositionTime);
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
            if ui.button("Velocity-Time [v-t]").clicked() {
                self.active_graph = GraphKind::VelocityTime;
                self.presenter.set_graph_kind(GraphKind::VelocityTime);
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
            if ui.button("Acceleration-Time [a-t]").clicked() {
                self.active_graph = GraphKind::AccelerationTime;
                self.presenter.set_graph_kind(GraphKind::AccelerationTime);
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
        });
    }

    fn render_simulate_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Simulate", |ui| {
            let label = if self.is_playing {
                "Pause (Space)"
            } else {
                "Play (Space)"
            };
            if ui.button(label).clicked() {
                self.is_playing = !self.is_playing;
            }
            if ui.button("Step Forward (Right)").clicked() {
                self.service.step_forward();
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
            if ui.button("Step Backward (Left)").clicked() {
                self.service.step_backward();
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
            if ui.button("Reset to t=0 (Home)").clicked() {
                self.service.seek(0.0);
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
        });
    }

    fn render_scene_catalog_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Scene", |ui| {
            let list = self.service.list_scenarios();
            for scenario in list {
                if ui.button(&scenario).clicked() {
                    let _ = self.service.load_scenario(&scenario);
                    self.presenter
                        .consume_snapshot(self.service.scene(), self.service.current_time());
                    self.status_text = format!("Scenario '{}' loaded.", scenario);
                }
            }
        });
    }

    fn render_help_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Help", |ui| {
            if ui.button("Contents (F1)").clicked() {
                self.help_open = true;
            }
            if ui.button("Equation Reference").clicked() {
                self.eq_ref_open = true;
            }
            if ui.button("About KINEMA").clicked() {
                self.about_open = true;
            }
        });
    }

    fn render_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button(" |< ").on_hover_text("Reset to t = 0").clicked() {
                self.service.seek(0.0);
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
            if ui.button(" << ").on_hover_text("Step back 1s").clicked() {
                for _ in 0..60 {
                    self.service.step_backward();
                }
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }
            let play_text = if self.is_playing { " || " } else { " ▶ " };
            if ui.button(play_text).on_hover_text("Play / Pause").clicked() {
                self.is_playing = !self.is_playing;
            }
            if ui.button(" >> ").on_hover_text("Step forward 1s").clicked() {
                for _ in 0..60 {
                    self.service.step_forward();
                }
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }

            ui.separator();
            ui.label("Speed:");
            egui::ComboBox::from_id_salt("speed_combo")
                .selected_text(format!("{:.1}x", self.speed))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.speed, 0.25, "0.25x");
                    ui.selectable_value(&mut self.speed, 0.5, "0.5x");
                    ui.selectable_value(&mut self.speed, 1.0, "1.0x");
                    ui.selectable_value(&mut self.speed, 2.0, "2.0x");
                    ui.selectable_value(&mut self.speed, 5.0, "5.0x");
                });

            ui.separator();
            ui.heading(format!("t = {:.3} s", self.service.current_time()));

            ui.separator();
            ui.label("Theme:");
            if ui
                .selectable_label(self.presenter.theme() == UiTheme::Classic, "Classic")
                .clicked()
            {
                self.presenter.set_theme(UiTheme::Classic);
            }
            if ui
                .selectable_label(self.presenter.theme() == UiTheme::Phosphor, "Phosphor")
                .clicked()
            {
                self.presenter.set_theme(UiTheme::Phosphor);
            }
            if ui
                .selectable_label(self.presenter.theme() == UiTheme::Amber, "Amber")
                .clicked()
            {
                self.presenter.set_theme(UiTheme::Amber);
            }
        });
    }

    fn render_status_bar(&self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_text);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let fps = if self.is_playing { "60 FPS" } else { "IDLE" };
                    ui.label(format!("dt = 1/240 s | {}", fps));
                });
            });
        });
    }

    fn render_side_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("inspector_side_panel")
            .resizable(true)
            .show(ui, |ui| {
                ui.heading("EQUATIONS & INSPECTOR");
                ui.separator();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.render_body_equations(ui);
                    ui.separator();
                    self.render_meeting_inspector(ui);
                    ui.separator();
                    self.render_pulley_inspector(ui);
                    ui.separator();
                    self.render_fbd_inspector(ui);
                });
            });
    }

    fn render_body_equations(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("BODIES IN MOTION").strong());
        let bodies = self.presenter.model().bodies.clone();

        for body in bodies {
            ui.group(|ui| {
                ui.label(egui::RichText::new(&body.name).strong());
                ui.monospace(&body.formula_text);
                ui.label(format!(
                    "x = {:.2} m  |  v = {:.2} m/s  |  a = {:.2} m/s²",
                    body.current_position, body.current_velocity, body.current_acceleration
                ));
            });
        }
    }

    fn render_meeting_inspector(&self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("MEETING SOLVER").strong());
        ui.monospace(&self.presenter.model().meeting_diagnosis);
        for marker in &self.presenter.model().meeting_markers {
            ui.label(format!("• {}", marker.label));
        }
        for stop in &self.presenter.model().stopping_markers {
            ui.label(format!("• {}", stop.label));
        }
        for apex in &self.presenter.model().apex_markers {
            ui.label(format!("• {}", apex.label));
        }
    }

    fn render_pulley_inspector(&self, ui: &mut egui::Ui) {
        if self.presenter.model().pulley_views.is_empty()
            && self.presenter.model().rope_views.is_empty()
        {
            return;
        }
        ui.label(egui::RichText::new("ROPES & PULLEYS").strong());
        for p in &self.presenter.model().pulley_views {
            ui.label(format!(
                "{}: T = {:.2} N, a = {:.3} m/s²",
                p.system_type, p.tension, p.acceleration
            ));
        }
        for r in &self.presenter.model().rope_views {
            ui.label(format!(
                "Rope [{}]: {} nodes, stretch = {:.3}%",
                r.rope_id, r.node_count, r.stretch_percent
            ));
        }
    }

    fn render_fbd_inspector(&self, ui: &mut egui::Ui) {
        if self.presenter.model().fbd_views.is_empty() {
            return;
        }
        ui.label(egui::RichText::new("FREE-BODY DIAGRAM (FBD)").strong());
        for fbd in &self.presenter.model().fbd_views {
            ui.group(|ui| {
                ui.label(format!("Body: {} ({})", fbd.body_id, fbd.friction_state));
                ui.label(format!(
                    "W = {:.1}N | N = {:.1}N | f_r = {:.1}N | F_net = {:.2}N",
                    fbd.weight, fbd.normal, fbd.friction, fbd.net_force
                ));
            });
        }
    }

    fn render_central_workspace(&mut self, ui: &mut egui::Ui) {
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

        // Handle panning with mouse drag
        if response.dragged() {
            self.canvas_pan_x += response.drag_delta().x;
        }

        let ground_y = rect.max.y - 45.0;
        let origin_x = rect.min.x + self.canvas_pan_x;

        // Draw metric track and ticks
        let track_color = Color32::from_rgb(80, 95, 115);
        painter.line_segment(
            [
                Pos2::new(rect.min.x, ground_y),
                Pos2::new(rect.max.x, ground_y),
            ],
            Stroke::new(2.0, track_color),
        );

        let tick_spacing = 25.0 * self.canvas_zoom;
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

        // Draw bodies
        self.paint_bodies(&painter, rect, origin_x, ground_y);
        // Draw ropes
        self.paint_ropes(&painter, rect);
    }

    fn paint_bodies(&self, painter: &egui::Painter, rect: Rect, origin_x: f32, ground_y: f32) {
        let model = self.presenter.model();
        for (i, body) in model.bodies.iter().enumerate() {
            let px = origin_x + (body.current_position as f32 * self.canvas_zoom);
            let py = ground_y - 24.0;

            if px < rect.min.x - 50.0 || px > rect.max.x + 50.0 {
                continue;
            }

            let cart_color = match i % 3 {
                0 => Color32::from_rgb(64, 200, 255),  // Cyan
                1 => Color32::from_rgb(255, 180, 50),  // Amber
                _ => Color32::from_rgb(100, 240, 140), // Lime
            };

            let cart_rect = Rect::from_min_size(Pos2::new(px, py), Vec2::new(36.0, 18.0));
            painter.rect(
                cart_rect,
                egui::CornerRadius::ZERO,
                cart_color,
                Stroke::new(1.0, Color32::BLACK),
                egui::StrokeKind::Inside,
            );

            // Wheels
            painter.circle_filled(
                Pos2::new(px + 6.0, ground_y - 4.0),
                4.0,
                Color32::LIGHT_GRAY,
            );
            painter.circle_filled(
                Pos2::new(px + 30.0, ground_y - 4.0),
                4.0,
                Color32::LIGHT_GRAY,
            );

            // Label
            painter.text(
                Pos2::new(px + 18.0, py - 6.0),
                egui::Align2::CENTER_BOTTOM,
                &body.name,
                egui::FontId::monospace(11.0),
                Color32::WHITE,
            );

            // Velocity arrow
            if body.current_velocity.abs() > 0.05 {
                let arrow_len = (body.current_velocity as f32 * 2.0).clamp(-60.0, 60.0);
                let p_start = Pos2::new(px + 18.0, py + 9.0);
                let p_end = Pos2::new(px + 18.0 + arrow_len, py + 9.0);
                painter.line_segment([p_start, p_end], Stroke::new(2.0, Color32::YELLOW));
            }
        }

        // Draw meeting markers
        for m in &model.meeting_markers {
            let mx = origin_x + (m.position as f32 * self.canvas_zoom);
            if mx >= rect.min.x && mx <= rect.max.x {
                painter.text(
                    Pos2::new(mx, ground_y - 40.0),
                    egui::Align2::CENTER_CENTER,
                    "X (MEET)",
                    egui::FontId::monospace(12.0),
                    Color32::from_rgb(255, 60, 60),
                );
            }
        }
    }

    fn paint_ropes(&self, painter: &egui::Painter, rect: Rect) {
        let model = self.presenter.model();
        let center_x = rect.center().x;
        let center_y = rect.center().y;

        for rope in &model.rope_views {
            for seg in &rope.segments {
                let p0 = Pos2::new(
                    center_x + (seg.p0[0] as f32 * 25.0),
                    center_y - (seg.p0[1] as f32 * 25.0),
                );
                let p1 = Pos2::new(
                    center_x + (seg.p1[0] as f32 * 25.0),
                    center_y - (seg.p1[1] as f32 * 25.0),
                );
                let color = parse_hex_color(&seg.color_hex);
                painter.line_segment([p0, p1], Stroke::new(3.0, color));
            }
        }
    }

    fn render_graphs_view(&mut self, ui: &mut egui::Ui) {
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

    fn render_dialogs(&mut self, ctx: &egui::Context) {
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

    fn export_current_canvas(&mut self) {
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

pub fn run_gui() -> Result<(), eframe::Error> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("KINEMA - Interactive Physics Workbench"),
        ..Default::default()
    };
    eframe::run_native(
        "KINEMA - Interactive Physics Workbench",
        native_options,
        Box::new(|_cc| Ok(Box::new(KinemaGuiApp::new()))),
    )
}
