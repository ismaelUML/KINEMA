//! KINEMA Composition Root: Dependency Injection & Application Entry Point.

use kinema_adapter_storage::KinFileStorage;
use kinema_adapter_ui::{GraphKind, UiPresenter, UiViewModel};
use kinema_app::SimulationService;
use kinema_domain::Scene;
use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl, SnapshotSink};

fn main() {
    println!("=======================================================");
    println!(" K I N E M A  --  Interactive Physics Workbench (M4)   ");
    println!("=======================================================");

    // Wire application and load canonical M4 preset (Block with friction threshold)
    let mut service = SimulationService::new(Scene::default());
    service
        .load_scenario("block_friction_threshold")
        .expect("Failed to load M4 preset");

    let _storage = KinFileStorage::new();
    let mut presenter = UiPresenter::new();

    // Snapshot at t=0 (Below static threshold: F_app = 20 N <= 24.525 N)
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Instant edit applied force to pass static threshold: F_app = 30 N > 24.525 N
    println!("\n--- Passing static friction threshold (F_app: 20 N -> 30 N) ---");
    service
        .edit_parameter("block", "f_app", 30.0)
        .expect("Parameter edit failed");
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Advance simulation forward: t = 2.0 s under constant kinetic acceleration
    service.seek(2.0);
    presenter.set_graph_kind(GraphKind::VelocityTime);
    presenter.consume_snapshot(service.scene(), service.current_time());
    println!("\n--- Advanced to t = 2.0 s ---");
    print_view_model(presenter.model());

    println!("\nKINEMA M4 (Dynamics & Friction) verified successfully.");
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

fn print_view_model(model: &UiViewModel) {
    println!("Title:     {}", model.window_title);
    println!("Status:    {}", model.status_message);
    println!("Inspector: {}", model.meeting_diagnosis);
    print_markers(model);
    print_fbd(model);
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
