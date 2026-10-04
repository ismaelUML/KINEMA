//! KINEMA Composition Root: Dependency Injection & Application Entry Point.

use kinema_adapter_cli::CliRunner;
use kinema_adapter_storage::KinFileStorage;
use kinema_adapter_ui::UiPresenter;
use kinema_app::SimulationService;
use kinema_domain::motion::Mru;
use kinema_domain::scene::{Body, Scene};
use kinema_ports::{SimulationControl, SnapshotSink};

fn main() {
    println!("=======================================================");
    println!(" K I N E M A  --  Interactive Physics Workbench (M0)   ");
    println!("=======================================================");

    // Initialize initial scene
    let mut initial_scene = Scene::new("Two cars meeting");
    initial_scene.add_body(Body {
        id: "car_a".to_string(),
        name: "Car A (MRU)".to_string(),
        motion: Mru::new(0.0, 15.0),
    });
    initial_scene.add_body(Body {
        id: "car_b".to_string(),
        name: "Car B (MRU)".to_string(),
        motion: Mru::new(100.0, -10.0),
    });

    // Wire layers (Hexagonal Architecture)
    let _storage = KinFileStorage::new();
    let mut service = SimulationService::new(initial_scene);
    let mut presenter = UiPresenter::new();

    // Initial state snapshot
    presenter.consume_snapshot(service.scene(), service.current_time());
    println!("Window Title: {}", presenter.model().window_title);
    println!("Status:       {}", presenter.model().status_message);

    // Run sample step
    service.step_forward();
    presenter.consume_snapshot(service.scene(), service.current_time());

    let report = CliRunner::summarize_scene(service.scene(), service.current_time());
    for line in report {
        println!("{}", line);
    }

    println!("\nKINEMA M0 skeleton initialized successfully.");
}
