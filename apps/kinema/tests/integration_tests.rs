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
    scene.add_body(Body::new("veh_1", "Vehicle 1", Mru::new(0.0, 20.0)));

    let storage = KinFileStorage::new();
    let serialized = storage.serialize_scene(&scene);
    let parsed_scene = storage.parse_str(&serialized).expect("Roundtrip parse");

    let mut service = SimulationService::new(parsed_scene);
    let mut presenter = UiPresenter::new();

    service.step_forward();
    presenter.consume_snapshot(service.scene(), service.current_time());

    let summary = CliRunner::summarize_scene(service.scene(), service.current_time());
    assert!(!summary.is_empty());
    assert_eq!(presenter.model().bodies.len(), 1);
}

#[test]
fn test_m1_end_to_end_acceptance() {
    use kinema_ports::{ScenarioCatalog, SceneEditing};

    let empty = Scene::new("Empty");
    let mut service = SimulationService::new(empty);
    let mut presenter = UiPresenter::new();

    // 1. Load canonical M1 scenario via ScenarioCatalog port
    service
        .load_scenario("two_cars_mru")
        .expect("scenario must load");
    presenter.consume_snapshot(service.scene(), service.current_time());

    let initial_model = presenter.model();
    assert_eq!(initial_model.bodies.len(), 2);
    assert_eq!(
        initial_model.bodies[0].formula_text,
        "x(t) = 0.00 + 15.00 · t"
    );
    assert_eq!(
        initial_model.bodies[1].formula_text,
        "x(t) = 100.00 - 10.00 · t"
    );
    assert_eq!(initial_model.meeting_markers.len(), 1);
    assert_eq!(initial_model.meeting_markers[0].time, 4.0);
    assert_eq!(initial_model.meeting_markers[0].position, 60.0);

    // 2. Edit velocity on same frame: v_A = 35.0 m/s
    service
        .edit_parameter("car_a", "v", 35.0)
        .expect("edit parameter v");
    presenter.consume_snapshot(service.scene(), service.current_time());

    let updated_model = presenter.model();
    // Formula updated instantly
    assert_eq!(
        updated_model.bodies[0].formula_text,
        "x(t) = 0.00 + 35.00 · t"
    );
    // Meeting instant recalculated on same frame: t* = 100 / (35 - (-10)) = 100 / 45 = 2.2222222222222223
    let expected_t = 100.0 / 45.0;
    let actual_t = updated_model.meeting_markers[0].time;
    let rel_error = (actual_t - expected_t).abs() / expected_t;
    assert!(
        rel_error < 1e-9,
        "Relative error {} must be strictly < 1e-9",
        rel_error
    );
}

#[test]
fn test_m2_end_to_end_acceptance() {
    use kinema_adapter_ui::GraphKind;
    use kinema_ports::{ScenarioCatalog, SceneEditing};

    let empty = Scene::new("Empty");
    let mut service = SimulationService::new(empty);
    let mut presenter = UiPresenter::new();

    // 1. Load canonical M2 preset (MRU vs MRUV)
    service
        .load_scenario("two_cars_mruv")
        .expect("M2 preset must load");
    presenter.set_graph_kind(GraphKind::VelocityTime);
    presenter.consume_snapshot(service.scene(), service.current_time());

    let initial_model = presenter.model();
    assert_eq!(initial_model.bodies.len(), 2);
    assert_eq!(initial_model.meeting_markers.len(), 2);

    // Two roots: t = 5 s, t = 20 s
    assert!((initial_model.meeting_markers[0].time - 5.0).abs() < 1e-9);
    assert!((initial_model.meeting_markers[0].position - 75.0).abs() < 1e-9);
    assert!((initial_model.meeting_markers[1].time - 20.0).abs() < 1e-9);
    assert!((initial_model.meeting_markers[1].position - 300.0).abs() < 1e-9);

    // Stopping instant for Car B: t_s = 5 s
    assert_eq!(initial_model.stopping_markers.len(), 1);
    assert!((initial_model.stopping_markers[0].time - 5.0).abs() < 1e-9);

    // 2. Edit acceleration on same frame: a = 4.0 m/s²
    service
        .edit_parameter("car_b", "a", 4.0)
        .expect("edit parameter a");
    presenter.consume_snapshot(service.scene(), service.current_time());

    let updated_model = presenter.model();
    // Formula updated instantly with new acceleration
    assert!(updated_model.bodies[1].formula_text.contains("2.00 · t²"));
    // New stopping time for Car B: t_s = -(-10)/4 = 2.5 s
    assert!((updated_model.stopping_markers[0].time - 2.5).abs() < 1e-9);
}
