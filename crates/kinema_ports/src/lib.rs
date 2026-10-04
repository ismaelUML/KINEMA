//! Port interfaces (Input and Output boundaries) for KINEMA.

use kinema_domain::Scene;

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

/// Output Port: Snapshot sink (UI/Presenter sink).
pub trait SnapshotSink {
    fn consume_snapshot(&mut self, scene: &Scene, current_time: f64);
}
