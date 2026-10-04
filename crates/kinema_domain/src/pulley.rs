//! Ideal rope and pulley systems (Stage A: massless, inextensible rope).
//!
//! Real ropes stretch, have inertia, and fray under load. But for high school
//! physics and introductory mechanics, assuming an ideal massless rope saves
//! students from partial differential equations before they have even mastered algebra.

use crate::dynamics::FrictionState;
use crate::motion::{Motion1D, ParametricLaw};

/// Canonical Atwood machine with two hanging masses connected by an ideal rope over a massless pulley.
///
/// Convention: positive coordinate displacement `s` means mass 2 descends and mass 1 ascends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AtwoodMachine {
    pub m1: f64,
    pub m2: f64,
    pub g: f64,
    pub s0: f64,
    pub v0: f64,
}

impl AtwoodMachine {
    /// Constructs a new Atwood machine.
    ///
    /// Rejects non-positive masses and non-finite values because physics engines
    /// divide by total mass, and negative mass belongs in sci-fi.
    pub fn new(m1: f64, m2: f64) -> Result<Self, &'static str> {
        if !m1.is_finite() || m1 <= 0.0 || !m2.is_finite() || m2 <= 0.0 {
            return Err("Atwood masses must be strictly positive and finite");
        }
        Ok(Self {
            m1,
            m2,
            g: 9.80665,
            s0: 0.0,
            v0: 0.0,
        })
    }

    pub fn with_gravity(mut self, g: f64) -> Self {
        if g.is_finite() && g >= 0.0 {
            self.g = g;
        }
        self
    }

    pub fn with_initial_state(mut self, s0: f64, v0: f64) -> Self {
        if s0.is_finite() {
            self.s0 = s0;
        }
        if v0.is_finite() {
            self.v0 = v0;
        }
        self
    }

    /// Analytic acceleration: a = (m2 - m1) * g / (m1 + m2).
    #[inline]
    pub fn acceleration(&self) -> f64 {
        (self.m2 - self.m1) * self.g / (self.m1 + self.m2)
    }

    /// Ideal rope tension: T = 2 * m1 * m2 * g / (m1 + m2).
    #[inline]
    pub fn tension(&self) -> f64 {
        (2.0 * self.m1 * self.m2 * self.g) / (self.m1 + self.m2)
    }

    /// Vertical coordinate for mass 1 (left hanging body; ascends when s > 0).
    #[inline]
    pub fn mass1_position_at(&self, t: f64) -> f64 {
        -self.position_at(t)
    }

    /// Vertical coordinate for mass 2 (right hanging body; descends when s > 0).
    #[inline]
    pub fn mass2_position_at(&self, t: f64) -> f64 {
        self.position_at(t)
    }

    /// Fixed-timestep Semi-Implicit Euler stepper for numerical validation.
    pub fn step_euler(&self, s: f64, v: f64, dt: f64) -> (f64, f64, f64) {
        let a = self.acceleration();
        let next_v = v + a * dt;
        let next_s = s + next_v * dt;
        (next_s, next_v, a)
    }
}

impl Motion1D for AtwoodMachine {
    fn position_at(&self, t: f64) -> f64 {
        let a = self.acceleration();
        self.s0 + self.v0 * t + 0.5 * a * t * t
    }

    fn velocity_at(&self, t: f64) -> f64 {
        self.v0 + self.acceleration() * t
    }

    fn acceleration_at(&self, _t: f64) -> f64 {
        self.acceleration()
    }
}

impl ParametricLaw for AtwoodMachine {
    fn formula_text(&self) -> String {
        format!(
            "a = (m₂ - m₁)·g / (m₁ + m₂) = {:.3} m/s², T = 2·m₁·m₂·g / (m₁ + m₂) = {:.2} N",
            self.acceleration(),
            self.tension()
        )
    }

    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("m1", self.m1),
            ("m2", self.m2),
            ("g", self.g),
            ("s0", self.s0),
            ("v0", self.v0),
        ]
    }

    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str> {
        if !value.is_finite() {
            return Err("Parameter value must be finite");
        }
        match name {
            "m1" if value > 0.0 => self.m1 = value,
            "m2" if value > 0.0 => self.m2 = value,
            "g" if value >= 0.0 => self.g = value,
            "s0" => self.s0 = value,
            "v0" => self.v0 = value,
            "m1" | "m2" => return Err("Mass must be strictly positive"),
            "g" => return Err("Gravity cannot be negative"),
            _ => return Err("Unknown parameter for AtwoodMachine"),
        }
        Ok(())
    }
}

/// Block on a horizontal table (m1) connected via an ideal pulley to a hanging mass (m2).
///
/// Accounts for Coulomb static and kinetic friction on the table surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TablePulleySystem {
    pub m1: f64,
    pub m2: f64,
    pub mu_s: f64,
    pub mu_k: f64,
    pub g: f64,
    pub s0: f64,
    pub v0: f64,
}

