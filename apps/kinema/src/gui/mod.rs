// Why custom canvas painting instead of egui_plot for the scene?
// Standard plot widgets don't give us custom cart wheels, Atwood pulleys, or tension-colored
// catenary ropes without wrestling their coordinate transforms all night.
// Doing raw painter lines and rects is 100x simpler, deterministically fast, and looks
// unmistakably like a 1995 physics simulation textbook CD-ROM.

mod canvas;
mod chrome;
mod dialogs;
mod graphs;
mod inspector;
mod theme;

use eframe::egui;
use kinema_adapter_ui::{GraphKind, UiPresenter};
use kinema_app::SimulationService;
use kinema_domain::Scene;
use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl, SnapshotSink};
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
    canvas_pan_y: f32,
    canvas_zoom: f32,
    fps_measured: f32,
    selected_body_id: Option<String>,
}

impl Default for KinemaGuiApp {
    fn default() -> Self {
        let mut service = SimulationService::new(Scene::default());
        let _ = service.load_scenario("two_cars_mru");
        let mut presenter = UiPresenter::new();
        presenter.set_theme(kinema_adapter_ui::UiTheme::Telemetry);
        presenter.consume_snapshot(service.scene(), service.current_time());

        Self {
            service,
            presenter,
            is_playing: false,
            speed: 1.0,
            last_tick: Instant::now(),
            active_graph: GraphKind::PositionTime,
            status_text: "Ready. Space: Play | Arrows: Step | F: Reset Camera".to_string(),
            help_open: false,
            eq_ref_open: false,
            about_open: false,
            canvas_pan_x: 60.0,
            canvas_pan_y: 0.0,
            canvas_zoom: 6.0,
            fps_measured: 60.0,
            selected_body_id: None,
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
        if ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.canvas_pan_x = 60.0;
            self.canvas_pan_y = 0.0;
            self.canvas_zoom = 6.0;
            self.status_text = "Camera reset to default view.".to_string();
        }
    }

}

impl eframe::App for KinemaGuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt_wall = (now - self.last_tick).as_secs_f64().min(0.1);
        self.last_tick = now;
        if dt_wall > 0.0005 {
            let instantaneous = (1.0 / dt_wall) as f32;
            self.fps_measured = self.fps_measured * 0.90 + instantaneous * 0.10;
        }

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
