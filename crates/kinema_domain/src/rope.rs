//! Particle-chain rope (Stage B: discrete particles with Verlet integration).
//!
//! Continuous string theory is great on paper, but on a computer, discretizing
//! into point masses joined by distance constraints gives us an unconditionally stable,
//! visually convincing rope that can hang, swing, drag over surfaces with friction,
//! and be pulled with external forces.

/// Single point mass node along the particle-chain rope.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RopeNode {
    pub pos: [f64; 2],
    pub prev_pos: [f64; 2],
    pub mass: f64,
    pub pinned: bool,
    pub external_force: [f64; 2],
}

impl RopeNode {
    pub fn new(x: f64, y: f64, mass: f64, pinned: bool) -> Self {
        Self {
            pos: [x, y],
            prev_pos: [x, y],
            mass,
            pinned,
            external_force: [0.0, 0.0],
        }
    }
}

/// Particle-chain rope discretized into N point masses.
#[derive(Debug, Clone, PartialEq)]
pub struct ParticleRope {
    pub nodes: Vec<RopeNode>,
    pub total_length: f64,
    pub total_mass: f64,
    pub gravity: [f64; 2],
    pub friction_mu: f64,
    pub surface_y: Option<f64>,
    pub relaxation_passes: usize,
    pub damping: f64,
}

impl ParticleRope {
    /// Default node count specified in Section 6.5.
    pub const DEFAULT_NODE_COUNT: usize = 24;
    /// Default constraint relaxation passes specified in Section 6.5.
    pub const DEFAULT_RELAXATION_PASSES: usize = 12;

    /// Creates a suspended rope between two coordinates (p0 and p1) with default 24 nodes.
    pub fn new_catenary(p0: [f64; 2], p1: [f64; 2], length: f64, total_mass: f64) -> Result<Self, &'static str> {
        Self::with_node_count(p0, p1, length, total_mass, Self::DEFAULT_NODE_COUNT)
    }

    /// Creates a rope with a configurable node count.
    pub fn with_node_count(
        p0: [f64; 2],
        p1: [f64; 2],
        length: f64,
        total_mass: f64,
        node_count: usize,
    ) -> Result<Self, &'static str> {
        if node_count < 2 {
            return Err("Rope must have at least 2 nodes");
        }
        if !length.is_finite() || length <= 0.0 {
            return Err("Rope length must be positive and finite");
        }
        if !total_mass.is_finite() || total_mass <= 0.0 {
            return Err("Rope mass must be positive and finite");
        }

        let node_mass = total_mass / (node_count as f64);
        let mut nodes = Vec::with_capacity(node_count);

        let chord_sq = (p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2);
        let sag = if length * length > chord_sq {
            (length * length - chord_sq).sqrt() * 0.5
        } else {
            0.0
        };

        for i in 0..node_count {
            let frac = (i as f64) / ((node_count - 1) as f64);
            let x = p0[0] + (p1[0] - p0[0]) * frac;
            let y = p0[1] + (p1[1] - p0[1]) * frac - 4.0 * sag * frac * (1.0 - frac);
            let pinned = i == 0;
            nodes.push(RopeNode::new(x, y, node_mass, pinned));
        }

        let d0 = length / ((node_count - 1) as f64);
        for _ in 0..32 {
            relax_all_segments(&mut nodes, d0);
        }

        Ok(Self {
            nodes,
            total_length: length,
            total_mass,
            gravity: [0.0, -9.80665],
            friction_mu: 0.0,
            surface_y: None,
            relaxation_passes: Self::DEFAULT_RELAXATION_PASSES,
            damping: 0.9995,
        })
    }

    /// Segment rest length between any two adjacent nodes.
    #[inline]
    pub fn rest_segment_length(&self) -> f64 {
        if self.nodes.len() < 2 {
            0.0
        } else {
            self.total_length / ((self.nodes.len() - 1) as f64)
        }
    }

    /// Sets pinning for an arbitrary node index.
    pub fn set_pinned(&mut self, index: usize, pinned: bool) -> Result<(), &'static str> {
        if let Some(node) = self.nodes.get_mut(index) {
            node.pinned = pinned;
            Ok(())
        } else {
            Err("Node index out of bounds")
        }
    }

    /// Enforces distance constraints across all segments for `relaxation_passes` iterations.
    pub fn relax_constraints(&mut self) {
        let d0 = self.rest_segment_length();
        for _ in 0..self.relaxation_passes {
            relax_all_segments(&mut self.nodes, d0);
            if let Some(sy) = self.surface_y {
                clamp_surface(&mut self.nodes, sy);
            }
        }
    }

    /// Sets external pulling force on the end node (node N - 1).
    pub fn set_end_pull_force(&mut self, fx: f64, fy: f64) {
        if let Some(last) = self.nodes.last_mut() {
            if fx.is_finite() && fy.is_finite() {
                last.external_force = [fx, fy];
            }
        }
    }

    /// Enables horizontal surface contact with Coulomb friction at height surface_y.
    pub fn with_surface(mut self, surface_y: f64, mu: f64) -> Self {
        if surface_y.is_finite() && mu.is_finite() && mu >= 0.0 {
            self.surface_y = Some(surface_y);
            self.friction_mu = mu;
        }
        self
    }

    /// Sums Euclidean distances between all adjacent nodes.
    pub fn current_total_length(&self) -> f64 {
        let mut sum = 0.0;
        for i in 0..self.nodes.len().saturating_sub(1) {
            let p0 = self.nodes[i].pos;
            let p1 = self.nodes[i + 1].pos;
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            sum += (dx * dx + dy * dy).sqrt();
        }
        sum
    }

    /// Relative stretch: |L_current - L_rest| / L_rest.
    pub fn stretch_ratio(&self) -> f64 {
        if self.total_length <= 0.0 {
            return 0.0;
        }
        let cur = self.current_total_length();
        (cur - self.total_length).abs() / self.total_length
    }

    /// Tension estimation per segment in Newtons.
    ///
    /// Uses effective constraint restoring force F = m_eff * |d - d0| / dt^2.
    pub fn segment_tensions(&self, dt: f64) -> Vec<f64> {
        let n_segs = self.nodes.len().saturating_sub(1);
        let mut tensions = Vec::with_capacity(n_segs);
        let d0 = self.rest_segment_length();
        let dt_sq = (dt * dt).max(1e-9);

        for i in 0..n_segs {
            let p0 = self.nodes[i].pos;
            let p1 = self.nodes[i + 1].pos;
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            let d = (dx * dx + dy * dy).sqrt();
            let delta = (d - d0).abs();
            let m_eff = self.nodes[i].mass.min(self.nodes[i + 1].mass);
            tensions.push(m_eff * delta / dt_sq);
        }
        tensions
    }

    /// Advances the rope by a fixed timestep dt using Verlet integration and distance constraints.
    pub fn step(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        // 1. Verlet position update for all unpinned nodes
        for node in &mut self.nodes {
            step_node_verlet(node, dt, self.gravity, self.damping);
        }

        // 2. Surface constraint & friction
        if let Some(sy) = self.surface_y {
            for node in &mut self.nodes {
                apply_surface_friction(node, sy, self.friction_mu, dt, self.gravity[1]);
            }
        }

        // 3. Distance constraint relaxation passes
        let d0 = self.rest_segment_length();
        for _ in 0..self.relaxation_passes {
            relax_all_segments(&mut self.nodes, d0);
            if let Some(sy) = self.surface_y {
                clamp_surface(&mut self.nodes, sy);
            }
        }
    }
}

