//! Module M4: Dynamics with Coulomb Friction on Horizontal Surfaces and Inclined Planes.
//!
//! Handles Newton's second law (\Sigma F = m * a), static-to-kinetic threshold transitions,
//! normal force calculations on slopes, and both exact piecewise closed-form evaluation
//! and fixed-timestep Semi-Implicit Euler integration.

use crate::motion::{Motion1D, ParametricLaw};

/// State of contact surface friction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrictionState {
    /// Net driving force does not exceed maximum static friction (fs_max = mu_s * N).
    #[default]
    Static,
    /// Body is sliding; friction opposes velocity or impending motion with magnitude fk = mu_k * N.
    Kinetic,
}

/// Free-Body Diagram snapshot summarizing all forces acting on the block.
#[derive(Debug, Clone, PartialEq)]
pub struct FreeBodyDiagram {
    /// Gravitational force magnitude (W = m * g).
    pub weight: f64,
    /// Normal contact force perpendicular to the surface (N = m * g * cos(theta)).
    pub normal: f64,
    /// Friction force along the incline/surface (signed scalar; opposes motion or drive).
    pub friction: f64,
    /// Applied external force along the incline/surface (positive up-slope, negative down-slope).
    pub applied_force: f64,
    /// Component of gravity parallel to the incline (-m * g * sin(theta)).
    pub gravity_parallel: f64,
    /// Resulting net force along the direction of motion (\Sigma F).
    pub net_force: f64,
    /// Current friction regime (Static or Kinetic).
    pub state: FrictionState,
}

/// Dynamics model for a rigid block on a horizontal or inclined surface with Coulomb friction.
/// Coordinate convention: coordinate x is directed along the surface (up the incline).
/// Angle theta is in radians; theta = 0 corresponds to a flat horizontal plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockDynamics {
    pub mass: f64,
    pub theta: f64,
    pub mu_s: f64,
    pub mu_k: f64,
    pub f_app: f64,
    pub gravity: f64,
    pub x0: f64,
    pub v0: f64,
}

impl Default for BlockDynamics {
    fn default() -> Self {
        Self {
            mass: 1.0,
            theta: 0.0,
            mu_s: 0.5,
            mu_k: 0.3,
            f_app: 0.0,
            gravity: 9.80665,
            x0: 0.0,
            v0: 0.0,
        }
    }
}

impl BlockDynamics {
    pub fn new(mass: f64, theta: f64, mu_s: f64, mu_k: f64) -> Self {
        Self {
            mass: if mass > 0.0 { mass } else { 1.0 },
            theta: theta.clamp(0.0, std::f64::consts::FRAC_PI_2 - 1e-6),
            mu_s: mu_s.max(0.0),
            mu_k: mu_k.max(0.0).min(mu_s.max(0.0)),
            f_app: 0.0,
            gravity: 9.80665,
            x0: 0.0,
            v0: 0.0,
        }
    }

    pub fn horizontal(mass: f64, mu_s: f64, mu_k: f64, f_app: f64) -> Self {
        let mut block = Self::new(mass, 0.0, mu_s, mu_k);
        block.f_app = f_app;
        block
    }

    pub fn with_applied_force(mut self, f_app: f64) -> Self {
        self.f_app = f_app;
        self
    }

    pub fn with_initial_state(mut self, x0: f64, v0: f64) -> Self {
        self.x0 = x0;
        self.v0 = v0;
        self
    }

    pub fn with_gravity(mut self, gravity: f64) -> Self {
        self.gravity = gravity.max(0.0);
        self
    }

    /// Total gravitational force W = m * g.
    pub fn weight(&self) -> f64 {
        self.mass * self.gravity
    }

    /// Normal force perpendicular to the surface N = m * g * cos(theta).
    pub fn normal_force(&self) -> f64 {
        self.mass * self.gravity * self.theta.cos()
    }

    /// Gravity component along the incline: directed down the slope, so negative along +x.
    pub fn gravity_parallel(&self) -> f64 {
        -self.mass * self.gravity * self.theta.sin()
    }

    /// Net active driving force along the slope before friction: F_act = F_app - m * g * sin(theta).
    pub fn active_driving_force(&self) -> f64 {
        self.f_app + self.gravity_parallel()
    }

    /// Maximum static friction holding threshold: fs_max = mu_s * N.
    pub fn max_static_friction(&self) -> f64 {
        self.mu_s * self.normal_force()
    }

    /// Kinetic friction magnitude: fk = mu_k * N.
    pub fn kinetic_friction_magnitude(&self) -> f64 {
        self.mu_k * self.normal_force()
    }

