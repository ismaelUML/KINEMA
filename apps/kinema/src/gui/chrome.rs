use super::KinemaGuiApp;
use eframe::egui;
use kinema_adapter_ui::{GraphKind, UiTheme};
use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl, SnapshotSink};

impl KinemaGuiApp {
    pub(super) fn render_menu_and_toolbar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("top_bar").show(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                self.render_file_menu(ui);
                self.render_edit_menu(ui);
                self.render_entities_menu(ui);
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
            if ui.button("📄 New Scene... (Ctrl+N)").clicked() {
                self.new_scene_open = true;
            }
            if ui.button("✨ Clear Scene (Empty Canvas)").clicked() {
                self.create_new_scene("Blank Canvas", 9.80665);
            }
            ui.separator();
            if ui.button("💾 Save Scene (.kin)... (Ctrl+S)").clicked() {
                self.save_scene_open = true;
            }
            if ui.button("📂 Open Scene (.kin)... (Ctrl+O)").clicked() {
                self.load_scene_open = true;
            }
            ui.separator();
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

    fn render_entities_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Entities", |ui| {
            if ui.button("🚗 + Vehicle (MRU: 15 m/s)").clicked() {
                self.spawn_vehicle_mru();
            }
            if ui.button("🏎 + Vehicle (MRUV: Braking)").clicked() {
                self.spawn_vehicle_mruv();
            }
            if ui.button("🏀 + Free Fall Ball (20m)").clicked() {
                self.spawn_free_fall();
            }
            if ui.button("📦 + Sliding Crate (50kg)").clicked() {
                self.spawn_friction_crate();
            }
            if ui.button("📐 + Incline Ramp (30°)").clicked() {
                self.spawn_incline_block();
            }
            if let Some(id) = self.selected_body_id.clone() {
                ui.separator();
                if ui.button(format!("🗑 Delete '{}'", id)).clicked() {
                    self.delete_body(&id);
                }
            }
        });
    }

    fn render_view_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("View", |ui| {
            ui.label("Theme:");
            if ui.button("Telemetry (Modern Dark)").clicked() {
                self.presenter.set_theme(UiTheme::Telemetry);
            }
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
            ui.heading(format!("t = {:.2}s", self.service.current_time()));

            let mut curr_t = self.service.current_time();
            let t_max = self.presenter.model().duration_hint.max(10.0);
            let slider = egui::Slider::new(&mut curr_t, 0.0..=t_max)
                .show_value(false)
                .step_by(0.01);
            if ui.add_sized([140.0, 18.0], slider).changed() {
                self.service.seek(curr_t);
                self.presenter
                    .consume_snapshot(self.service.scene(), self.service.current_time());
            }

            render_spawner_toolbar_controls(ui, self);
            render_zoom_toolbar_controls(ui, &mut self.canvas_zoom, &mut self.canvas_pan_x);
            ui.separator();
            render_theme_toolbar_controls(ui, &mut self.presenter);
        });
    }

    pub(super) fn render_status_bar(&self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status_bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_text);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let fps = if self.is_playing {
                        format!("{:.0} FPS", self.fps_measured)
                    } else {
                        "PAUSED".to_string()
                    };
                    ui.label(format!("dt = 1/240 s | {}", fps));
                });
            });
        });
    }
}

fn render_zoom_toolbar_controls(ui: &mut egui::Ui, zoom: &mut f32, pan_x: &mut f32) {
    ui.separator();
    ui.label("Zoom:");
    if ui
        .button(" - ")
        .on_hover_text("Zoom out (Mouse wheel down)")
        .clicked()
    {
        *zoom = (*zoom * 0.8).clamp(0.5, 40.0);
    }
    ui.label(format!("{:.1}x", *zoom / 6.0));
    if ui
        .button(" + ")
        .on_hover_text("Zoom in (Mouse wheel up)")
        .clicked()
    {
        *zoom = (*zoom * 1.25).clamp(0.5, 40.0);
    }
    if ui.button("1:1").on_hover_text("Reset zoom and pan").clicked() {
        *zoom = 6.0;
        *pan_x = 60.0;
    }
}

fn render_spawner_toolbar_controls(ui: &mut egui::Ui, app: &mut KinemaGuiApp) {
    if ui
        .button("📄 New Scene")
        .on_hover_text("Create a new blank physics scene (Ctrl+N)")
        .clicked()
    {
        app.new_scene_open = true;
    }

    ui.menu_button("+ Spawn", |ui| {
        if ui.button("🚗 Vehicle (MRU: 15 m/s)").clicked() {
            app.spawn_vehicle_mru();
        }
        if ui.button("🏎 Vehicle (MRUV: Braking)").clicked() {
            app.spawn_vehicle_mruv();
        }
        if ui.button("🏀 Free Fall Ball (20m)").clicked() {
            app.spawn_free_fall();
        }
        if ui.button("📦 Friction Crate (50kg)").clicked() {
            app.spawn_friction_crate();
        }
        if ui.button("📐 Incline Ramp (30°)").clicked() {
            app.spawn_incline_block();
        }
    });

    if let Some(id) = app.selected_body_id.clone() {
        if ui
            .button("🗑 Delete")
            .on_hover_text("Delete selected body from scene")
            .clicked()
        {
            app.delete_body(&id);
        }
    }
}

