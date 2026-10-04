//! Driving/Presenter Adapter: UI State, rendering abstractions, and presenter sink.

use kinema_domain::Scene;
use kinema_ports::SnapshotSink;

#[derive(Debug, Clone, PartialEq)]
pub struct UiViewModel {
    pub window_title: String,
    pub current_time: f64,
    pub bodies_count: usize,
    pub status_message: String,
}

impl Default for UiViewModel {
    fn default() -> Self {
        Self {
            window_title: "KINEMA - [Interactive Physics Workbench]".to_string(),
            current_time: 0.0,
            bodies_count: 0,
            status_message: "Ready.".to_string(),
        }
    }
}

pub struct UiPresenter {
    model: UiViewModel,
}

impl UiPresenter {
    pub fn new() -> Self {
        Self {
            model: UiViewModel::default(),
        }
    }
}

impl Default for UiPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl UiPresenter {
    pub fn model(&self) -> &UiViewModel {
        &self.model
    }

    pub fn set_status(&mut self, msg: &str) {
        self.model.status_message = msg.to_string();
    }
}

impl SnapshotSink for UiPresenter {
    fn consume_snapshot(&mut self, scene: &Scene, current_time: f64) {
        self.model.window_title = format!("KINEMA - [{}]", scene.name);
        self.model.current_time = current_time;
        self.model.bodies_count = scene.bodies.len();
        self.model.status_message = format!("Running ({} bodies)", scene.bodies.len());
    }
}
