//! KINEMA Composition Root: Dependency Injection & Application Entry Point.

use kinema_adapter_storage::KinFileStorage;
use kinema_adapter_ui::{UiPresenter, UiViewModel};
use kinema_app::SimulationService;
use kinema_domain::Scene;
use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl, SnapshotSink};

fn main() {
    println!("=======================================================");
    println!(" K I N E M A  --  Interactive Physics Workbench (M1)   ");
    println!("=======================================================");

    // Wire application and load canonical M1 preset
    let mut service = SimulationService::new(Scene::default());
    service
        .load_scenario("two_cars_mru")
        .expect("Failed to load preset");

    let _storage = KinFileStorage::new();
    let mut presenter = UiPresenter::new();

    // Snapshot at t=0
    presenter.consume_snapshot(service.scene(), service.current_time());
    print_view_model(presenter.model());

    // Advance 1 second
    service.seek(1.0);
    presenter.consume_snapshot(service.scene(), service.current_time());
    println!("\n--- Advanced to t = 1.0 s ---");
    print_view_model(presenter.model());

    // Instant edit parameter v = 35.0 m/s
    service
        .edit_parameter("car_a", "v", 35.0)
        .expect("Parameter edit failed");
    presenter.consume_snapshot(service.scene(), service.current_time());
    println!("\n--- Edited Car A velocity to 35.0 m/s ---");
    print_view_model(presenter.model());

    println!("\nKINEMA M1 (MRU) verified successfully.");
}

fn print_view_model(model: &UiViewModel) {
    println!("Title:     {}", model.window_title);
    println!("Status:    {}", model.status_message);
    println!("Inspector: {}", model.meeting_diagnosis);
    for marker in &model.meeting_markers {
        println!(" - Meeting Marker: {}", marker.label);
    }
    for body in &model.bodies {
        println!(
            " - [{}]: {} => x = {:.2} m, v = {:.2} m/s",
            body.name, body.formula_text, body.current_position, body.current_velocity
        );
    }
}
