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
