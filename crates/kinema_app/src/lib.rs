//! Application Layer / Use Case Orchestration.

use kinema_domain::{
    analyze_meeting, AtwoodMachine, BlockDynamics, Body, EntityKind, FreeBodyDiagram,
    GravityPreset, MeetingOutcome, Motion, Motion1D, Mru, Mruv, Mvl, ParametricLaw, ParticleRope,
    Scene, TablePulleySystem,
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

    /// Finds all bodies with an apex instant (t_up, h_max) under vertical projectile motion.
    pub fn find_apex_instants(&self) -> Vec<(String, f64, f64)> {
        self.scene
            .bodies
            .iter()
            .filter_map(|b| {
                b.motion
                    .apex()
                    .map(|(t_up, h_max)| (b.id.clone(), t_up, h_max))
            })
            .collect()
    }

    /// Finds all bodies with a ground impact instant (t_impact, |v_impact|) at y = 0.
    pub fn find_ground_impacts(&self) -> Vec<(String, f64, f64)> {
        self.scene
            .bodies
            .iter()
            .filter_map(|b| {
                b.motion
                    .ground_impact()
                    .map(|(t_i, v_i)| (b.id.clone(), t_i, v_i))
            })
            .collect()
    }

    /// Evaluates the Free-Body Diagram snapshot for all dynamic bodies at the current simulation time.
    pub fn free_body_diagrams(&self) -> Vec<(String, FreeBodyDiagram)> {
        self.scene
            .bodies
            .iter()
            .filter_map(|b| {
                let v = b.motion.velocity_at(self.time);
                b.motion.free_body_diagram(v).map(|fbd| (b.id.clone(), fbd))
            })
            .collect()
    }

    /// Evaluates rope tension for all bodies operating ideal pulley systems.
    pub fn pulley_tensions(&self) -> Vec<(String, f64)> {
        self.scene
            .bodies
            .iter()
            .filter_map(|b| b.motion.pulley_tension().map(|t| (b.id.clone(), t)))
            .collect()
    }

    /// Accesses all particle-chain ropes in the scene.
    pub fn ropes(&self) -> &[ParticleRope] {
        &self.scene.ropes
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
        let substep = 1.0 / 240.0;
        for _ in 0..4 {
            for rope in &mut self.scene.ropes {
                rope.step(substep);
            }
        }
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
            "20m_free_fall".to_string(),
            "feather_and_hammer_moon".to_string(),
            "vertical_projectile".to_string(),
            "block_friction_threshold".to_string(),
            "incline_plane_slide".to_string(),
            "heavy_crate_push".to_string(),
            "atwood_machine".to_string(),
            "table_pulley_friction".to_string(),
            "hanging_catenary_rope".to_string(),
            "rope_surface_friction".to_string(),
        ]
    }

    fn load_scenario(&mut self, name: &str) -> Result<Scene, String> {
        let scene = resolve_scenario(name).ok_or_else(|| format!("Unknown scenario '{}'", name))?;
        self.push_undo();
        self.time = 0.0;
        self.scene = scene.clone();
        Ok(scene)
    }
}

fn resolve_scenario(name: &str) -> Option<Scene> {
    match name {
        "two_cars_mru" | "Two cars meeting" => Some(build_two_cars_scenario()),
        "two_cars_mruv" | "Two cars meeting (MRU vs MRUV)" => Some(build_two_cars_mruv_scenario()),
        "parallel_cars" => Some(build_parallel_scenario()),
        "coinciding_cars" => Some(build_coinciding_scenario()),
        "20m_free_fall" | "Free fall 20m drop (Earth)" => Some(build_20m_free_fall_scenario()),
        "feather_and_hammer_moon" | "Feather and Hammer (Moon)" => {
            Some(build_feather_hammer_scenario())
        }
        "vertical_projectile" | "Vertical projectile launch (Earth)" => {
            Some(build_vertical_projectile_scenario())
        }
        "block_friction_threshold" | "Block with friction threshold" => {
            Some(build_block_friction_scenario())
        }
        "incline_plane_slide" | "Incline plane sliding angle" => {
            Some(build_incline_slide_scenario())
        }
        "heavy_crate_push" | "Heavy crate push" => Some(build_heavy_crate_scenario()),
        "atwood_machine" | "Atwood machine" => Some(build_atwood_scenario()),
        "table_pulley_friction" | "Table pulley with friction" => {
            Some(build_table_pulley_scenario())
        }
        "hanging_catenary_rope" | "Hanging catenary rope" => Some(build_catenary_rope_scenario()),
        "rope_surface_friction" | "Rope on surface with friction" => {
            Some(build_rope_friction_scenario())
        }
        _ => None,
    }
}

