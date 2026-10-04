//! Scene model containing bodies and constants.

use crate::motion::Motion;
use crate::rope::ParticleRope;

/// Physical typology of the entity determining its visual model, reference frame, and interactions.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum EntityKind {
    /// Wheeled vehicle rolling on a horizontal surface, assigned to a specific lane offset.
    Vehicle { lane: usize },
    /// Spherical body in free vertical fall towards ground y = 0.
    FreeFall { initial_height: f64 },
    /// Ballistic projectile launched vertically from ground level.
    VerticalProjectile { v0: f64 },
    /// Galileo Apollo 15 dual-drop experiment: steel geological hammer or falcon feather.
    FeatherAndHammer { is_feather: bool },
    /// Industrial wooden crate or friction block subjected to dry Coulomb friction.
    FrictionBlock {
        mass: f64,
        mu_s: f64,
        mu_k: f64,
        f_app: f64,
    },
    /// Sliding block on a triangular inclined wedge with angle theta.
    InclineBlock { angle_rad: f64, incline_length: f64 },
    /// Canonical Atwood machine with two suspended hanging masses.
    AtwoodSystem { m1: f64, m2: f64 },
    /// Table-pulley half-Atwood system with tabletop block and hanging mass.
    TablePulleySystem { m1: f64, m2: f64 },
    /// Generic 1D kinematic point particle.
    #[default]
    Generic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub id: String,
    pub name: String,
    pub motion: Motion,
    pub kind: EntityKind,
}

impl Body {
    pub fn new(id: &str, name: &str, motion: impl Into<Motion>) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            motion: motion.into(),
            kind: EntityKind::Generic,
        }
    }

    pub fn with_kind(mut self, kind: EntityKind) -> Self {
        self.kind = kind;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub name: String,
    pub gravity: f64,
    pub bodies: Vec<Body>,
    pub ropes: Vec<ParticleRope>,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            name: "Default Scene".to_string(),
            gravity: 9.80665,
            bodies: Vec::new(),
            ropes: Vec::new(),
        }
    }
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            gravity: 9.80665,
            bodies: Vec::new(),
            ropes: Vec::new(),
        }
    }

    pub fn add_body(&mut self, body: Body) {
        self.bodies.push(body);
    }

    pub fn add_rope(&mut self, rope: ParticleRope) {
        self.ropes.push(rope);
    }
}