/// Verlet integration for a single node.
fn step_node_verlet(node: &mut RopeNode, dt: f64, gravity: [f64; 2], damping: f64) {
    if node.pinned {
        node.prev_pos = node.pos;
        return;
    }

    let ax = gravity[0] + node.external_force[0] / node.mass;
    let ay = gravity[1] + node.external_force[1] / node.mass;

    let vx = (node.pos[0] - node.prev_pos[0]) * damping;
    let vy = (node.pos[1] - node.prev_pos[1]) * damping;

    let dt2 = dt * dt;
    let next_x = node.pos[0] + vx + ax * dt2;
    let next_y = node.pos[1] + vy + ay * dt2;

    node.prev_pos = node.pos;
    node.pos = [next_x, next_y];
}

/// Applies Coulomb friction when a node touches or drops below the ground surface.
fn apply_surface_friction(node: &mut RopeNode, surface_y: f64, mu: f64, dt: f64, gy: f64) {
    if node.pos[1] <= surface_y {
        node.pos[1] = surface_y;
        if mu > 0.0 {
            let normal_f = (node.mass * gy.abs()).max(0.0);
            let vx = (node.pos[0] - node.prev_pos[0]) / dt;
            let f_fric = -vx.signum() * mu * normal_f;
            let ax_fric = f_fric / node.mass;
            node.pos[0] += ax_fric * dt * dt;
        }
    }
}

/// One relaxation pass over all adjacent segment constraints.
fn relax_all_segments(nodes: &mut [RopeNode], d0: f64) {
    let count = nodes.len();
    if count < 2 {
        return;
    }
    for i in 0..count - 1 {
        relax_pair(nodes, i, i + 1, d0);
    }
}

/// Relaxes a single distance constraint between node i and node j.
fn relax_pair(nodes: &mut [RopeNode], i: usize, j: usize, d0: f64) {
    let p0 = nodes[i].pos;
    let p1 = nodes[j].pos;

    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let dist = (dx * dx + dy * dy).sqrt();

    if dist <= 1e-12 {
        return;
    }

    let delta = dist - d0;
    let nx = dx / dist;
    let ny = dy / dist;

    let p0_pinned = nodes[i].pinned;
    let p1_pinned = nodes[j].pinned;

    match (p0_pinned, p1_pinned) {
        (false, false) => {
            nodes[i].pos[0] += 0.5 * delta * nx;
            nodes[i].pos[1] += 0.5 * delta * ny;
            nodes[j].pos[0] -= 0.5 * delta * nx;
            nodes[j].pos[1] -= 0.5 * delta * ny;
        }
        (true, false) => {
            nodes[j].pos[0] -= delta * nx;
            nodes[j].pos[1] -= delta * ny;
        }
        (false, true) => {
            nodes[i].pos[0] += delta * nx;
            nodes[i].pos[1] += delta * ny;
        }
        (true, true) => {}
    }
}

/// Clamps all nodes so they do not penetrate the floor.
fn clamp_surface(nodes: &mut [RopeNode], surface_y: f64) {
    for node in nodes {
        if node.pos[1] < surface_y {
            node.pos[1] = surface_y;
        }
    }
}