    /// Evaluates friction force and friction state for a given instantaneous velocity.
    pub fn evaluate_friction(&self, v: f64) -> (f64, FrictionState) {
        if v.abs() < 1e-9 {
            self.eval_static_friction()
        } else {
            self.eval_kinetic_friction(v)
        }
    }

    fn eval_static_friction(&self) -> (f64, FrictionState) {
        let f_act = self.active_driving_force();
        let fs_max = self.max_static_friction();
        if f_act.abs() <= fs_max {
            (-f_act, FrictionState::Static)
        } else {
            let dir = if f_act > 0.0 { 1.0 } else { -1.0 };
            (
                -dir * self.kinetic_friction_magnitude(),
                FrictionState::Kinetic,
            )
        }
    }

    fn eval_kinetic_friction(&self, v: f64) -> (f64, FrictionState) {
        let dir = if v > 0.0 { 1.0 } else { -1.0 };
        (
            -dir * self.kinetic_friction_magnitude(),
            FrictionState::Kinetic,
        )
    }

    /// Constructs the complete Free-Body Diagram at instantaneous velocity v.
    pub fn free_body_diagram(&self, v: f64) -> FreeBodyDiagram {
        let (friction, state) = self.evaluate_friction(v);
        let f_act = self.active_driving_force();
        let net_force = if state == FrictionState::Static {
            0.0
        } else {
            f_act + friction
        };

        FreeBodyDiagram {
            weight: self.weight(),
            normal: self.normal_force(),
            friction,
            applied_force: self.f_app,
            gravity_parallel: self.gravity_parallel(),
            net_force,
            state,
        }
    }

    /// Instantaneous acceleration for given velocity: a = \Sigma F / m.
    pub fn acceleration_at_velocity(&self, v: f64) -> f64 {
        self.free_body_diagram(v).net_force / self.mass
    }

    /// Stopping instant if the block is sliding and decelerating towards rest.
    pub fn stopping_time(&self) -> Option<f64> {
        if self.v0.abs() < 1e-9 {
            return None;
        }
        let dir = if self.v0 > 0.0 { 1.0 } else { -1.0 };
        let fk = dir * self.kinetic_friction_magnitude();
        let a1 = (self.active_driving_force() - fk) / self.mass;
        if (a1 > 0.0 && self.v0 < 0.0) || (a1 < 0.0 && self.v0 > 0.0) {
            Some(-self.v0 / a1)
        } else {
            None
        }
    }

    /// Single-step Semi-Implicit (Symplectic) Euler integrator with Coulomb stick-slip zero crossing.
    /// Returns: (next_position, next_velocity, next_acceleration, next_friction_state).
    pub fn step_euler(&self, pos: f64, vel: f64, dt: f64) -> (f64, f64, f64, FrictionState) {
        let fbd = self.free_body_diagram(vel);
        let a = fbd.net_force / self.mass;
        let next_v_cand = vel + a * dt;

        if is_stick_zero_crossing(
            vel,
            next_v_cand,
            self.active_driving_force(),
            self.max_static_friction(),
        ) {
            (pos, 0.0, 0.0, FrictionState::Static)
        } else {
            let next_v = next_v_cand;
            let next_pos = pos + next_v * dt;
            let next_fbd = self.free_body_diagram(next_v);
            (
                next_pos,
                next_v,
                next_fbd.net_force / self.mass,
                next_fbd.state,
            )
        }
    }
}

/// Helper function to prevent numerical friction oscillation across v = 0.
/// If friction alone decelerates the block past zero and active force is within static limit, block sticks.
fn is_stick_zero_crossing(v_curr: f64, v_next: f64, f_act: f64, fs_max: f64) -> bool {
    v_curr.abs() > 1e-9 && (v_curr > 0.0) != (v_next > 0.0) && f_act.abs() <= fs_max
}

impl Motion1D for BlockDynamics {
    fn position_at(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return self.x0;
        }
        eval_closed_form_kinematics(self, t).0
    }

    fn velocity_at(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return self.v0;
        }
        eval_closed_form_kinematics(self, t).1
    }

    fn acceleration_at(&self, t: f64) -> f64 {
        if t <= 0.0 {
            return self.acceleration_at_velocity(self.v0);
        }
        eval_closed_form_kinematics(self, t).2
    }
}

/// Evaluates closed-form piecewise motion under constant applied force and Coulomb friction.
/// Returns (position, velocity, acceleration).
fn eval_closed_form_kinematics(dyn_model: &BlockDynamics, t: f64) -> (f64, f64, f64) {
    if dyn_model.v0.abs() < 1e-9 {
        eval_from_rest_kinematics(dyn_model, t)
    } else {
        eval_moving_kinematics(dyn_model, t)
    }
}