impl TablePulleySystem {
    /// Constructs a table pulley system.
    ///
    /// Rejects non-positive masses and negative friction coefficients.
    pub fn new(m1: f64, m2: f64, mu_s: f64, mu_k: f64) -> Result<Self, &'static str> {
        if !m1.is_finite() || m1 <= 0.0 || !m2.is_finite() || m2 <= 0.0 {
            return Err("Pulley system masses must be strictly positive and finite");
        }
        if !mu_s.is_finite() || mu_s < 0.0 || !mu_k.is_finite() || mu_k < 0.0 {
            return Err("Friction coefficients must be non-negative and finite");
        }
        Ok(Self {
            m1,
            m2,
            mu_s,
            mu_k,
            g: 9.80665,
            s0: 0.0,
            v0: 0.0,
        })
    }

    pub fn with_gravity(mut self, g: f64) -> Self {
        if g.is_finite() && g >= 0.0 {
            self.g = g;
        }
        self
    }

    pub fn with_initial_state(mut self, s0: f64, v0: f64) -> Self {
        if s0.is_finite() {
            self.s0 = s0;
        }
        if v0.is_finite() {
            self.v0 = v0;
        }
        self
    }

    /// Evaluates if the table block stays static at rest: m2 * g <= mu_s * m1 * g.
    #[inline]
    pub fn is_static_at_rest(&self) -> bool {
        self.v0 == 0.0 && (self.m2 * self.g <= self.mu_s * self.m1 * self.g)
    }

    /// Current friction regime for the system.
    pub fn friction_state(&self) -> FrictionState {
        if self.is_static_at_rest() {
            FrictionState::Static
        } else {
            FrictionState::Kinetic
        }
    }

    /// System acceleration:
    /// - If static: 0.0
    /// - If kinetic: (m2 * g - mu_k * m1 * g) / (m1 + m2)
    pub fn acceleration(&self) -> f64 {
        if self.is_static_at_rest() {
            0.0
        } else {
            let num = self.m2 * self.g - self.mu_k * self.m1 * self.g;
            num / (self.m1 + self.m2)
        }
    }

    /// Rope tension:
    /// - If static: T = m2 * g
    /// - If kinetic: T = m1 * (a + mu_k * g)
    pub fn tension(&self) -> f64 {
        if self.is_static_at_rest() {
            self.m2 * self.g
        } else {
            let a = self.acceleration();
            self.m1 * (a + self.mu_k * self.g)
        }
    }

    /// Table friction force magnitude:
    /// - If static: balances m2 * g exactly
    /// - If kinetic: mu_k * m1 * g
    pub fn friction_force(&self) -> f64 {
        if self.is_static_at_rest() {
            self.m2 * self.g
        } else {
            self.mu_k * self.m1 * self.g
        }
    }

    /// Numerical Semi-Implicit Euler stepper for fixed-dt simulation.
    pub fn step_euler(&self, s: f64, v: f64, dt: f64) -> (f64, f64, f64, FrictionState) {
        let is_static = v == 0.0 && (self.m2 * self.g <= self.mu_s * self.m1 * self.g);
        if is_static {
            (s, 0.0, 0.0, FrictionState::Static)
        } else {
            let a = (self.m2 * self.g - self.mu_k * self.m1 * self.g) / (self.m1 + self.m2);
            let next_v = v + a * dt;
            let next_s = s + next_v * dt;
            (next_s, next_v, a, FrictionState::Kinetic)
        }
    }
}

impl Motion1D for TablePulleySystem {
    fn position_at(&self, t: f64) -> f64 {
        let a = self.acceleration();
        self.s0 + self.v0 * t + 0.5 * a * t * t
    }

    fn velocity_at(&self, t: f64) -> f64 {
        self.v0 + self.acceleration() * t
    }

    fn acceleration_at(&self, _t: f64) -> f64 {
        self.acceleration()
    }
}

impl ParametricLaw for TablePulleySystem {
    fn formula_text(&self) -> String {
        format!(
            "a = (m₂·g - μ_k·m₁·g)/(m₁ + m₂) = {:.3} m/s², T = {:.2} N [{:?}]",
            self.acceleration(),
            self.tension(),
            self.friction_state()
        )
    }

    fn parameters(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("m1", self.m1),
            ("m2", self.m2),
            ("mu_s", self.mu_s),
            ("mu_k", self.mu_k),
            ("g", self.g),
            ("s0", self.s0),
            ("v0", self.v0),
        ]
    }

    fn set_parameter(&mut self, name: &str, value: f64) -> Result<(), &'static str> {
        if !value.is_finite() {
            return Err("Parameter value must be finite");
        }
        match name {
            "m1" if value > 0.0 => self.m1 = value,
            "m2" if value > 0.0 => self.m2 = value,
            "mu_s" if value >= 0.0 => self.mu_s = value,
            "mu_k" if value >= 0.0 => self.mu_k = value,
            "g" if value >= 0.0 => self.g = value,
            "s0" => self.s0 = value,
            "v0" => self.v0 = value,
            "m1" | "m2" => return Err("Mass must be strictly positive"),
            "mu_s" | "mu_k" => return Err("Friction coefficients cannot be negative"),
            "g" => return Err("Gravity cannot be negative"),
            _ => return Err("Unknown parameter for TablePulleySystem"),
        }
        Ok(())
    }
}
