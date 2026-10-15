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

            render_zoom_toolbar_controls(ui, &mut self.canvas_zoom, &mut self.canvas_pan_x);

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

    pub(super) fn render_status_bar(&self, ui: &mut egui::Ui) {
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
