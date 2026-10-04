//! 1D Motion abstraction and implementations.

/// Trait representing a one-dimensional motion law.
pub trait Motion1D {
    /// Returns position at time t.
    fn position_at(&self, t: f64) -> f64;
    /// Returns velocity at time t.
    fn velocity_at(&self, t: f64) -> f64;
    /// Returns acceleration at time t.
    fn acceleration_at(&self, t: f64) -> f64;
}

/// Uniform Rectilinear Motion (MRU): x(t) = x0 + v * t
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mru {
    pub x0: f64,
    pub v: f64,
}

impl Mru {
    pub fn new(x0: f64, v: f64) -> Self {
        Self { x0, v }
    }
}

impl Motion1D for Mru {
    fn position_at(&self, t: f64) -> f64 {
        self.x0 + self.v * t
    }

    fn velocity_at(&self, _t: f64) -> f64 {
        self.v
    }

    fn acceleration_at(&self, _t: f64) -> f64 {
        0.0
    }
}

/// Allows UI panels and serializers to inspect and mutate parameters without
/// hardcoding matching logic for every motion type out there.
pub trait ParametricLaw {
    /// Formats the law as mathematical text. Handles negative signs so we don't display 'x(t) = 0 + -10 * t'.
    fn formula_text(&self) -> String;
    /// Names and values of the parameters exposed to editors.
    fn parameters(&self) -> Vec<(&'static str, f64)>;
    /// Mutates a single parameter. Returns Err if someone mistypes the parameter key.
    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str>;
}

impl ParametricLaw for Mru {
    fn formula_text(&self) -> String {
        // We branch on velocity sign specifically to avoid ugly '+ -10.0' in equations shown to users.
        if self.v >= 0.0 {
            format!("x(t) = {:.2} + {:.2} · t", self.x0, self.v)
        } else {
            format!("x(t) = {:.2} - {:.2} · t", self.x0, self.v.abs())
        }
    }

    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![("x0", self.x0), ("v", self.v)]
    }

    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str> {
        match name {
            "x0" => {
                self.x0 = value;
                Ok(())
            }
            "v" => {
                self.v = value;
                Ok(())
            }
            _ => Err("unknown parameter"),
        }
    }
}

/// Uniformly Varied Rectilinear Motion (MRUV): x(t) = x0 + v0 * t + 0.5 * a * t^2
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mruv {
    pub x0: f64,
    pub v0: f64,
    pub a: f64,
}

impl Mruv {
    pub fn new(x0: f64, v0: f64, a: f64) -> Self {
        Self { x0, v0, a }
    }

    /// Stopping instant: t_s = -v_0 / a, only when v_0 and a have opposite signs.
    pub fn stopping_time(&self) -> Option<f64> {
        const EPS: f64 = 1e-12;
        if self.a.abs() < EPS {
            return None;
        }
        if self.v0 * self.a < 0.0 {
            Some(-self.v0 / self.a)
        } else {
            None
        }
    }

    /// Stopping distance: Delta x = -v0^2 / (2a)
    pub fn stopping_distance(&self) -> Option<f64> {
        self.stopping_time().map(|t| self.position_at(t) - self.x0)
    }
}

impl Motion1D for Mruv {
    fn position_at(&self, t: f64) -> f64 {
        self.x0 + self.v0 * t + 0.5 * self.a * t * t
    }

    fn velocity_at(&self, t: f64) -> f64 {
        self.v0 + self.a * t
    }

    fn acceleration_at(&self, _t: f64) -> f64 {
        self.a
    }
}

impl ParametricLaw for Mruv {
    fn formula_text(&self) -> String {
        let half_a = 0.5 * self.a;
        let v_sign = if self.v0 >= 0.0 { "+" } else { "-" };
        let a_sign = if half_a >= 0.0 { "+" } else { "-" };
        format!(
            "x(t) = {:.2} {} {:.2} · t {} {:.2} · t²",
            self.x0,
            v_sign,
            self.v0.abs(),
            a_sign,
            half_a.abs()
        )
    }

    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![("x0", self.x0), ("v0", self.v0), ("a", self.a)]
    }

    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str> {
        match name {
            "x0" => {
                self.x0 = value;
                Ok(())
            }
            "v" | "v0" => {
                self.v0 = value;
                Ok(())
            }
            "a" => {
                self.a = value;
                Ok(())
            }
            _ => Err("unknown parameter"),
        }
    }
}

