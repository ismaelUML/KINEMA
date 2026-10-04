// Why custom canvas painting instead of egui_plot for the scene?
// Standard plot widgets don't give us custom cart wheels, Atwood pulleys, or tension-colored
// catenary ropes without wrestling their coordinate transforms all night.
// Doing raw painter lines and rects is 100x simpler, deterministically fast, and looks
// unmistakably like a 1995 physics simulation textbook CD-ROM.

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use kinema_adapter_storage::PngCanvasExporter;
use kinema_adapter_ui::{GraphKind, HelpTopic, UiPresenter, UiTheme};
use kinema_app::SimulationService;
use kinema_domain::{EntityKind, Scene};
use kinema_ports::{
    CancellationToken, ImageExporter, ScenarioCatalog, SceneEditing, SimulationControl,
    SnapshotSink,
};
use std::time::Instant;

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

    fn paint_vehicle(
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

    fn paint_wheel(&self, painter: &egui::Painter, cx: f32, cy: f32) {
        painter.circle_filled(Pos2::new(cx, cy), 5.0, Color32::BLACK);
        painter.circle_filled(Pos2::new(cx, cy), 2.5, Color32::LIGHT_GRAY);
    }

    fn paint_free_fall(
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

    fn paint_lunar_drop(
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

    fn paint_vertical_projectile(
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

    fn paint_friction_crate(
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

    fn paint_incline_plane(
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

    fn paint_atwood_machine(
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

    fn paint_table_pulley(
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

    fn paint_ropes(&self, ctx: &PaintContext<'_>) {
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
