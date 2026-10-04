use kinema_adapter_cli::CliRunner;
use kinema_domain::motion::Mru;
use kinema_domain::scene::{Body, Scene};

#[test]
fn test_cli_scene_summarize() {
    let mut scene = Scene::new("Test");
    scene.add_body(Body {
        id: "a".to_string(),
        name: "Car 1".to_string(),
        motion: Mru::new(0.0, 10.0),
    });

    let summary = CliRunner::summarize_scene(&scene, 2.0);
    assert_eq!(summary.len(), 2);
    assert!(summary[0].contains("Test"));
    assert!(summary[1].contains("Car 1"));
    assert!(summary[1].contains("20.00 m"));
}
