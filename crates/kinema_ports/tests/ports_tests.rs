use kinema_domain::Scene;
use kinema_ports::SnapshotSink;

struct MockSink {
    received_time: Option<f64>,
}

impl SnapshotSink for MockSink {
    fn consume_snapshot(&mut self, _scene: &Scene, current_time: f64) {
        self.received_time = Some(current_time);
    }
}

#[test]
fn test_snapshot_sink_port() {
    let mut sink = MockSink { received_time: None };
    let scene = Scene::new("Test");
    sink.consume_snapshot(&scene, 1.25);
    assert_eq!(sink.received_time, Some(1.25));
}
