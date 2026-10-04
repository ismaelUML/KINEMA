//! Application Layer / Use Case Orchestration.

use kinema_domain::Scene;
use kinema_ports::{SceneEditing, SimulationControl};

pub struct SimulationService {
    scene: Scene,
    time: f64,
    speed: f64,
    is_running: bool,
    undo_stack: Vec<Scene>,
    redo_stack: Vec<Scene>,
}

impl SimulationService {
    pub const MAX_UNDO_LIMIT: usize = 200;

    pub fn new(scene: Scene) -> Self {
        Self {
            scene,
            time: 0.0,
            speed: 1.0,
            is_running: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn current_time(&self) -> f64 {
        self.time
    }

    pub fn is_playing(&self) -> bool {
        self.is_running
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() >= Self::MAX_UNDO_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.scene.clone());
        self.redo_stack.clear();
    }
}

impl SimulationControl for SimulationService {
    fn play(&mut self) {
        self.is_running = true;
    }

    fn pause(&mut self) {
        self.is_running = false;
    }

    fn step_forward(&mut self) {
        self.time += 1.0 / 60.0;
    }

    fn step_backward(&mut self) {
        self.time = (self.time - 1.0 / 60.0).max(0.0);
    }

    fn reset(&mut self) {
        self.time = 0.0;
        self.is_running = false;
    }

    fn seek(&mut self, time: f64) {
        self.time = time.max(0.0);
    }

    fn set_speed(&mut self, speed: f64) {
        self.speed = speed.clamp(0.1, 10.0);
    }
}

impl SceneEditing for SimulationService {
    fn edit_parameter(&mut self, body_id: &str, param: &str, value: f64) -> Result<(), String> {
        self.push_undo();
        for body in &mut self.scene.bodies {
            if body.id == body_id {
                match param {
                    "x0" => body.motion.x0 = value,
                    "v" => body.motion.v = value,
                    _ => return Err(format!("Unknown parameter {}", param)),
                }
                return Ok(());
            }
        }
        Err(format!("Body with ID {} not found", body_id))
    }

    fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.scene.clone());
            self.scene = prev;
            true
        } else {
            false
        }
    }

    fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.scene.clone());
            self.scene = next;
            true
        } else {
            false
        }
    }
}
