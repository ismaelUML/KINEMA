use kinema_app::SimulationService;
use kinema_domain::motion::{Motion1D, Mru};
use kinema_domain::scene::{Body, Scene};
use kinema_ports::{SceneEditing, SimulationControl};

#[test]
fn test_simulation_control_lifecycle() {
    let scene = Scene::new("Test Scene");
    let mut service = SimulationService::new(scene);

    assert_eq!(service.current_time(), 0.0);
    assert!(!service.is_playing());

    service.play();
    assert!(service.is_playing());

    service.pause();
    assert!(!service.is_playing());

    service.seek(5.5);
    assert_eq!(service.current_time(), 5.5);

    service.reset();
    assert_eq!(service.current_time(), 0.0);
}

#[test]
fn test_scene_editing_and_bounded_undo() {
    use kinema_domain::Motion1D;

    let mut scene = Scene::new("Test Scene");
    scene.add_body(Body::new("car_a", "Car A", Mru::new(0.0, 10.0)));

    let mut service = SimulationService::new(scene);
    assert_eq!(service.scene().bodies[0].motion.velocity_at(0.0), 10.0);

    service.edit_parameter("car_a", "v", 25.0).unwrap();
    assert_eq!(service.scene().bodies[0].motion.velocity_at(0.0), 25.0);

    assert!(service.undo());
    assert_eq!(service.scene().bodies[0].motion.velocity_at(0.0), 10.0);

    assert!(service.redo());
    assert_eq!(service.scene().bodies[0].motion.velocity_at(0.0), 25.0);
}

#[test]
fn test_scenario_catalog_and_two_cars_meeting() {
    use kinema_domain::MeetingOutcome;
    use kinema_ports::ScenarioCatalog;

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);

    let scenarios = service.list_scenarios();
    assert!(scenarios.contains(&"two_cars_mru".to_string()));

    let loaded = service
        .load_scenario("two_cars_mru")
        .expect("should load canonical preset");
    assert_eq!(loaded.name, "Two cars meeting");
    assert_eq!(loaded.bodies.len(), 2);

    let outcome = service
        .solve_two_body_meeting()
        .expect("should solve meeting");
    match outcome {
        MeetingOutcome::Single(inst) => {
            assert!((inst.time - 4.0).abs() < 1e-9);
            assert!((inst.position - 60.0).abs() < 1e-9);
        }
        _ => panic!("Expected single meeting at 4s, got {:?}", outcome),
    }

    assert!(service.load_scenario("non_existent").is_err());
}

#[test]
fn test_m2_mruv_scenario_and_multi_curve_sampling() {
    use kinema_app::CurveType;
    use kinema_domain::MeetingOutcome;
    use kinema_ports::ScenarioCatalog;

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);

    service
        .load_scenario("two_cars_mruv")
        .expect("should load M2 preset");
    assert_eq!(service.scene().name, "Two cars meeting (MRU vs MRUV)");

    // Two roots meeting test (t=5s, t=20s)
    let outcome = service
        .solve_two_body_meeting()
        .expect("should solve quadratic meeting");
    match outcome {
        MeetingOutcome::Dual(r1, r2) => {
            assert!((r1.time - 5.0).abs() < 1e-9);
            assert!((r1.position - 75.0).abs() < 1e-9);
            assert!((r2.time - 20.0).abs() < 1e-9);
            assert!((r2.position - 300.0).abs() < 1e-9);
        }
        _ => panic!("Expected two roots, got {:?}", outcome),
    }

    // Stopping instant test for car_b
    let stops = service.find_stopping_instants();
    assert_eq!(stops.len(), 1);
    assert_eq!(stops[0].0, "car_b");
    assert!((stops[0].1 - 5.0).abs() < 1e-9);
    assert!((stops[0].2 - 75.0).abs() < 1e-9);

    // Multi-curve sampling: velocity & acceleration curves
    let v_curve = service
        .sample_curve("car_b", CurveType::Velocity, 0.0, 5.0, 3)
        .expect("sample velocity");
    assert_eq!(v_curve.points[0].position, -10.0);
    assert_eq!(v_curve.points[2].position, 0.0);

    let a_curve = service
        .sample_curve("car_b", CurveType::Acceleration, 0.0, 5.0, 3)
        .expect("sample acceleration");
    assert_eq!(a_curve.points[0].position, 2.0);
    assert_eq!(a_curve.points[2].position, 2.0);
}

