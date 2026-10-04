//! Scene model containing bodies and constants.

use crate::motion::Motion;

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
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            name: "Default Scene".to_string(),
            gravity: 9.80665,
            bodies: Vec::new(),
        }
    }
}

impl Scene {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            gravity: 9.80665,
            bodies: Vec::new(),
        }
    }

    pub fn add_body(&mut self, body: Body) {
        self.bodies.push(body);
    }
}
