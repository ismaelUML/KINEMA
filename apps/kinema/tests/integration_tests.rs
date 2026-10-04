use kinema_adapter_cli::CliRunner;
use kinema_adapter_storage::KinFileStorage;
use kinema_adapter_ui::UiPresenter;
use kinema_app::SimulationService;
use kinema_domain::motion::Mru;
use kinema_domain::scene::{Body, Scene};
use kinema_ports::{SimulationControl, SnapshotSink};

#[test]
fn test_m0_end_to_end_wire() {
    let mut scene = Scene::new("Integration Scene");
    scene.add_body(Body {
        id: "veh_1".to_string(),
        name: "Vehicle 1".to_string(),
        motion: Mru::new(0.0, 20.0),
    });

    let storage = KinFileStorage::new();
    let serialized = storage.serialize_scene(&scene);
    let parsed_scene = storage.parse_str(&serialized).expect("Roundtrip parse");

    let mut service = SimulationService::new(parsed_scene);
    let mut presenter = UiPresenter::new();

    service.step_forward();
    presenter.consume_snapshot(service.scene(), service.current_time());

    let summary = CliRunner::summarize_scene(service.scene(), service.current_time());
    assert!(!summary.is_empty());
    assert_eq!(presenter.model().bodies_count, 1);
}
