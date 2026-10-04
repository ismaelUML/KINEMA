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
