use kinema_app::SimulationService;
use kinema_domain::motion::Mru;
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
    let mut scene = Scene::new("Test Scene");
    scene.add_body(Body {
        id: "car_a".to_string(),
        name: "Car A".to_string(),
        motion: Mru::new(0.0, 10.0),
    });

    let mut service = SimulationService::new(scene);
    assert_eq!(service.scene().bodies[0].motion.v, 10.0);

    service.edit_parameter("car_a", "v", 25.0).unwrap();
    assert_eq!(service.scene().bodies[0].motion.v, 25.0);

    assert!(service.undo());
    assert_eq!(service.scene().bodies[0].motion.v, 10.0);

    assert!(service.redo());
    assert_eq!(service.scene().bodies[0].motion.v, 25.0);
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
