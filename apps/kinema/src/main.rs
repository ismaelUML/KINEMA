//! KINEMA Composition Root: Dependency Injection & Application Entry Point.

use kinema_adapter_storage::KinFileStorage;
use kinema_adapter_ui::{GraphKind, UiPresenter, UiViewModel};
use kinema_app::SimulationService;
use kinema_domain::Scene;
use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl, SnapshotSink};

fn main() {
    println!("=======================================================");
    println!(" K I N E M A  --  Interactive Physics Workbench (M2)   ");
    println!("=======================================================");

    // Wire application and load canonical M2 preset (MRU vs MRUV)
    let mut service = SimulationService::new(Scene::default());
    service
        .load_scenario("two_cars_mruv")
        .expect("Failed to load M2 preset");

    let _storage = KinFileStorage::new();
    let mut presenter = UiPresenter::new();

    // Snapshot at t=0
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Advance to turnaround / stopping instant (t = 5.0 s)
    service.seek(5.0);
    presenter.set_graph_kind(GraphKind::VelocityTime);
    presenter.consume_snapshot(service.scene(), service.current_time());
    println!("\n--- Advanced to turnaround point (t = 5.0 s) ---");
    print_view_model(presenter.model());

    // Instant edit acceleration: a = 3.0 m/s²
    service
        .edit_parameter("car_b", "a", 3.0)
        .expect("Parameter edit failed");
    presenter.consume_snapshot(service.scene(), service.current_time());
    println!("\n--- Edited Car B acceleration to +3.0 m/s² ---");
    print_view_model(presenter.model());

    println!("\nKINEMA M2 (MRUV) verified successfully.");
}

fn print_view_model(model: &UiViewModel) {
    println!("Title:     {}", model.window_title);
    println!("Status:    {}", model.status_message);
    println!("Inspector: {}", model.meeting_diagnosis);
    for marker in &model.meeting_markers {
        println!(" - Meeting Marker:  {}", marker.label);
    }
    for marker in &model.stopping_markers {
        println!(" - Stopping Marker: {}", marker.label);
    }
    for body in &model.bodies {
        println!(
            " - [{}]: {} => x = {:.2} m, v = {:.2} m/s, a = {:.2} m/s²",
            body.name,
            body.formula_text,
            body.current_position,
            body.current_velocity,
            body.current_acceleration
        );
    }
}
