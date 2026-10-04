use kinema_adapter_ui::UiPresenter;
use kinema_domain::scene::Scene;
use kinema_ports::SnapshotSink;

#[test]
fn test_ui_presenter_updates_model() {
    let mut presenter = UiPresenter::new();
    assert_eq!(presenter.model().status_message, "Ready.");

    let scene = Scene::new("Test Scene");
    presenter.consume_snapshot(&scene, 3.5);

    assert_eq!(presenter.model().current_time, 3.5);
    assert_eq!(presenter.model().window_title, "KINEMA - [Test Scene]");
}

#[test]
fn test_ui_presenter_m1_formulas_markers_and_graphs() {
    use kinema_domain::motion::Mru;
    use kinema_domain::scene::Body;

    let mut presenter = UiPresenter::new();
    let mut scene = Scene::new("Two cars meeting");
    scene.add_body(Body {
        id: "car_a".to_string(),
        name: "Car A".to_string(),
        motion: Mru::new(0.0, 15.0),
    });
    scene.add_body(Body {
        id: "car_b".to_string(),
        name: "Car B".to_string(),
        motion: Mru::new(100.0, -10.0),
    });

    presenter.consume_snapshot(&scene, 2.0);

    let model = presenter.model();
    assert_eq!(model.bodies.len(), 2);
    assert_eq!(model.bodies[0].formula_text, "x(t) = 0.00 + 15.00 · t");
    assert_eq!(model.bodies[1].formula_text, "x(t) = 100.00 - 10.00 · t");
    assert_eq!(model.bodies[0].current_position, 30.0);
    assert_eq!(model.bodies[1].current_position, 80.0);

    // Meeting marker
    assert_eq!(model.meeting_markers.len(), 1);
    assert_eq!(model.meeting_markers[0].time, 4.0);
    assert_eq!(model.meeting_markers[0].position, 60.0);
    assert!(!model.meeting_markers[0].is_past);
    assert_eq!(
        model.meeting_markers[0].label,
        "meeting (t=4.00s, x=60.00m)"
    );

    // Graph series
    assert_eq!(model.graph_series.len(), 2);
    assert_eq!(model.graph_series[0].points.len(), 11);
    assert_eq!(model.graph_series[1].points.len(), 11);
}
