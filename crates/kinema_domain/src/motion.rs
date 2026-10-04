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

/// Dynamic motion discriminator supporting both MRU and MRUV.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Motion {
    Mru(Mru),
    Mruv(Mruv),
}

impl Motion {
    pub fn stopping_time(&self) -> Option<f64> {
        match self {
            Motion::Mru(_) => None,
            Motion::Mruv(m) => m.stopping_time(),
        }
    }
}

impl Motion1D for Motion {
    fn position_at(&self, t: f64) -> f64 {
        match self {
            Motion::Mru(m) => m.position_at(t),
            Motion::Mruv(m) => m.position_at(t),
        }
    }

    fn velocity_at(&self, t: f64) -> f64 {
        match self {
            Motion::Mru(m) => m.velocity_at(t),
            Motion::Mruv(m) => m.velocity_at(t),
        }
    }

    fn acceleration_at(&self, t: f64) -> f64 {
        match self {
            Motion::Mru(m) => m.acceleration_at(t),
            Motion::Mruv(m) => m.acceleration_at(t),
        }
    }
}

impl ParametricLaw for Motion {
    fn formula_text(&self) -> String {
        match self {
            Motion::Mru(m) => m.formula_text(),
            Motion::Mruv(m) => m.formula_text(),
        }
    }

    fn parameters(&self) -> Vec<(&'static str, f64)> {
        match self {
            Motion::Mru(m) => m.parameters(),
            Motion::Mruv(m) => m.parameters(),
        }
    }

    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str> {
        match self {
            Motion::Mru(m) => m.set_parameter(name, value),
            Motion::Mruv(m) => m.set_parameter(name, value),
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
