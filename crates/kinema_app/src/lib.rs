//! Application Layer / Use Case Orchestration.

use kinema_domain::{
    analyze_meeting, Body, MeetingOutcome, Motion, Motion1D, Mru, Mruv, ParametricLaw, Scene,
};
use kinema_ports::{ScenarioCatalog, SceneEditing, SimulationControl};

/// Kinematic curve type for graphing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveType {
    Position,
    Velocity,
    Acceleration,
}

/// Sample point on a trajectory graph (time, value).
#[derive(Debug, Clone, PartialEq)]
pub struct TrajectoryPoint {
    pub time: f64,
    pub position: f64,
}

/// Sampled trajectory series for a specific body.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyTrajectory {
    pub body_id: String,
    pub points: Vec<TrajectoryPoint>,
}

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

    /// Evaluates meeting between the first two bodies in the scene, if present.
    pub fn solve_two_body_meeting(&self) -> Option<MeetingOutcome> {
        // If we don't even have two bodies, there's nothing to collide or compare.
        if self.scene.bodies.len() < 2 {
            return None;
        }
        let body_a = &self.scene.bodies[0];
        let body_b = &self.scene.bodies[1];
        Some(analyze_meeting(&body_a.motion, &body_b.motion))
    }

    /// Finds all bodies with a finite turnaround/stopping instant.
    pub fn find_stopping_instants(&self) -> Vec<(String, f64, f64)> {
        self.scene
            .bodies
            .iter()
            .filter_map(|b| {
                b.motion
                    .stopping_time()
                    .map(|ts| (b.id.clone(), ts, b.motion.position_at(ts)))
            })
            .collect()
    }

    /// Samples kinematic curve (position, velocity, or acceleration) across [t_start, t_end].
    pub fn sample_curve(
        &self,
        body_id: &str,
        curve: CurveType,
        t_start: f64,
        t_end: f64,
        num_points: usize,
    ) -> Result<BodyTrajectory, String> {
        let body = self
            .scene
            .bodies
            .iter()
            .find(|b| b.id == body_id)
            .ok_or_else(|| format!("Body with ID '{}' not found", body_id))?;

        let count = num_points.max(2);
        let step = (t_end - t_start) / (count - 1) as f64;
        let mut points = Vec::with_capacity(count);

        for i in 0..count {
            let t = t_start + i as f64 * step;
            let val = eval_motion_curve(&body.motion, curve, t);
            points.push(TrajectoryPoint {
                time: t,
                position: val,
            });
        }

        Ok(BodyTrajectory {
            body_id: body_id.to_string(),
            points,
        })
    }

    /// Backward-compatible alias for sampling position trajectory.
    pub fn sample_trajectory(
        &self,
        body_id: &str,
        t_start: f64,
        t_end: f64,
        num_points: usize,
    ) -> Result<BodyTrajectory, String> {
        self.sample_curve(body_id, CurveType::Position, t_start, t_end, num_points)
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() >= Self::MAX_UNDO_LIMIT {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(self.scene.clone());
        self.redo_stack.clear();
    }
}

fn eval_motion_curve(motion: &Motion, curve: CurveType, t: f64) -> f64 {
    match curve {
        CurveType::Position => motion.position_at(t),
        CurveType::Velocity => motion.velocity_at(t),
        CurveType::Acceleration => motion.acceleration_at(t),
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
                return body
                    .motion
                    .set_parameter(param, value)
                    .map_err(|e| format!("Failed to set {}: {}", param, e));
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

impl ScenarioCatalog for SimulationService {
    fn list_scenarios(&self) -> Vec<String> {
        vec![
            "two_cars_mru".to_string(),
            "two_cars_mruv".to_string(),
            "parallel_cars".to_string(),
            "coinciding_cars".to_string(),
        ]
    }

    fn load_scenario(&mut self, name: &str) -> Result<Scene, String> {
        let scene = match name {
            "two_cars_mru" | "Two cars meeting" => build_two_cars_scenario(),
            "two_cars_mruv" | "Two cars meeting (MRU vs MRUV)" => build_two_cars_mruv_scenario(),
            "parallel_cars" => build_parallel_scenario(),
            "coinciding_cars" => build_coinciding_scenario(),
            _ => return Err(format!("Unknown scenario '{}'", name)),
        };
        self.push_undo();
        self.time = 0.0;
        self.scene = scene.clone();
        Ok(scene)
    }
}

fn build_two_cars_scenario() -> Scene {
    let mut scene = Scene::new("Two cars meeting");
    scene.add_body(Body::new("car_a", "Car A (15 m/s)", Mru::new(0.0, 15.0)));
    scene.add_body(Body::new(
        "car_b",
        "Car B (-10 m/s)",
        Mru::new(100.0, -10.0),
    ));
    scene
}

fn build_two_cars_mruv_scenario() -> Scene {
    let mut scene = Scene::new("Two cars meeting (MRU vs MRUV)");
    scene.add_body(Body::new(
        "car_a",
        "Car A (MRU 15 m/s)",
        Mru::new(0.0, 15.0),
    ));
    scene.add_body(Body::new(
        "car_b",
        "Car B (MRUV v0=-10, a=+2)",
        Mruv::new(100.0, -10.0, 2.0),
    ));
    scene
}

fn build_parallel_scenario() -> Scene {
    let mut scene = Scene::new("Parallel cars never meeting");
    scene.add_body(Body::new("car_a", "Car A", Mru::new(0.0, 20.0)));
    scene.add_body(Body::new("car_b", "Car B", Mru::new(50.0, 20.0)));
    scene
}

fn build_coinciding_scenario() -> Scene {
    let mut scene = Scene::new("Coinciding cars");
    scene.add_body(Body::new("car_a", "Car A", Mru::new(25.0, 10.0)));
    scene.add_body(Body::new("car_b", "Car B", Mru::new(25.0, 10.0)));
    scene
}
