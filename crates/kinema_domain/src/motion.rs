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
