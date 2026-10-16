use super::KinemaGuiApp;
use eframe::egui;
use kinema_adapter_storage::{KinFileStorage, PngCanvasExporter};
use kinema_adapter_ui::HelpTopic;
use kinema_ports::{CancellationToken, ImageExporter, SceneRepository, SnapshotSink};

impl KinemaGuiApp {
    pub(super) fn render_dialogs(&mut self, ctx: &egui::Context) {
        self.render_help_dialogs(ctx);
        self.render_new_scene_dialog(ctx);
        self.render_save_scene_dialog(ctx);
        self.render_load_scene_dialog(ctx);
    }

    fn render_help_dialogs(&mut self, ctx: &egui::Context) {
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

    fn render_new_scene_dialog(&mut self, ctx: &egui::Context) {
        if !self.new_scene_open {
            return;
        }
        let mut is_open = self.new_scene_open;
        let mut should_create = false;
        let mut should_close = false;

        egui::Window::new("📄 Create New Scene")
            .open(&mut is_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Scene Name:");
                ui.text_edit_singleline(&mut self.new_scene_name);

                ui.separator();
                ui.label("Gravity Preset:");
                render_gravity_preset_buttons(ui, &mut self.new_scene_gravity);

                ui.horizontal(|ui| {
                    ui.label("Gravity (m/s²):");
                    ui.add(
                        egui::DragValue::new(&mut self.new_scene_gravity)
                            .speed(0.1)
                            .range(0.0..=100.0)
                            .suffix(" m/s²"),
                    );
                });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("✨ Create Blank Scene").clicked() {
                        should_create = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        self.new_scene_open = is_open && !should_close;
        if should_create {
            let name = self.new_scene_name.clone();
            let g = self.new_scene_gravity;
            self.create_new_scene(&name, g);
            self.new_scene_open = false;
        }
    }

    fn render_save_scene_dialog(&mut self, ctx: &egui::Context) {
        if !self.save_scene_open {
            return;
        }
        let mut is_open = self.save_scene_open;
        let mut should_save = false;
        let mut should_close = false;

        egui::Window::new("💾 Save Scene (.kin)")
            .open(&mut is_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("File Path:");
                ui.text_edit_singleline(&mut self.save_scene_path);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        should_save = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        self.save_scene_open = is_open && !should_close;
        if should_save {
            let storage = KinFileStorage;
            match storage.save(&self.save_scene_path, self.service.scene()) {
                Ok(()) => {
                    self.status_text = format!("Scene saved to '{}'", self.save_scene_path);
                    self.save_scene_open = false;
                }
                Err(e) => {
                    self.status_text = format!("Save error: {}", e);
                }
            }
        }
    }

    fn render_load_scene_dialog(&mut self, ctx: &egui::Context) {
        if !self.load_scene_open {
            return;
        }
        let mut is_open = self.load_scene_open;
        let mut should_load = false;
        let mut should_close = false;

        egui::Window::new("📂 Open Scene (.kin)")
            .open(&mut is_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("File Path:");
                ui.text_edit_singleline(&mut self.load_scene_path);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() {
                        should_load = true;
                    }
                    if ui.button("Cancel").clicked() {
                        should_close = true;
                    }
                });
            });

        self.load_scene_open = is_open && !should_close;
        if should_load {
            let storage = KinFileStorage;
            match storage.load(&self.load_scene_path) {
                Ok(scene) => {
                    let name = scene.name.clone();
                    self.service.load_custom_scene(scene);
                    self.selected_body_id = None;
                    self.presenter
                        .consume_snapshot(self.service.scene(), self.service.current_time());
                    self.status_text =
                        format!("Loaded scene '{}' from '{}'", name, self.load_scene_path);
                    self.load_scene_open = false;
                }
                Err(e) => {
                    self.status_text = format!("Load error: {}", e);
                }
            }
        }
    }

    pub(super) fn export_current_canvas(&mut self) {
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

fn render_gravity_preset_buttons(ui: &mut egui::Ui, gravity: &mut f64) {
    ui.horizontal(|ui| {
        if ui.button("Earth (9.81)").clicked() {
            *gravity = 9.80665;
        }
        if ui.button("Moon (1.62)").clicked() {
            *gravity = 1.62;
        }
        if ui.button("Mars (3.71)").clicked() {
            *gravity = 3.71;
        }
        if ui.button("Zero-G (0.0)").clicked() {
            *gravity = 0.0;
        }
    });
}
