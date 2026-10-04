use kinema_domain::Scene;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cooperative cancellation token for long-running operations like PNG export or batch runs.
#[derive(Debug, Clone, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// Input Port: Simulation control lifecycle.
pub trait SimulationControl {
    fn play(&mut self);
    fn pause(&mut self);
    fn step_forward(&mut self);
    fn step_backward(&mut self);
    fn reset(&mut self);
    fn seek(&mut self, time: f64);
    fn set_speed(&mut self, speed: f64);
}

/// Input Port: Scene editing operations.
pub trait SceneEditing {
    fn edit_parameter(&mut self, body_id: &str, param: &str, value: f64) -> Result<(), String>;
    fn undo(&mut self) -> bool;
    fn redo(&mut self) -> bool;
}

/// Input Port: Built-in scenario catalogue.
pub trait ScenarioCatalog {
    fn list_scenarios(&self) -> Vec<String>;
    fn load_scenario(&mut self, name: &str) -> Result<Scene, String>;
}

/// Output Port: Scene persistence.
pub trait SceneRepository {
    fn load(&self, path: &str) -> Result<Scene, String>;
    fn save(&self, path: &str, scene: &Scene) -> Result<(), String>;
}

/// Output Port: Canvas image export with cooperative cancellation.
pub trait ImageExporter {
    fn export_png(
        &self,
        scene: &Scene,
        time: f64,
        path: &str,
        token: &CancellationToken,
    ) -> Result<(), String>;
}

/// Output Port: Snapshot sink (UI/Presenter sink).
pub trait SnapshotSink {
    fn consume_snapshot(&mut self, scene: &Scene, current_time: f64);
}