#[test]
fn test_sample_trajectory_points() {
    use kinema_ports::ScenarioCatalog;

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service.load_scenario("two_cars_mru").unwrap();

    let trajectory = service
        .sample_trajectory("car_a", 0.0, 4.0, 5)
        .expect("should sample car_a");

    assert_eq!(trajectory.body_id, "car_a");
    assert_eq!(trajectory.points.len(), 5);
    assert_eq!(trajectory.points[0].time, 0.0);
    assert_eq!(trajectory.points[0].position, 0.0);
    assert_eq!(trajectory.points[4].time, 4.0);
    assert_eq!(trajectory.points[4].position, 60.0);

    assert!(service.sample_trajectory("unknown", 0.0, 1.0, 5).is_err());
}

#[test]
fn test_m3_20m_free_fall_acceptance_criteria() {
    use kinema_ports::ScenarioCatalog;

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("20m_free_fall")
        .expect("load 20m drop");

    let impacts = service.find_ground_impacts();
    assert_eq!(impacts.len(), 1);
    let (id, t_impact, v_impact) = &impacts[0];
    assert_eq!(id, "ball");
    // t_i = sqrt(2 * 20 / 9.81) = sqrt(4.07747) = 2.01927 s
    assert!((t_impact - 2.01927).abs() < 1e-4);
    // |v_i| = sqrt(2 * 9.81 * 20) = 19.809 m/s
    assert!((v_impact - 19.809).abs() < 1e-3);

    // No upward apex when dropped with v0 = 0
    let apexes = service.find_apex_instants();
    assert!(apexes.is_empty());
}

#[test]
fn test_m3_feather_and_hammer_moon_simultaneous_fall() {
    use kinema_ports::ScenarioCatalog;

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("feather_and_hammer_moon")
        .expect("load moon scenario");

    let impacts = service.find_ground_impacts();
    assert_eq!(impacts.len(), 2);
    // Both hit the lunar surface at t = sqrt(2 * 1.62 / 1.62) = sqrt(2) ≈ 1.4142 s
    let expected_t = (2.0_f64).sqrt();
    for (_id, t_i, _v_i) in &impacts {
        assert!((t_i - expected_t).abs() < 1e-6);
    }
}

#[test]
fn test_m3_vertical_projectile_apex_and_curves() {
    use kinema_app::CurveType;
    use kinema_ports::ScenarioCatalog;

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("vertical_projectile")
        .expect("load vertical projectile");

    let apexes = service.find_apex_instants();
    assert_eq!(apexes.len(), 1);
    let (id, t_up, h_max) = &apexes[0];
    assert_eq!(id, "rock");
    // t_up = 20 / 9.81 ≈ 2.03874 s
    assert!((t_up - (20.0 / 9.81)).abs() < 1e-4);
    // h_max = 20^2 / (2 * 9.81) ≈ 20.38736 m
    assert!((h_max - (400.0 / 19.62)).abs() < 1e-4);

    let impacts = service.find_ground_impacts();
    assert_eq!(impacts.len(), 1);
    let (_, t_impact, v_impact) = &impacts[0];
    // Symmetrical flight: total air time is 2 * t_up
    assert!((t_impact - (40.0 / 9.81)).abs() < 1e-4);
    // Impact speed equals launch speed in vacuum
    assert!((v_impact - 20.0).abs() < 1e-4);

    // Verify velocity curve: v(0) = 20, v(t_up) ≈ 0
    let v_curve = service
        .sample_curve("rock", CurveType::Velocity, 0.0, *t_up, 2)
        .expect("sample v curve");
    assert!((v_curve.points[0].position - 20.0).abs() < 1e-6);
    assert!(v_curve.points[1].position.abs() < 1e-3);
}