/// Planetary gravitational acceleration presets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GravityPreset {
    pub name: &'static str,
    pub g: f64,
}

impl GravityPreset {
    pub const EARTH: f64 = 9.80665;
    pub const EARTH_STANDARD: f64 = 9.81;
    pub const MOON: f64 = 1.62;
    pub const MARS: f64 = 3.71;
    pub const JUPITER: f64 = 24.79;

    pub fn all() -> &'static [GravityPreset] {
        &[
            GravityPreset {
                name: "Earth",
                g: Self::EARTH_STANDARD,
            },
            GravityPreset {
                name: "Moon",
                g: Self::MOON,
            },
            GravityPreset {
                name: "Mars",
                g: Self::MARS,
            },
            GravityPreset {
                name: "Jupiter",
                g: Self::JUPITER,
            },
        ]
    }
}

/// Free Vertical Motion (MVL / Caída Libre y Tiro Vertical): y(t) = y0 + v0 * t - 0.5 * g * t^2
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mvl {
    pub y0: f64,
    pub v0: f64,
    pub g: f64,
}

impl Mvl {
    pub fn new(y0: f64, v0: f64, g: f64) -> Self {
        Self { y0, v0, g }
    }

    /// Time to apex when thrown upwards: t_up = v0 / g
    pub fn time_to_apex(&self) -> Option<f64> {
        if self.g > 0.0 && self.v0 > 0.0 {
            Some(self.v0 / self.g)
        } else {
            None
        }
    }

    /// Maximum height reached: h_max = y0 + v0^2 / (2g)
    pub fn max_height(&self) -> f64 {
        if let Some(t_up) = self.time_to_apex() {
            self.position_at(t_up)
        } else {
            self.y0
        }
    }

    /// Ground impact instant (y = 0): t_i = (v0 + sqrt(v0^2 + 2*g*y0)) / g
    pub fn impact_instant(&self) -> Option<f64> {
        if self.g <= 0.0 {
            return None;
        }
        let discriminant = self.v0 * self.v0 + 2.0 * self.g * self.y0;
        if discriminant >= 0.0 {
            let t = (self.v0 + discriminant.sqrt()) / self.g;
            if t >= 0.0 {
                return Some(t);
            }
        }
        None
    }

    /// Impact speed magnitude: |v_i| = sqrt(v0^2 + 2*g*y0)
    pub fn impact_speed(&self) -> Option<f64> {
        if self.g <= 0.0 {
            return None;
        }
        let discriminant = self.v0 * self.v0 + 2.0 * self.g * self.y0;
        if discriminant >= 0.0 {
            Some(discriminant.sqrt())
        } else {
            None
        }
    }
}

impl Motion1D for Mvl {
    fn position_at(&self, t: f64) -> f64 {
        self.y0 + self.v0 * t - 0.5 * self.g * t * t
    }

    fn velocity_at(&self, t: f64) -> f64 {
        self.v0 - self.g * t
    }

    fn acceleration_at(&self, _t: f64) -> f64 {
        -self.g
    }
}

impl ParametricLaw for Mvl {
    fn formula_text(&self) -> String {
        let half_g = 0.5 * self.g;
        let v_sign = if self.v0 >= 0.0 { "+" } else { "-" };
        format!(
            "y(t) = {:.2} {} {:.2} · t - {:.2} · t²",
            self.y0,
            v_sign,
            self.v0.abs(),
            half_g
        )
    }

    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![("y0", self.y0), ("v0", self.v0), ("g", self.g)]
    }

    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str> {
        match name {
            "y0" | "x0" => {
                self.y0 = value;
                Ok(())
            }
            "v0" | "v" => {
                self.v0 = value;
                Ok(())
            }
            "g" => {
                if value < 0.0 {
                    return Err("gravity must be non-negative");
                }
                self.g = value;
                Ok(())
            }
            _ => Err("unknown parameter"),
        }
    }
}

