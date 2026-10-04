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

#[test]
fn test_ui_presenter_m5_pulley_views() {
    use kinema_domain::pulley::{AtwoodMachine, TablePulleySystem};
    use kinema_domain::scene::Body;

    let mut presenter = UiPresenter::new();
    let mut scene = Scene::new("Pulley Systems");
    let atwood = AtwoodMachine::new(2.0, 3.0).unwrap().with_gravity(9.81);
    let table = TablePulleySystem::new(10.0, 6.0, 0.5, 0.3)
        .unwrap()
        .with_gravity(9.81);

    scene.add_body(Body::new("atwood", "Atwood", atwood));
    scene.add_body(Body::new("table", "Table Pulley", table));

    presenter.consume_snapshot(&scene, 1.0);

    let model = presenter.model();
    assert_eq!(model.pulley_views.len(), 2);

    let atwood_view = &model.pulley_views[0];
    assert_eq!(atwood_view.system_type, "Atwood Machine");
    assert!((atwood_view.tension - 23.544).abs() < 1e-4);
    assert!((atwood_view.acceleration - 1.962).abs() < 1e-4);

    let table_view = &model.pulley_views[1];
    assert_eq!(table_view.system_type, "Table Pulley");
    assert!((table_view.tension - 47.82375).abs() < 1e-4);
}

#[test]
fn test_ui_presenter_m5_rope_views_and_color_ramp() {
    use kinema_adapter_ui::tension_to_color_hex;
    use kinema_domain::rope::ParticleRope;

    // Test color ramp helper
    assert_eq!(tension_to_color_hex(0.0), "#0000FF"); // pure blue
    assert_eq!(tension_to_color_hex(1.0), "#FF0000"); // pure red

    let mut presenter = UiPresenter::new();
    let mut scene = Scene::new("Rope Canvas");
    let rope = ParticleRope::new_catenary([0.0, 3.0], [2.0, 3.0], 2.5, 1.0).unwrap();
    scene.add_rope(rope);

    presenter.consume_snapshot(&scene, 0.0);

    let model = presenter.model();
    assert_eq!(model.rope_views.len(), 1);
    let rope_view = &model.rope_views[0];
    assert_eq!(rope_view.node_count, 24);
    assert_eq!(rope_view.segments.len(), 23);

    // Each segment has valid coordinates and non-empty hex color string
    for seg in &rope_view.segments {
        assert!(seg.color_hex.starts_with('#'));
        assert_eq!(seg.color_hex.len(), 7);
        assert!(seg.tension_ratio >= 0.0 && seg.tension_ratio <= 1.0);
    }
}

#[test]
fn test_ui_presenter_m6_themes_palette_swap() {
    use kinema_adapter_ui::UiTheme;

    let mut presenter = UiPresenter::new();
    let scene = Scene::new("Themed Scene");

    // 1. Default Classic Theme
    presenter.consume_snapshot(&scene, 0.0);
    assert_eq!(presenter.model().active_theme, UiTheme::Classic);
    assert_eq!(presenter.model().palette.window_face, "#C0C0C0");
    assert_eq!(presenter.model().palette.title_bg, "#000080");

    // 2. Phosphor Theme (#33FF33 on black)
    presenter.set_theme(UiTheme::Phosphor);
    presenter.consume_snapshot(&scene, 0.0);
    assert_eq!(presenter.model().active_theme, UiTheme::Phosphor);
    assert_eq!(presenter.model().palette.window_face, "#000000");
    assert_eq!(presenter.model().palette.text_primary, "#33FF33");

    // 3. Amber Theme (#FFB000 on black)
    presenter.set_theme(UiTheme::Amber);
    presenter.consume_snapshot(&scene, 0.0);
    assert_eq!(presenter.model().active_theme, UiTheme::Amber);
    assert_eq!(presenter.model().palette.window_face, "#000000");
    assert_eq!(presenter.model().palette.text_primary, "#FFB000");
}

#[test]
fn test_ui_presenter_m6_help_system_dialogs() {
    use kinema_adapter_ui::HelpTopic;

    let mut presenter = UiPresenter::new();

    // 1. F1 Contents
    presenter.open_help(HelpTopic::Contents);
    assert!(presenter.model().help_dialog_open);
    assert_eq!(presenter.model().current_help_topic, Some(HelpTopic::Contents));
    assert!(presenter.model().help_text.as_ref().unwrap().contains("Keyboard Shortcuts"));

    // 2. Equation Reference
    presenter.open_help(HelpTopic::EquationReference);
    assert!(presenter.model().help_text.as_ref().unwrap().contains("M1 - MRU"));
    assert!(presenter.model().help_text.as_ref().unwrap().contains("M5 - Atwood Machine"));

    // 3. About Dialog
    presenter.open_help(HelpTopic::About);
    assert!(presenter.model().help_text.as_ref().unwrap().contains("Old but functional"));

    // 4. Close Dialog
    presenter.close_help();
    assert!(!presenter.model().help_dialog_open);
    assert!(presenter.model().help_text.is_none());
}

#[test]
fn test_ui_presenter_m6_menu_structure() {
    let presenter = UiPresenter::new();
    let menus = &presenter.model().menus;

    assert_eq!(menus.len(), 6);
    let titles: Vec<&str> = menus.iter().map(|m| m.title.as_str()).collect();
    assert_eq!(titles, vec!["File", "Edit", "View", "Simulate", "Scene", "Help"]);

    let file_menu = &menus[0];
    let file_actions: Vec<&str> = file_menu.items.iter().map(|i| i.action_id.as_str()).collect();
    assert!(file_actions.contains(&"file.new"));
    assert!(file_actions.contains(&"file.export_png"));

    let help_menu = &menus[5];
    let help_actions: Vec<&str> = help_menu.items.iter().map(|i| i.action_id.as_str()).collect();
    assert!(help_actions.contains(&"help.contents"));
    assert!(help_actions.contains(&"help.equations"));
}

