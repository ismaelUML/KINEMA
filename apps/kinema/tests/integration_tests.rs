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

#[test]
fn test_m3_end_to_end_acceptance() {
    use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl};

    let empty = Scene::new("Empty");
    let mut service = SimulationService::new(empty);
    let mut presenter = UiPresenter::new();

    // 1. Load canonical M3 20m free fall scenario
    service
        .load_scenario("20m_free_fall")
        .expect("M3 preset must load");
    presenter.consume_snapshot(service.scene(), service.current_time());

    let initial_model = presenter.model();
    assert_eq!(initial_model.bodies.len(), 1);
    assert!(initial_model.bodies[0].formula_text.contains("20.00"));
    assert_eq!(initial_model.impact_markers.len(), 1);

    // M3 Acceptance: t_i ≈ 2.019 s, |v_i| ≈ 19.8 m/s
    let impact = &initial_model.impact_markers[0];
    assert!((impact.time - 2.01927).abs() < 1e-4);
    assert!((impact.speed - 19.809).abs() < 1e-3);

    // 2. Exact Scrubbing: seek to impact instant yields y ≈ 0
    service.seek(impact.time);
    presenter.consume_snapshot(service.scene(), service.current_time());
    let scrubbed_pos = presenter.model().bodies[0].current_position;
    assert!(
        scrubbed_pos.abs() < 1e-3,
        "Position at impact instant should be ~0, got {scrubbed_pos}"
    );

    // 3. Serialization Roundtrip through KinFileStorage
    let storage = KinFileStorage::new();
    let serialized = storage.serialize_scene(service.scene());
    let reloaded = storage.parse_str(&serialized).expect("Roundtrip parse");
    let reloaded_service = SimulationService::new(reloaded);
    let mut reloaded_presenter = UiPresenter::new();
    reloaded_presenter.consume_snapshot(reloaded_service.scene(), reloaded_service.current_time());
    let reloaded_impact = &reloaded_presenter.model().impact_markers[0];
    assert!((reloaded_impact.time - 2.01927).abs() < 1e-4);

    // 4. Vertical projectile: apex and impact
    service
        .load_scenario("vertical_projectile")
        .expect("load projectile");
    presenter.consume_snapshot(service.scene(), service.current_time());
    let proj_model = presenter.model();
    assert_eq!(proj_model.apex_markers.len(), 1);
    let apex = &proj_model.apex_markers[0];
    assert!((apex.time - (20.0 / 9.81)).abs() < 1e-4);
    assert!((apex.height - (400.0 / 19.62)).abs() < 1e-4);

    // 5. Bounded Undo/Redo on parameter editing
    service.edit_parameter("rock", "v0", 30.0).expect("edit v0");
    presenter.consume_snapshot(service.scene(), service.current_time());
    let edited_apex = &presenter.model().apex_markers[0];
    assert!((edited_apex.time - (30.0 / 9.81)).abs() < 1e-4);

    assert!(service.undo());
    presenter.consume_snapshot(service.scene(), service.current_time());
    let undone_apex = &presenter.model().apex_markers[0];
    assert!((undone_apex.time - (20.0 / 9.81)).abs() < 1e-4);

    assert!(service.redo());
    presenter.consume_snapshot(service.scene(), service.current_time());
    let redone_apex = &presenter.model().apex_markers[0];
    assert!((redone_apex.time - (30.0 / 9.81)).abs() < 1e-4);
}

#[test]
fn test_m4_end_to_end_acceptance() {
    use kinema_domain::dynamics::BlockDynamics;
    use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl};

    let empty = Scene::new("Empty");
    let mut service = SimulationService::new(empty);
    let mut presenter = UiPresenter::new();

    // 1. Load canonical M4 preset: Block on horizontal surface with friction
    service
        .load_scenario("block_friction_threshold")
        .expect("M4 preset must load");
    presenter.consume_snapshot(service.scene(), service.current_time());

    let initial_model = presenter.model();
    assert_eq!(initial_model.bodies.len(), 1);
    assert_eq!(initial_model.fbd_views.len(), 1);

    // Initial state: F_app = 20 N <= fs_max = 24.525 N => STATIC
    let initial_fbd = &initial_model.fbd_views[0];
    assert_eq!(initial_fbd.friction_state, "STATIC");
    assert_eq!(initial_fbd.net_force, 0.0);
    assert_eq!(initial_model.bodies[0].current_acceleration, 0.0);

    // Scrubbing while at rest: stays at rest
    service.seek(5.0);
    presenter.consume_snapshot(service.scene(), service.current_time());
    assert_eq!(presenter.model().bodies[0].current_position, 0.0);
    assert_eq!(presenter.model().bodies[0].current_velocity, 0.0);

    // 2. In-frame parameter edit: increase F_app to 30 N > 24.525 N
    service
        .edit_parameter("block", "f_app", 30.0)
        .expect("edit f_app to 30 N");
    service.seek(0.0);
    presenter.consume_snapshot(service.scene(), service.current_time());

    // Transitions to KINETIC
    let kinetic_fbd = &presenter.model().fbd_views[0];
    assert_eq!(kinetic_fbd.friction_state, "KINETIC");
    assert!((kinetic_fbd.net_force - 15.285).abs() < 1e-3);
    let expected_a = 15.285 / 5.0; // 3.057 m/s^2

    // Advance 2.0 s: analytic rate verification
    service.seek(2.0);
    presenter.consume_snapshot(service.scene(), service.current_time());
    let body = &presenter.model().bodies[0];
    assert!((body.current_velocity - (expected_a * 2.0)).abs() < 1e-4);
    assert!((body.current_position - (0.5 * expected_a * 4.0)).abs() < 1e-4);

    // 3. Semi-Implicit Euler fixed-timestep integrator comparison vs analytic bound
    let dt: f64 = 1.0 / 240.0;
    let block = BlockDynamics::horizontal(5.0, 0.5, 0.3, 30.0).with_gravity(9.81);
    let mut num_pos = 0.0;
    let mut num_vel = 0.0;
    for _ in 0..480 {
        let (np, nv, _, _) = block.step_euler(num_pos, num_vel, dt);
        num_pos = np;
        num_vel = nv;
    }
    // Integrator error bound verification:
    // Velocity is exact for constant acceleration
    assert!((num_vel - body.current_velocity).abs() < 1e-9);
    // Position error strictly < 0.02 m
    assert!((num_pos - body.current_position).abs() < 0.02);

    // 4. Persistence roundtrip through KinFileStorage
    let storage = KinFileStorage::new();
    let serialized = storage.serialize_scene(service.scene());
    let reloaded = storage.parse_str(&serialized).expect("roundtrip parse");
    assert_eq!(reloaded.bodies.len(), 1);
    let mut reloaded_service = SimulationService::new(reloaded);
    reloaded_service.seek(2.0);
    let mut reloaded_presenter = UiPresenter::new();
    reloaded_presenter.consume_snapshot(reloaded_service.scene(), reloaded_service.current_time());
    assert_eq!(
        reloaded_presenter.model().fbd_views[0].friction_state,
        "KINETIC"
    );
    assert!(
        (reloaded_presenter.model().bodies[0].current_velocity - (expected_a * 2.0)).abs() < 1e-4
    );

    // 5. Bounded Undo/Redo
    assert!(service.undo());
    presenter.consume_snapshot(service.scene(), service.current_time());
    assert_eq!(presenter.model().fbd_views[0].friction_state, "STATIC");

    assert!(service.redo());
    presenter.consume_snapshot(service.scene(), service.current_time());
    assert_eq!(presenter.model().fbd_views[0].friction_state, "KINETIC");
}