use crate::dynamics::{BlockDynamics, FreeBodyDiagram};

/// Dynamic motion discriminator supporting MRU, MRUV, MVL, and Dynamics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motion {
    Mru(Mru),
    Mruv(Mruv),
    Mvl(Mvl),
    Dynamics(BlockDynamics),
}

impl Motion {
    pub fn stopping_time(&self) -> Option<f64> {
        match self {
            Motion::Mru(_) => None,
            Motion::Mruv(m) => m.stopping_time(),
            Motion::Mvl(m) => m.time_to_apex(),
            Motion::Dynamics(d) => d.stopping_time(),
        }
    }

    /// Free vertical motion apex instant (time_to_apex, max_height) if upward launch exists.
    pub fn apex(&self) -> Option<(f64, f64)> {
        match self {
            Motion::Mvl(m) => m.time_to_apex().map(|t| (t, m.max_height())),
            _ => None,
        }
    }

    /// Free vertical motion ground impact instant (impact_time, impact_speed).
    pub fn ground_impact(&self) -> Option<(f64, f64)> {
        match self {
            Motion::Mvl(m) => match (m.impact_instant(), m.impact_speed()) {
                (Some(t), Some(v)) => Some((t, v)),
                _ => None,
            },
            _ => None,
        }
    }

    /// Free-Body Diagram snapshot for dynamic bodies.
    pub fn free_body_diagram(&self, v: f64) -> Option<FreeBodyDiagram> {
        match self {
            Motion::Dynamics(d) => Some(d.free_body_diagram(v)),
            _ => None,
        }
    }
}

impl Motion1D for Motion {
    fn position_at(&self, t: f64) -> f64 {
        match self {
            Motion::Mru(m) => m.position_at(t),
            Motion::Mruv(m) => m.position_at(t),
            Motion::Mvl(m) => m.position_at(t),
            Motion::Dynamics(d) => d.position_at(t),
        }
    }

    fn velocity_at(&self, t: f64) -> f64 {
        match self {
            Motion::Mru(m) => m.velocity_at(t),
            Motion::Mruv(m) => m.velocity_at(t),
            Motion::Mvl(m) => m.velocity_at(t),
            Motion::Dynamics(d) => d.velocity_at(t),
        }
    }

    fn acceleration_at(&self, t: f64) -> f64 {
        match self {
            Motion::Mru(m) => m.acceleration_at(t),
            Motion::Mruv(m) => m.acceleration_at(t),
            Motion::Mvl(m) => m.acceleration_at(t),
            Motion::Dynamics(d) => d.acceleration_at(t),
        }
    }
}

impl ParametricLaw for Motion {
    fn formula_text(&self) -> String {
        match self {
            Motion::Mru(m) => m.formula_text(),
            Motion::Mruv(m) => m.formula_text(),
            Motion::Mvl(m) => m.formula_text(),
            Motion::Dynamics(d) => d.formula_text(),
        }
    }

    fn parameters(&self) -> Vec<(&'static str, f64)> {
        match self {
            Motion::Mru(m) => m.parameters(),
            Motion::Mruv(m) => m.parameters(),
            Motion::Mvl(m) => m.parameters(),
            Motion::Dynamics(d) => d.parameters(),
        }
    }

    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str> {
        match self {
            Motion::Mru(m) => m.set_parameter(name, value),
            Motion::Mruv(m) => m.set_parameter(name, value),
            Motion::Mvl(m) => m.set_parameter(name, value),
            Motion::Dynamics(d) => d.set_parameter(name, value),
        }
    }
}

impl From<Mru> for Motion {
    fn from(m: Mru) -> Self {
        Motion::Mru(m)
    }
}

impl From<Mruv> for Motion {
    fn from(m: Mruv) -> Self {
        Motion::Mruv(m)
    }
}

impl From<Mvl> for Motion {
    fn from(m: Mvl) -> Self {
        Motion::Mvl(m)
    }
}

impl From<BlockDynamics> for Motion {
    fn from(d: BlockDynamics) -> Self {
        Motion::Dynamics(d)
    }
}