fn build_two_cars_scenario() -> Scene {
    let mut scene = Scene::new("Two cars meeting");
    scene.add_body(
        Body::new("car_a", "Car A (15 m/s)", Mru::new(0.0, 15.0))
            .with_kind(EntityKind::Vehicle { lane: 0 }),
    );
    scene.add_body(
        Body::new("car_b", "Car B (-10 m/s)", Mru::new(100.0, -10.0))
            .with_kind(EntityKind::Vehicle { lane: 1 }),
    );
    scene
}

fn build_two_cars_mruv_scenario() -> Scene {
    let mut scene = Scene::new("Two cars meeting (MRU vs MRUV)");
    scene.add_body(
        Body::new("car_a", "Car A (MRU 15 m/s)", Mru::new(0.0, 15.0))
            .with_kind(EntityKind::Vehicle { lane: 0 }),
    );
    // Canonical M2 Two-Roots Problem: Car B has constant acceleration a = +2 m/s².
    // It decelerates to v=0 at t=5s (first meeting) and accelerates rightwards to second meeting at t=20s.
    scene.add_body(
        Body::new(
            "car_b",
            "Car B (MRUV v0=-10, a=+2, Braking)",
            Mruv::braking(100.0, -10.0, 2.0),
        )
        .with_kind(EntityKind::Vehicle { lane: 1 }),
    );
    scene
}

fn build_parallel_scenario() -> Scene {
    let mut scene = Scene::new("Parallel cars never meeting");
    scene.add_body(
        Body::new("car_a", "Car A", Mru::new(0.0, 20.0)).with_kind(EntityKind::Vehicle { lane: 0 }),
    );
    scene.add_body(
        Body::new("car_b", "Car B", Mru::new(50.0, 20.0))
            .with_kind(EntityKind::Vehicle { lane: 1 }),
    );
    scene
}

fn build_coinciding_scenario() -> Scene {
    let mut scene = Scene::new("Coinciding cars");
    // Assigning separate lanes prevents Car B from completely occluding Car A in 2D space.
    scene.add_body(
        Body::new("car_a", "Car A", Mru::new(25.0, 10.0))
            .with_kind(EntityKind::Vehicle { lane: 0 }),
    );
    scene.add_body(
        Body::new("car_b", "Car B", Mru::new(25.0, 10.0))
            .with_kind(EntityKind::Vehicle { lane: 1 }),
    );
    scene
}

fn build_20m_free_fall_scenario() -> Scene {
    let mut scene = Scene::new("Free fall 20m drop (Earth)");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    scene.add_body(
        Body::new(
            "ball",
            "Dropping Ball (20m)",
            Mvl::new(20.0, 0.0, g).with_ground_stop(true),
        )
        .with_kind(EntityKind::FreeFall {
            initial_height: 20.0,
        }),
    );
    scene
}

fn build_feather_hammer_scenario() -> Scene {
    let mut scene = Scene::new("Feather and Hammer (Moon)");
    let g = GravityPreset::MOON;
    scene.gravity = g;
    scene.add_body(
        Body::new(
            "hammer",
            "Geological Hammer",
            Mvl::new(1.62, 0.0, g).with_ground_stop(true),
        )
        .with_kind(EntityKind::FeatherAndHammer { is_feather: false }),
    );
    scene.add_body(
        Body::new(
            "feather",
            "Falcon Feather",
            Mvl::new(1.62, 0.0, g).with_ground_stop(true),
        )
        .with_kind(EntityKind::FeatherAndHammer { is_feather: true }),
    );
    scene
}

