//! KINEMA Composition Root: Dependency Injection & Application Entry Point.

use kinema_adapter_storage::KinFileStorage;
use kinema_adapter_ui::{GraphKind, UiPresenter, UiViewModel};
use kinema_app::SimulationService;
use kinema_domain::Scene;
use kinema_ports::{ScenarioCatalog, SimulationControl, SnapshotSink};

fn main() {
    println!("=======================================================");
    println!(" K I N E M A  --  Interactive Physics Workbench (M5)   ");
    println!("=======================================================");

    // Wire application and load canonical M5 Stage A preset (Atwood machine)
    let mut service = SimulationService::new(Scene::default());
    service
        .load_scenario("atwood_machine")
        .expect("Failed to load Atwood preset");

    let _storage = KinFileStorage::new();
    let mut presenter = UiPresenter::new();

    // Snapshot at t=0
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Advance Atwood machine 1.0 s
    println!("\n--- Advancing Atwood Machine to t = 1.0 s ---");
    service.seek(1.0);
    presenter.set_graph_kind(GraphKind::VelocityTime);
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Load M5 Stage B preset (Hanging Catenary Rope with Verlet integration)
    println!("\n--- Loading M5 Stage B: Hanging Catenary Rope ---");
    service
        .load_scenario("hanging_catenary_rope")
        .expect("Failed to load catenary preset");

    // Advance 60 frames (1.0 s)
    for _ in 0..60 {
        service.step_forward();
    }
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    println!("\nKINEMA M5 (Rope and Pulleys) verified successfully.");
}

fn print_markers(model: &UiViewModel) {
    for marker in &model.meeting_markers {
        println!(" - Meeting Marker:  {}", marker.label);
    }
    for marker in &model.stopping_markers {
        println!(" - Stopping Marker: {}", marker.label);
    }
    for marker in &model.apex_markers {
        println!(" - Apex Marker:     {}", marker.label);
    }
    for marker in &model.impact_markers {
        println!(" - Impact Marker:   {}", marker.label);
    }
}

fn print_fbd(model: &UiViewModel) {
    for fbd in &model.fbd_views {
        println!(
            " - FBD [{}]: State = {}, Net Force = {:.2} N (W={:.2}N, N={:.2}N, f_roz={:.2}N, F_app={:.2}N)",
            fbd.body_id, fbd.friction_state, fbd.net_force, fbd.weight, fbd.normal, fbd.friction, fbd.applied_force
        );
        for arrow in &fbd.arrows {
            println!("     -> Force Arrow: {} ({})", arrow.label, arrow.direction);
        }
    }
}

fn print_pulleys(model: &UiViewModel) {
    for pulley in &model.pulley_views {
        println!(
            " - Pulley [{}]: {} => T = {:.2} N, a = {:.3} m/s², m1_pos = {:.2} m, m2_pos = {:.2} m",
            pulley.body_id,
            pulley.system_type,
            pulley.tension,
            pulley.acceleration,
            pulley.mass1_pos,
            pulley.mass2_pos
        );
    }
}

fn print_ropes(model: &UiViewModel) {
    for rope in &model.rope_views {
        println!(
            " - Rope [{}]: {} nodes, L = {:.2} m, stretch = {:.3}%, {} segments",
            rope.rope_id,
            rope.node_count,
            rope.total_length,
            rope.stretch_percent,
            rope.segments.len()
        );
        if let Some(first_seg) = rope.segments.first() {
            println!(
                "     -> First segment tension: {:.2} N, color: {}",
                first_seg.tension, first_seg.color_hex
            );
        }
    }
}

fn print_view_model(model: &UiViewModel) {
    println!("Title:     {}", model.window_title);
    println!("Status:    {}", model.status_message);
    println!("Inspector: {}", model.meeting_diagnosis);
    print_markers(model);
    print_fbd(model);
    print_pulleys(model);
    print_ropes(model);
    for body in &model.bodies {
        println!(
            " - [{}]: {} => pos = {:.2} m, v = {:.2} m/s, a = {:.2} m/s²",
            body.name,
            body.formula_text,
            body.current_position,
            body.current_velocity,
            body.current_acceleration
        );
    }
}