#[test]
fn test_m4_block_friction_threshold_scenario() {
    use kinema_domain::FrictionState;
    use kinema_ports::{ScenarioCatalog, SceneEditing};

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("block_friction_threshold")
        .expect("load block friction");

    // Initially F_app = 20 N <= 24.525 N => Static
    let fbds = service.free_body_diagrams();
    assert_eq!(fbds.len(), 1);
    let (id, fbd) = &fbds[0];
    assert_eq!(id, "block");
    assert_eq!(fbd.state, FrictionState::Static);
    assert_eq!(fbd.net_force, 0.0);

    // Edit F_app to 30 N > 24.525 N => Kinetic acceleration
    service
        .edit_parameter("block", "f_app", 30.0)
        .expect("edit f_app");
    let updated_fbds = service.free_body_diagrams();
    assert_eq!(updated_fbds[0].1.state, FrictionState::Kinetic);
    assert!((updated_fbds[0].1.net_force - 15.285).abs() < 1e-3);
}

#[test]
fn test_m4_incline_plane_slide_scenario() {
    use kinema_domain::FrictionState;
    use kinema_ports::{ScenarioCatalog, SceneEditing};

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("incline_plane_slide")
        .expect("load incline");

    // Initially theta = 30 deg => Static
    let fbds = service.free_body_diagrams();
    assert_eq!(fbds[0].1.state, FrictionState::Static);

    // Edit angle to 45 deg => Slides down incline
    service
        .edit_parameter("slider", "theta_deg", 45.0)
        .expect("edit theta");
    let updated_fbds = service.free_body_diagrams();
    assert_eq!(updated_fbds[0].1.state, FrictionState::Kinetic);
    assert!(updated_fbds[0].1.net_force < 0.0);
}

#[test]
fn test_m5_atwood_scenario_and_tensions() {
    use kinema_ports::{ScenarioCatalog, SimulationControl};

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("atwood_machine")
        .expect("load atwood preset");

    let tensions = service.pulley_tensions();
    assert_eq!(tensions.len(), 1);
    assert_eq!(tensions[0].0, "atwood");
    // m1=2, m2=3, g=9.81 => T = 2 * 2 * 3 * 9.81 / 5 = 23.544 N
    assert!((tensions[0].1 - (12.0 * 9.81 / 5.0)).abs() < 1e-4);

    // Step forward 1 second (60 steps)
    for _ in 0..60 {
        service.step_forward();
    }
    assert!((service.current_time() - 1.0).abs() < 1e-6);
    let body = &service.scene().bodies[0];
    let a = (3.0 - 2.0) * 9.81 / 5.0;
    assert!((body.motion.velocity_at(1.0) - a).abs() < 1e-4);
}

#[test]
fn test_m5_table_pulley_scenario_and_parameter_edit() {
    use kinema_ports::{ScenarioCatalog, SceneEditing};

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("table_pulley_friction")
        .expect("load table pulley");

    let tensions = service.pulley_tensions();
    assert_eq!(tensions.len(), 1);

    // Edit hanging mass m2 from 6.0 to 1.0 kg (1.0 * g < 0.5 * 10 * g => transitions to static!)
    service
        .edit_parameter("table_pulley", "m2", 1.0)
        .expect("edit m2 to 1.0");
    let updated_tensions = service.pulley_tensions();
    // Static tension: T = m2 * g = 9.81 N
    assert!((updated_tensions[0].1 - 9.81).abs() < 1e-4);
}

#[test]
fn test_m5_catenary_rope_simulation_step() {
    use kinema_ports::{ScenarioCatalog, SimulationControl};

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("hanging_catenary_rope")
        .expect("load catenary");

    assert_eq!(service.ropes().len(), 1);
    let initial_stretch = service.ropes()[0].stretch_ratio();
    assert!(initial_stretch < 0.02);

    // Advance 120 frames (2.0s wall clock, 480 sub-steps)
    for _ in 0..120 {
        service.step_forward();
    }
    assert!((service.current_time() - 2.0).abs() < 1e-6);
    let final_stretch = service.ropes()[0].stretch_ratio();
    assert!(
        final_stretch < 0.01,
        "Stretch must stay under 1%, got {:.4}%",
        final_stretch * 100.0
    );
}

#[test]
fn test_m5_rope_surface_friction_scenario() {
    use kinema_ports::ScenarioCatalog;

    let scene = Scene::new("Empty");
    let mut service = SimulationService::new(scene);
    service
        .load_scenario("rope_surface_friction")
        .expect("load surface rope");

    let ropes = service.ropes();
    assert_eq!(ropes.len(), 1);
    assert_eq!(ropes[0].surface_y, Some(0.0));
    assert_eq!(ropes[0].friction_mu, 0.4);
}
