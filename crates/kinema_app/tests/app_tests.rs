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
