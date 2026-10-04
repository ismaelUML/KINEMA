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

#[test]
fn test_m5_end_to_end_acceptance() {
    use kinema_domain::pulley::AtwoodMachine;
    use kinema_domain::rope::ParticleRope;
    use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl};

    let empty = Scene::new("Empty");
    let mut service = SimulationService::new(empty);
    let mut presenter = UiPresenter::new();

    // 1. Stage A: Atwood machine formula and simulated bound
    service
        .load_scenario("atwood_machine")
        .expect("Atwood preset must load");
    presenter.consume_snapshot(service.scene(), service.current_time());

    let initial_pulleys = &presenter.model().pulley_views;
    assert_eq!(initial_pulleys.len(), 1);
    let atwood_view = &initial_pulleys[0];
    assert_eq!(atwood_view.system_type, "Atwood Machine");

    // a = (3 - 2) * 9.81 / 5 = 1.962 m/s²
    // T = 2 * 2 * 3 * 9.81 / 5 = 23.544 N
    let expected_a = 1.962;
    let expected_t = 23.544;
    assert!((atwood_view.acceleration - expected_a).abs() < 1e-4);
    assert!((atwood_view.tension - expected_t).abs() < 1e-4);

    // Integrator bound for Atwood: 480 steps at 240 Hz (2.0s)
    let atwood = AtwoodMachine::new(2.0, 3.0).unwrap().with_gravity(9.81);
    let mut num_s = 0.0;
    let mut num_v = 0.0;
    for _ in 0..480 {
        let (ns, nv, _) = atwood.step_euler(num_s, num_v, 1.0 / 240.0);
        num_s = ns;
        num_v = nv;
    }
    assert!((num_v - expected_a * 2.0).abs() < 1e-9);
    assert!((num_s - 0.5 * expected_a * 4.0).abs() < 0.01);

    // 2. Stage A: Table pulley static-to-kinetic threshold
    service
        .load_scenario("table_pulley_friction")
        .expect("Table pulley preset must load");
    presenter.consume_snapshot(service.scene(), service.current_time());
    let table_view = &presenter.model().pulley_views[0];
    assert_eq!(table_view.system_type, "Table Pulley");
    // Initially m1=10, m2=6 => m2*g = 58.86 N > 49.05 N (Kinetic)
    assert!((table_view.acceleration - 1.839375).abs() < 1e-3);
    assert!((table_view.tension - 47.82375).abs() < 1e-3);

    // Edit hanging mass down to m2 = 1 kg (m2*g = 9.81 N <= 49.05 N => Transitions to Static!)
    service
        .edit_parameter("table_pulley", "m2", 1.0)
        .expect("edit m2 to 1kg");
    presenter.consume_snapshot(service.scene(), service.current_time());
    let static_view = &presenter.model().pulley_views[0];
    assert_eq!(static_view.acceleration, 0.0);
    assert!((static_view.tension - 9.81).abs() < 1e-3);

    // 3. Stage B: Particle-chain rope (24 nodes, 12 relaxation passes)
    service
        .load_scenario("hanging_catenary_rope")
        .expect("Catenary preset must load");

    // Advance 120 frames (2.0s wall clock, 480 sub-steps)
    for _ in 0..120 {
        service.step_forward();
    }
    presenter.consume_snapshot(service.scene(), service.current_time());
    let rope_view = &presenter.model().rope_views[0];
    assert_eq!(rope_view.node_count, 24);
    assert_eq!(rope_view.segments.len(), 23);

    // Stage B Acceptance 1: Stretch stays strictly under 1%
    assert!(
        rope_view.stretch_percent < 1.0,
        "Stretch must stay under 1%, got {:.3}%",
        rope_view.stretch_percent
    );

    // Stage B Acceptance 2: 10 minutes (144,000 steps) stability & no NaN
    let mut rope = ParticleRope::new_catenary([0.0, 4.0], [2.0, 4.0], 3.0, 1.5).unwrap();
    rope.set_pinned(23, true).unwrap();
    for _ in 0..144_000 {
        rope.step(1.0 / 240.0);
    }
    for node in &rope.nodes {
        assert!(node.pos[0].is_finite());
        assert!(node.pos[1].is_finite());
    }
    assert!(rope.stretch_ratio() < 0.01);

    // 4. Persistence roundtrip with KinFileStorage
    let storage = KinFileStorage::new();
    let serialized = storage.serialize_scene(service.scene());
    let reloaded = storage.parse_str(&serialized).expect("roundtrip parse");
    assert_eq!(reloaded.ropes.len(), 1);
    assert_eq!(reloaded.ropes[0].nodes.len(), 24);
    assert_eq!(reloaded.ropes[0].relaxation_passes, 12);

    // 5. Bounded Undo/Redo
    assert!(service.undo());
    assert!(service.redo());
}