fn render_theme_toolbar_controls(
    ui: &mut egui::Ui,
    presenter: &mut kinema_adapter_ui::UiPresenter,
) {
    ui.label("Theme:");
    if ui
        .selectable_label(presenter.theme() == UiTheme::Telemetry, "Telemetry")
        .clicked()
    {
        presenter.set_theme(UiTheme::Telemetry);
    }
    if ui
        .selectable_label(presenter.theme() == UiTheme::Classic, "Classic")
        .clicked()
    {
        presenter.set_theme(UiTheme::Classic);
    }
    if ui
        .selectable_label(presenter.theme() == UiTheme::Phosphor, "Phosphor")
        .clicked()
    {
        presenter.set_theme(UiTheme::Phosphor);
    }
    if ui
        .selectable_label(presenter.theme() == UiTheme::Amber, "Amber")
        .clicked()
    {
        presenter.set_theme(UiTheme::Amber);
    }
}

impl KinemaGuiApp {
    pub(super) fn create_new_scene(&mut self, name: &str, gravity: f64) {
        let _ = self.service.create_new_scene(name, gravity);
        self.selected_body_id = None;
        self.service.seek(0.0);
        self.presenter
            .consume_snapshot(self.service.scene(), self.service.current_time());
        self.status_text =
            format!("Created new blank scene '{}' (g = {:.2} m/s²).", name, gravity);
    }

    pub(super) fn delete_body(&mut self, id: &str) {
        let _ = self.service.remove_body(id);
        if self.selected_body_id.as_deref() == Some(id) {
            self.selected_body_id = None;
        }
        self.presenter
            .consume_snapshot(self.service.scene(), self.service.current_time());
        self.status_text = format!("Deleted body '{}'.", id);
    }

    pub(super) fn spawn_vehicle_mru(&mut self) {
        use kinema_domain::{Body, EntityKind, Mru};
        let count = self.service.scene().bodies.len() + 1;
        let id = format!("car_{}", count);
        let name = format!("Vehicle {}", count);
        let body = Body::new(&id, &name, Mru::new(0.0, 15.0))
            .with_kind(EntityKind::Vehicle { lane: count % 3 });
        let _ = self.service.add_body(body);
        self.selected_body_id = Some(id);
        self.presenter
            .consume_snapshot(self.service.scene(), self.service.current_time());
        self.status_text = format!("Added new MRU vehicle '{}'.", name);
    }

    pub(super) fn spawn_vehicle_mruv(&mut self) {
        use kinema_domain::{Body, EntityKind, Mruv};
        let count = self.service.scene().bodies.len() + 1;
        let id = format!("car_mruv_{}", count);
        let name = format!("Braking Car {}", count);
        let body = Body::new(&id, &name, Mruv::braking(80.0, -12.0, 2.5))
            .with_kind(EntityKind::Vehicle { lane: count % 3 });
        let _ = self.service.add_body(body);
        self.selected_body_id = Some(id);
        self.presenter
            .consume_snapshot(self.service.scene(), self.service.current_time());
        self.status_text = format!("Added new MRUV vehicle '{}'.", name);
    }

    pub(super) fn spawn_free_fall(&mut self) {
        use kinema_domain::{Body, EntityKind, Mvl};
        let count = self.service.scene().bodies.len() + 1;
        let id = format!("ball_{}", count);
        let name = format!("Drop Ball {}", count);
        let body = Body::new(&id, &name, Mvl::new(20.0, 0.0, 9.81).with_ground_stop(true))
            .with_kind(EntityKind::FreeFall {
                initial_height: 20.0,
            });
        let _ = self.service.add_body(body);
        self.selected_body_id = Some(id);
        self.presenter
            .consume_snapshot(self.service.scene(), self.service.current_time());
        self.status_text = format!("Added free fall drop '{}'.", name);
    }

    pub(super) fn spawn_friction_crate(&mut self) {
        use kinema_domain::{BlockDynamics, Body, EntityKind};
        let count = self.service.scene().bodies.len() + 1;
        let id = format!("crate_{}", count);
        let name = format!("Wooden Crate {}", count);
        let block = BlockDynamics::horizontal(50.0, 0.5, 0.3, 30.0);
        let body = Body::new(&id, &name, block).with_kind(EntityKind::FrictionBlock {
            mass: 50.0,
            mu_s: 0.5,
            mu_k: 0.3,
            f_app: 30.0,
        });
        let _ = self.service.add_body(body);
        self.selected_body_id = Some(id);
        self.presenter
            .consume_snapshot(self.service.scene(), self.service.current_time());
        self.status_text = format!("Added friction crate '{}'.", name);
    }

    pub(super) fn spawn_incline_block(&mut self) {
        use kinema_domain::{Body, EntityKind, Mruv};
        let count = self.service.scene().bodies.len() + 1;
        let id = format!("ramp_block_{}", count);
        let name = format!("Incline Block {}", count);
        let angle = 30.0f64.to_radians();
        let a = 9.81 * angle.sin();
        let body = Body::new(&id, &name, Mruv::new(0.0, 0.0, a)).with_kind(
            EntityKind::InclineBlock {
                angle_rad: angle,
                incline_length: 20.0,
            },
        );
        let _ = self.service.add_body(body);
        self.selected_body_id = Some(id);
        self.presenter
            .consume_snapshot(self.service.scene(), self.service.current_time());
        self.status_text = format!("Added incline plane block '{}'.", name);
    }
}
