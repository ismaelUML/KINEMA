//! Scene model containing bodies and constants.

use crate::motion::Motion;
use crate::rope::ParticleRope;

#[derive(Debug, Clone, PartialEq)]
pub struct Body {
    pub id: String,
    pub name: String,
    pub motion: Motion,
}

impl Body {
    pub fn new(id: &str, name: &str, motion: impl Into<Motion>) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            motion: motion.into(),
        }
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