#[test]
fn test_m6_end_to_end_acceptance() {
    use kinema_adapter_storage::PngCanvasExporter;
    use kinema_adapter_ui::{HelpTopic, UiTheme};
    use kinema_ports::{
        CancellationToken, ImageExporter, ScenarioCatalog, SimulationControl, SnapshotSink,
    };
    use std::fs;

    let empty = Scene::new("Empty");
    let mut service = SimulationService::new(empty);
    let mut presenter = UiPresenter::new();

    // 1. Scenario Catalog Completeness
    let catalog = service.list_scenarios();
    assert!(catalog.contains(&"two_cars_mru".to_string()));
    assert!(catalog.contains(&"two_cars_mruv".to_string()));
    assert!(catalog.contains(&"parallel_cars".to_string()));
    assert!(catalog.contains(&"coinciding_cars".to_string()));
    assert!(catalog.contains(&"20m_free_fall".to_string()));
    assert!(catalog.contains(&"feather_and_hammer_moon".to_string()));
    assert!(catalog.contains(&"vertical_projectile".to_string()));
    assert!(catalog.contains(&"block_friction_threshold".to_string()));
    assert!(catalog.contains(&"incline_plane_slide".to_string()));
    assert!(catalog.contains(&"heavy_crate_push".to_string()));
    assert!(catalog.contains(&"atwood_machine".to_string()));
    assert!(catalog.contains(&"table_pulley_friction".to_string()));
    assert!(catalog.contains(&"hanging_catenary_rope".to_string()));
    assert!(catalog.contains(&"rope_surface_friction".to_string()));

    // Load active scenario for UI & Export test
    service
        .load_scenario("atwood_machine")
        .expect("Load scenario must succeed");
    service.seek(1.5);
    presenter.consume_snapshot(service.scene(), service.current_time());

    // 2. Themes & Palette Switching
    assert_eq!(presenter.theme(), UiTheme::Classic);
    assert_eq!(presenter.model().palette.title_bg, "#000080");
    assert_eq!(presenter.model().palette.window_face, "#C0C0C0");

    presenter.set_theme(UiTheme::Phosphor);
    assert_eq!(presenter.theme(), UiTheme::Phosphor);
    assert_eq!(presenter.model().palette.text_primary, "#33FF33");
    assert_eq!(presenter.model().palette.window_face, "#000000");

    presenter.set_theme(UiTheme::Amber);
    assert_eq!(presenter.theme(), UiTheme::Amber);
    assert_eq!(presenter.model().palette.text_primary, "#FFB000");
    assert_eq!(presenter.model().palette.window_face, "#000000");

    // 3. Help System & Dialogs
    presenter.open_help(HelpTopic::Contents);
    assert!(presenter.model().help_dialog_open);
    assert_eq!(
        presenter.model().current_help_topic,
        Some(HelpTopic::Contents)
    );
    assert!(presenter
        .model()
        .help_text
        .as_ref()
        .unwrap()
        .contains("Keyboard Shortcuts"));

    presenter.open_help(HelpTopic::EquationReference);
    assert_eq!(
        presenter.model().current_help_topic,
        Some(HelpTopic::EquationReference)
    );
    assert!(presenter
        .model()
        .help_text
        .as_ref()
        .unwrap()
        .contains("M1 - MRU"));
    assert!(presenter
        .model()
        .help_text
        .as_ref()
        .unwrap()
        .contains("M5 - Atwood Machine"));

    presenter.open_help(HelpTopic::About);
    assert_eq!(presenter.model().current_help_topic, Some(HelpTopic::About));
    assert!(presenter
        .model()
        .help_text
        .as_ref()
        .unwrap()
        .contains("Old but functional"));

    presenter.close_help();
    assert!(!presenter.model().help_dialog_open);
    assert!(presenter.model().help_text.is_none());

    // 4. Menu Tree Structure
    let menus = &presenter.model().menus;
    let root_titles: Vec<&str> = menus.iter().map(|item| item.title.as_str()).collect();
    assert_eq!(
        root_titles,
        vec!["File", "Edit", "View", "Simulate", "Scene", "Help"]
    );
    let file_menu = &menus[0];
    let file_actions: Vec<&str> = file_menu
        .items
        .iter()
        .map(|item| item.action_id.as_str())
        .collect();
    assert!(file_actions.contains(&"file.new"));
    assert!(file_actions.contains(&"file.open"));
    assert!(file_actions.contains(&"file.save"));
    assert!(file_actions.contains(&"file.export_png"));

    // 5. Canvas PNG Export & Verification
    let exporter = PngCanvasExporter::new(640, 480);
    let export_path = std::env::temp_dir().join("kinema_m6_acceptance.png");
    let export_str = export_path.to_str().unwrap();
    let token = CancellationToken::new();

    let export_res =
        exporter.export_png(service.scene(), service.current_time(), export_str, &token);
    assert!(export_res.is_ok(), "PNG export failed: {:?}", export_res);

    let png_bytes = fs::read(&export_path).expect("read exported PNG file");
    assert!(png_bytes.len() > 100);
    assert_eq!(&png_bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]); // Magic signature
    assert_eq!(&png_bytes[12..16], b"IHDR");
    let w = u32::from_be_bytes(png_bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(png_bytes[20..24].try_into().unwrap());
    assert_eq!(w, 640);
    assert_eq!(h, 480);
    let len = png_bytes.len();
    assert_eq!(&png_bytes[len - 8..len - 4], b"IEND");

    let _ = fs::remove_file(&export_path);

    // 6. Cooperative Cancellation & Rollback
    let cancel_path = std::env::temp_dir().join("kinema_m6_cancelled.png");
    let cancel_str = cancel_path.to_str().unwrap();
    let cancel_token = CancellationToken::new();
    cancel_token.cancel();

    let cancel_res = exporter.export_png(
        service.scene(),
        service.current_time(),
        cancel_str,
        &cancel_token,
    );
    assert!(cancel_res.is_err());
    assert_eq!(cancel_res.unwrap_err(), "Export cancelled by user");
    assert!(!cancel_path.exists(), "Cancelled file must be removed");
}
