use super::KinemaGuiApp;
use eframe::egui;
use kinema_adapter_ui::UiPresenter;
use kinema_app::SimulationService;
use kinema_ports::{SceneEditing, SnapshotSink};

#[derive(Clone, Copy, PartialEq, Eq)]
enum BodyCardAction {
    None,
    ToggleSelect,
    Delete,
}

impl KinemaGuiApp {
    pub(super) fn render_side_panel(&mut self, ui: &mut egui::Ui) {
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
        let mut pending_action = None;

        for body in &bodies {
            let is_selected = self.selected_body_id.as_deref() == Some(&body.id);
            let action = render_body_card(
                ui,
                &mut self.service,
                &mut self.presenter,
                &mut self.status_text,
                body,
                is_selected,
            );
            if action != BodyCardAction::None {
                pending_action = Some((body.id.clone(), is_selected, action));
            }
        }

        if let Some((id, is_selected, action)) = pending_action {
            self.apply_body_card_action(&id, is_selected, action);
        }
    }

    fn apply_body_card_action(&mut self, id: &str, is_selected: bool, action: BodyCardAction) {
        if action == BodyCardAction::ToggleSelect {
            self.selected_body_id = if is_selected {
                None
            } else {
                Some(id.to_string())
            };
        } else if action == BodyCardAction::Delete {
            self.delete_body(id);
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
}

fn make_param_drag_value<'a>(name: &str, val: &'a mut f64) -> egui::DragValue<'a> {
    match name {
        "mass" | "m1" | "m2" => {
            egui::DragValue::new(val).range(0.05..=10000.0).speed(0.1).suffix(" kg")
        }
        "mu_s" | "mu_k" => egui::DragValue::new(val).range(0.0..=2.0).speed(0.01),
        "theta_deg" => egui::DragValue::new(val).range(0.0..=89.0).speed(0.5).suffix("°"),
        "f_app" => {
            egui::DragValue::new(val).range(-5000.0..=5000.0).speed(1.0).suffix(" N")
        }
        "g" | "gravity" => {
            egui::DragValue::new(val).range(0.0..=100.0).speed(0.1).suffix(" m/s²")
        }
        "x0" | "y0" | "s0" => {
            egui::DragValue::new(val).range(-2000.0..=2000.0).speed(0.5).suffix(" m")
        }
        "v" | "v0" => {
            egui::DragValue::new(val).range(-500.0..=500.0).speed(0.5).suffix(" m/s")
        }
        "a" => egui::DragValue::new(val).range(-100.0..=100.0).speed(0.1).suffix(" m/s²"),
        _ => egui::DragValue::new(val).speed(0.1),
    }
}

fn render_single_param_row(
    ui: &mut egui::Ui,
    service: &mut SimulationService,
    presenter: &mut UiPresenter,
    status_text: &mut String,
    body_id: &str,
    name: &str,
    value: f64,
) {
    ui.horizontal(|ui| {
        ui.label(format!("{}:", name));
        let mut val = value;
        let drag = make_param_drag_value(name, &mut val);
        if ui.add(drag).changed() {
            if let Ok(()) = service.edit_parameter(body_id, name, val) {
                presenter.consume_snapshot(service.scene(), service.current_time());
                *status_text = format!("Set {} of '{}' to {:.2}", name, body_id, val);
            }
        }
    });
}

fn render_body_card(
    ui: &mut egui::Ui,
    service: &mut SimulationService,
    presenter: &mut UiPresenter,
    status_text: &mut String,
    body: &kinema_adapter_ui::UiBodyView,
    is_selected: bool,
) -> BodyCardAction {
    let mut action = BodyCardAction::None;
    ui.group(|ui| {
        ui.horizontal(|ui| {
            let title_text = if is_selected {
                format!("🎯 {} [LOCKED]", body.name)
            } else {
                format!("▷ {}", body.name)
            };
            if ui.button(egui::RichText::new(title_text).strong()).clicked() {
                action = BodyCardAction::ToggleSelect;
            }
            if is_selected
                && ui
                    .button(
                        egui::RichText::new("🗑 Delete").color(egui::Color32::from_rgb(240, 80, 80)),
                    )
                    .clicked()
            {
                action = BodyCardAction::Delete;
            }
        });
        ui.monospace(&body.formula_text);
        ui.label(format!(
            "x = {:.2} m  |  v = {:.2} m/s  |  a = {:.2} m/s²",
            body.current_position, body.current_velocity, body.current_acceleration
        ));
        render_body_parameters(ui, service, presenter, status_text, body);
    });
    action
}

fn render_body_parameters(
    ui: &mut egui::Ui,
    service: &mut SimulationService,
    presenter: &mut UiPresenter,
    status_text: &mut String,
    body: &kinema_adapter_ui::UiBodyView,
) {
    if body.parameters.is_empty() {
        return;
    }
    ui.separator();
    ui.label(egui::RichText::new("Parameters (Drag / Click to edit):").small());
    for (p_name, p_val) in &body.parameters {
        render_single_param_row(
            ui,
            service,
            presenter,
            status_text,
            &body.id,
            p_name,
            *p_val,
        );
    }
}