fn eval_from_rest_kinematics(dyn_model: &BlockDynamics, t: f64) -> (f64, f64, f64) {
    let f_act = dyn_model.active_driving_force();
    let fs_max = dyn_model.max_static_friction();

    if f_act.abs() <= fs_max {
        // Remainder at rest indefinitely under static equilibrium
        (dyn_model.x0, 0.0, 0.0)
    } else {
        let dir = if f_act > 0.0 { 1.0 } else { -1.0 };
        let fk = dir * dyn_model.kinetic_friction_magnitude();
        let a = (f_act - fk) / dyn_model.mass;
        let pos = dyn_model.x0 + 0.5 * a * t * t;
        let vel = a * t;
        (pos, vel, a)
    }
}

fn eval_moving_kinematics(dyn_model: &BlockDynamics, t: f64) -> (f64, f64, f64) {
    let dir = if dyn_model.v0 > 0.0 { 1.0 } else { -1.0 };
    let fk = dir * dyn_model.kinetic_friction_magnitude();
    let a1 = (dyn_model.active_driving_force() - fk) / dyn_model.mass;

    // Check if motion decelerates toward rest
    if (a1 > 0.0 && dyn_model.v0 < 0.0) || (a1 < 0.0 && dyn_model.v0 > 0.0) {
        let t_stop = -dyn_model.v0 / a1;
        if t < t_stop {
            (
                dyn_model.x0 + dyn_model.v0 * t + 0.5 * a1 * t * t,
                dyn_model.v0 + a1 * t,
                a1,
            )
        } else {
            let x_stop = dyn_model.x0 + dyn_model.v0 * t_stop + 0.5 * a1 * t_stop * t_stop;
            let remaining_t = t - t_stop;
            let mut from_stop = *dyn_model;
            from_stop.x0 = x_stop;
            from_stop.v0 = 0.0;
            eval_from_rest_kinematics(&from_stop, remaining_t)
        }
    } else {
        (
            dyn_model.x0 + dyn_model.v0 * t + 0.5 * a1 * t * t,
            dyn_model.v0 + a1 * t,
            a1,
        )
    }
}

impl ParametricLaw for BlockDynamics {
    fn formula_text(&self) -> String {
        let fbd = self.free_body_diagram(self.v0);
        match fbd.state {
            FrictionState::Static => {
                format!(
                    "Static: F_act = {:.2} N <= fs_max = {:.2} N => a = 0.00 m/s²",
                    self.active_driving_force().abs(),
                    self.max_static_friction()
                )
            }
            FrictionState::Kinetic => {
                let a = fbd.net_force / self.mass;
                format!(
                    "Kinetic: ΣF = {:.2} N, m = {:.2} kg => a = {:.2} m/s²",
                    fbd.net_force, self.mass, a
                )
            }
        }
    }

    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("mass", self.mass),
            ("theta_deg", self.theta.to_degrees()),
            ("mu_s", self.mu_s),
            ("mu_k", self.mu_k),
            ("f_app", self.f_app),
            ("gravity", self.gravity),
            ("x0", self.x0),
            ("v0", self.v0),
        ]
    }

    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str> {
        match name {
            "mass" | "m" => {
                if value <= 0.0 {
                    return Err("mass must be strictly positive");
                }
                self.mass = value;
                Ok(())
            }
            "theta" | "angle" | "theta_deg" => {
                let rad = if name == "theta_deg" {
                    value.to_radians()
                } else {
                    value
                };
                if rad < 0.0 || rad >= std::f64::consts::FRAC_PI_2 {
                    return Err("angle must be in [0, 90) degrees");
                }
                self.theta = rad;
                Ok(())
            }
            "mu_s" => {
                if value < 0.0 {
                    return Err("mu_s cannot be negative");
                }
                self.mu_s = value;
                if self.mu_k > self.mu_s {
                    self.mu_k = self.mu_s;
                }
                Ok(())
            }
            "mu_k" => {
                if value < 0.0 {
                    return Err("mu_k cannot be negative");
                }
                self.mu_k = value.min(self.mu_s);
                Ok(())
            }
            "f_app" | "f" => {
                self.f_app = value;
                Ok(())
            }
            "gravity" | "g" => {
                if value < 0.0 {
                    return Err("gravity cannot be negative");
                }
                self.gravity = value;
                Ok(())
            }
            "x0" => {
                self.x0 = value;
                Ok(())
            }
            "v0" | "v" => {
                self.v0 = value;
                Ok(())
            }
            _ => Err("unknown parameter"),
        }
    }
}
