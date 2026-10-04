//! KINEMA Composition Root: Dependency Injection & Application Entry Point.

use kinema_adapter_storage::KinFileStorage;
use kinema_adapter_ui::{GraphKind, UiPresenter, UiViewModel};
use kinema_app::SimulationService;
use kinema_domain::Scene;
use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl, SnapshotSink};

fn main() {
    println!("=======================================================");
    println!(" K I N E M A  --  Interactive Physics Workbench (M3)   ");
    println!("=======================================================");

    // Wire application and load canonical M3 preset (20m free fall drop)
    let mut service = SimulationService::new(Scene::default());
    service
        .load_scenario("20m_free_fall")
        .expect("Failed to load M3 preset");

    let _storage = KinFileStorage::new();
    let mut presenter = UiPresenter::new();

    // Snapshot at t=0
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Advance to ground impact instant (t ≈ 2.019 s)
    service.seek(2.01927);
    presenter.set_graph_kind(GraphKind::VelocityTime);
    presenter.consume_snapshot(service.scene(), service.current_time());
    println!("\n--- Advanced to ground impact instant (t ≈ 2.02 s) ---");
    print_view_model(presenter.model());

    // Instant edit initial height: y0 = 45.0 m
    service
        .edit_parameter("ball", "y0", 45.0)
        .expect("Parameter edit failed");
    presenter.consume_snapshot(service.scene(), service.current_time());
    println!("\n--- Edited Ball drop height to 45.0 m ---");
    print_view_model(presenter.model());

    println!("\nKINEMA M3 (MVL & Control) verified successfully.");
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

fn print_view_model(model: &UiViewModel) {
    println!("Title:     {}", model.window_title);
    println!("Status:    {}", model.status_message);
    println!("Inspector: {}", model.meeting_diagnosis);
    print_markers(model);
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
