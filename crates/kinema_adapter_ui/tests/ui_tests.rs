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
    scene.add_body(Body::new("car_a", "Car A", Mru::new(0.0, 15.0)));
    scene.add_body(Body::new("car_b", "Car B", Mru::new(100.0, -10.0)));

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

#[test]
fn test_ui_presenter_m2_mruv_stopping_and_velocity_graph() {
    use kinema_adapter_ui::GraphKind;
    use kinema_domain::motion::{Mru, Mruv};
    use kinema_domain::scene::Body;

    let mut presenter = UiPresenter::new();
    let mut scene = Scene::new("MRU vs MRUV");
    scene.add_body(Body::new("car_a", "Car A", Mru::new(0.0, 15.0)));
    scene.add_body(Body::new("car_b", "Car B", Mruv::new(100.0, -10.0, 2.0)));

    presenter.set_graph_kind(GraphKind::VelocityTime);
    presenter.consume_snapshot(&scene, 5.0);

    let model = presenter.model();
    assert_eq!(model.bodies.len(), 2);
    assert_eq!(
        model.bodies[1].formula_text,
        "x(t) = 100.00 - 10.00 · t + 1.00 · t²"
    );
    assert_eq!(model.bodies[1].stopping_time, Some(5.0));

    // Two roots meeting markers (t=5s and t=20s)
    assert_eq!(model.meeting_markers.len(), 2);
    assert_eq!(model.meeting_markers[0].time, 5.0);
    assert_eq!(model.meeting_markers[1].time, 20.0);

    // Stopping marker
    assert_eq!(model.stopping_markers.len(), 1);
    assert_eq!(model.stopping_markers[0].time, 5.0);
    assert_eq!(model.stopping_markers[0].position, 75.0);

    // Velocity graph series
    assert_eq!(model.graph_series.len(), 2);
    assert_eq!(model.active_graph_kind, GraphKind::VelocityTime);
}

#[test]
fn test_ui_presenter_m3_20m_drop_impact_marker() {
    use kinema_domain::motion::Mvl;
    use kinema_domain::scene::Body;

    let mut presenter = UiPresenter::new();
    let mut scene = Scene::new("20m Free Fall");
    scene.add_body(Body::new(
        "ball",
        "Dropping Ball",
        Mvl::new(20.0, 0.0, 9.81),
    ));

    presenter.consume_snapshot(&scene, 1.0);

    let model = presenter.model();
    assert_eq!(model.bodies.len(), 1);
    assert!(model.bodies[0].formula_text.contains("y(t) = 20.00"));
    assert_eq!(model.apex_markers.len(), 0);

    // Impact marker
    assert_eq!(model.impact_markers.len(), 1);
    let marker = &model.impact_markers[0];
    assert_eq!(marker.body_id, "ball");
    assert!((marker.time - 2.01927).abs() < 1e-4);
    assert!((marker.speed - 19.809).abs() < 1e-3);
    assert!(marker.label.contains("impact (Dropping Ball at t=2.02s"));
}

#[test]
fn test_ui_presenter_m3_vertical_projectile_apex_and_impact() {
    use kinema_domain::motion::Mvl;
    use kinema_domain::scene::Body;

    let mut presenter = UiPresenter::new();
    let mut scene = Scene::new("Vertical Projectile");
    scene.add_body(Body::new(
        "rock",
        "Launched Rock",
        Mvl::new(0.0, 20.0, 9.81),
    ));

    presenter.consume_snapshot(&scene, 0.0);

    let model = presenter.model();
    assert_eq!(model.apex_markers.len(), 1);
    let apex = &model.apex_markers[0];
    assert_eq!(apex.body_id, "rock");
    assert!((apex.time - (20.0 / 9.81)).abs() < 1e-4);
    assert!((apex.height - (400.0 / 19.62)).abs() < 1e-4);
    assert!(apex.label.contains("apex (Launched Rock at t=2.04s"));

    assert_eq!(model.impact_markers.len(), 1);
    let impact = &model.impact_markers[0];
    assert_eq!(impact.body_id, "rock");
    assert!((impact.time - (40.0 / 9.81)).abs() < 1e-4);
    assert!((impact.speed - 20.0).abs() < 1e-4);
}

#[test]
fn test_ui_presenter_m4_dynamics_fbd() {
    use kinema_domain::dynamics::BlockDynamics;
    use kinema_domain::scene::Body;

    let mut presenter = UiPresenter::new();
    let mut scene = Scene::new("FBD Inspection");
    let block = BlockDynamics::horizontal(5.0, 0.5, 0.3, 20.0).with_gravity(9.81);
    scene.add_body(Body::new("box", "Test Box", block));

    presenter.consume_snapshot(&scene, 0.0);

    let model = presenter.model();
    assert_eq!(model.fbd_views.len(), 1);
    let fbd_view = &model.fbd_views[0];
    assert_eq!(fbd_view.body_id, "box");
    assert_eq!(fbd_view.friction_state, "STATIC");
    assert_eq!(fbd_view.net_force, 0.0);
    assert_eq!(fbd_view.arrows.len(), 4);
}