fn build_vertical_projectile_scenario() -> Scene {
    let mut scene = Scene::new("Vertical projectile launch (Earth)");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    scene.add_body(
        Body::new(
            "rock",
            "Launched Rock (v0=20 m/s)",
            Mvl::new(0.0, 20.0, g).with_ground_stop(true),
        )
        .with_kind(EntityKind::VerticalProjectile { v0: 20.0 }),
    );
    scene
}

fn build_block_friction_scenario() -> Scene {
    let mut scene = Scene::new("Block with friction threshold");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    let block = BlockDynamics::horizontal(5.0, 0.5, 0.3, 20.0).with_gravity(g);
    scene.add_body(
        Body::new("block", "Block (5kg, μs=0.5, μk=0.3, F=20N)", block).with_kind(
            EntityKind::FrictionBlock {
                mass: 5.0,
                mu_s: 0.5,
                mu_k: 0.3,
                f_app: 20.0,
            },
        ),
    );
    scene
}

fn build_incline_slide_scenario() -> Scene {
    let mut scene = Scene::new("Incline plane sliding angle");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    let theta = 30.0_f64.to_radians();
    let block = BlockDynamics::new(2.0, theta, 0.6, 0.4).with_gravity(g);
    scene.add_body(
        Body::new("slider", "Block on 30° Incline (μs=0.6, μk=0.4)", block).with_kind(
            EntityKind::InclineBlock {
                angle_rad: theta,
                incline_length: 50.0,
            },
        ),
    );
    scene
}

fn build_heavy_crate_scenario() -> Scene {
    let mut scene = Scene::new("Heavy crate push");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    let crate_body = BlockDynamics::horizontal(50.0, 0.4, 0.25, 250.0).with_gravity(g);
    scene.add_body(
        Body::new("crate", "Heavy Crate (50kg, F=250N)", crate_body).with_kind(
            EntityKind::FrictionBlock {
                mass: 50.0,
                mu_s: 0.4,
                mu_k: 0.25,
                f_app: 250.0,
            },
        ),
    );
    scene
}

fn build_atwood_scenario() -> Scene {
    let mut scene = Scene::new("Atwood Machine (2kg vs 3kg)");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    let atwood = AtwoodMachine::new(2.0, 3.0)
        .expect("valid masses")
        .with_gravity(g);
    scene.add_body(
        Body::new("atwood", "Atwood Machine (m1=2kg, m2=3kg)", atwood)
            .with_kind(EntityKind::AtwoodSystem { m1: 2.0, m2: 3.0 }),
    );
    scene
}

fn build_table_pulley_scenario() -> Scene {
    let mut scene = Scene::new("Table Pulley with Friction");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    let system = TablePulleySystem::new(10.0, 6.0, 0.5, 0.3)
        .expect("valid system")
        .with_gravity(g);
    scene.add_body(
        Body::new(
            "table_pulley",
            "Table Pulley (m1=10kg, m2=6kg, μs=0.5, μk=0.3)",
            system,
        )
        .with_kind(EntityKind::TablePulleySystem { m1: 10.0, m2: 6.0 }),
    );
    scene
}

fn build_catenary_rope_scenario() -> Scene {
    let mut scene = Scene::new("Hanging Catenary Rope");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    let mut rope =
        ParticleRope::new_catenary([0.0, 3.0], [2.0, 3.0], 2.5, 1.0).expect("valid catenary");
    let _ = rope.set_pinned(23, true);
    rope.relax_constraints();
    scene.add_rope(rope);
    scene
}

fn build_rope_friction_scenario() -> Scene {
    let mut scene = Scene::new("Rope on Surface with Friction");
    let g = GravityPreset::EARTH_STANDARD;
    scene.gravity = g;
    let rope = ParticleRope::new_catenary([0.0, 1.0], [2.0, 1.0], 2.5, 1.0)
        .expect("valid rope")
        .with_surface(0.0, 0.4);
    scene.add_rope(rope);
    scene
}
