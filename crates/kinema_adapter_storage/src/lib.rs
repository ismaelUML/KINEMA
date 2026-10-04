use kinema_domain::dynamics::BlockDynamics;
use kinema_domain::motion::{Motion, Mru, Mruv, Mvl};
use kinema_domain::pulley::{AtwoodMachine, TablePulleySystem};
use kinema_domain::rope::ParticleRope;
use kinema_domain::scene::{Body, Scene};
use kinema_ports::SceneRepository;
use std::fs;
use std::io::Write;

pub const MAX_FILE_SIZE_BYTES: u64 = 1024 * 1024; // 1 MiB
pub const MAX_BODIES_COUNT: usize = 256;

#[derive(Default)]
pub struct KinFileStorage;

#[derive(Default)]
struct BodyDraft {
    id: String,
    motion_type: String,
    x0: f64,
    v: f64,
    a: f64,
    g: f64,
    mass: f64,
    m1: f64,
    m2: f64,
    theta: f64,
    mu_s: f64,
    mu_k: f64,
    f_app: f64,
}

impl BodyDraft {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            motion_type: "mru".to_string(),
            x0: 0.0,
            v: 0.0,
            a: 0.0,
            g: 9.80665,
            mass: 1.0,
            m1: 1.0,
            m2: 1.0,
            theta: 0.0,
            mu_s: 0.5,
            mu_k: 0.3,
            f_app: 0.0,
        }
    }

    fn into_body(self) -> Result<Body, String> {
        let motion: Motion = match self.motion_type.as_str() {
            "dynamics" => BlockDynamics::new(self.mass, self.theta, self.mu_s, self.mu_k)
                .with_applied_force(self.f_app)
                .with_initial_state(self.x0, self.v)
                .with_gravity(self.g)
                .into(),
            "atwood" => AtwoodMachine::new(self.m1, self.m2)
                .map_err(|e| format!("Invalid Atwood machine: {}", e))?
                .with_gravity(self.g)
                .with_initial_state(self.x0, self.v)
                .into(),
            "table_pulley" => TablePulleySystem::new(self.m1, self.m2, self.mu_s, self.mu_k)
                .map_err(|e| format!("Invalid table pulley: {}", e))?
                .with_gravity(self.g)
                .with_initial_state(self.x0, self.v)
                .into(),
            "mvl" => Mvl::new(self.x0, self.v, self.g).into(),
            "mruv" => Mruv::new(self.x0, self.v, self.a).into(),
            _ => {
                if self.a.abs() > 1e-12 {
                    Mruv::new(self.x0, self.v, self.a).into()
                } else {
                    Mru::new(self.x0, self.v).into()
                }
            }
        };
        Ok(Body::new(&self.id, &self.id, motion))
    }
}

struct RopeDraft {
    length: f64,
    mass: f64,
    node_count: usize,
    passes: usize,
    p0: [f64; 2],
    p1: [f64; 2],
    p1_pinned: bool,
    surface_y: Option<f64>,
    friction_mu: f64,
}

impl Default for RopeDraft {
    fn default() -> Self {
        Self {
            length: 2.4,
            mass: 1.2,
            node_count: ParticleRope::DEFAULT_NODE_COUNT,
            passes: ParticleRope::DEFAULT_RELAXATION_PASSES,
            p0: [0.0, 2.0],
            p1: [2.0, 2.0],
            p1_pinned: false,
            surface_y: None,
            friction_mu: 0.0,
        }
    }
}

impl RopeDraft {
    fn into_rope(self) -> Result<ParticleRope, String> {
        let mut rope = ParticleRope::with_node_count(
            self.p0,
            self.p1,
            self.length,
            self.mass,
            self.node_count,
        )
        .map_err(|e| format!("Invalid rope parameters: {}", e))?;

        rope.relaxation_passes = self.passes;
        if self.p1_pinned {
            let last_idx = rope.nodes.len().saturating_sub(1);
            let _ = rope.set_pinned(last_idx, true);
        }
        if let Some(sy) = self.surface_y {
            rope = rope.with_surface(sy, self.friction_mu);
        }
        Ok(rope)
    }
}

fn parse_finite_f64(val: &str, field: &str) -> Result<f64, String> {
    let num: f64 = val
        .parse()
        .map_err(|e| format!("Invalid {}: {}", field, e))?;
    if num.is_nan() || num.is_infinite() {
        return Err(format!("{} must be finite", field));
    }
    Ok(num)
}

fn parse_positive_f64(val: &str, field: &str) -> Result<f64, String> {
    let num = parse_finite_f64(val, field)?;
    if num <= 0.0 {
        return Err(format!("{} must be strictly positive", field));
    }
    Ok(num)
}

fn parse_non_negative_f64(val: &str, field: &str) -> Result<f64, String> {
    let num = parse_finite_f64(val, field)?;
    if num < 0.0 {
        return Err(format!("{} cannot be negative", field));
    }
    Ok(num)
}

impl KinFileStorage {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_str(&self, content: &str) -> Result<Scene, String> {
        let mut scene = Scene::default();
        let mut current_body: Option<BodyDraft> = None;
        let mut current_rope: Option<RopeDraft> = None;

        for raw_line in content.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let section = line[1..line.len() - 1].trim();
                commit_pending_drafts(&mut scene, &mut current_body, &mut current_rope)?;

                if let Some(id) = section.strip_prefix("body.") {
                    let trimmed = id.trim();
                    if trimmed.is_empty() {
                        return Err("Body ID cannot be empty".to_string());
                    }
                    current_body = Some(BodyDraft::new(trimmed));
                } else if section.starts_with("rope.") {
                    current_rope = Some(RopeDraft::default());
                }
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                dispatch_kv(
                    &mut scene,
                    &mut current_body,
                    &mut current_rope,
                    key.trim(),
                    val.trim().trim_matches('"'),
                )?;
            }
        }

        commit_pending_drafts(&mut scene, &mut current_body, &mut current_rope)?;
        Ok(scene)
    }

    pub fn serialize_scene(&self, scene: &Scene) -> String {
        let mut out = String::new();
        out.push_str("# KINEMA scene file - format 1\n");
        out.push_str("[scene]\n");
        out.push_str(&format!("name = \"{}\"\n", scene.name));
        out.push_str(&format!("gravity = {}\n\n", scene.gravity));

        for body in &scene.bodies {
            serialize_body(&mut out, body);
        }

        for (idx, rope) in scene.ropes.iter().enumerate() {
            serialize_rope(&mut out, idx, rope);
        }

        out
    }
}

fn serialize_body(out: &mut String, body: &Body) {
    out.push_str(&format!("[body.{}]\n", body.id));
    out.push_str("kind = \"vehicle\"\n");
    match &body.motion {
        Motion::Mru(m) => serialize_mru(out, m),
        Motion::Mruv(m) => serialize_mruv(out, m),
        Motion::Mvl(m) => serialize_mvl(out, m),
        Motion::Dynamics(d) => serialize_dynamics(out, d),
        Motion::Atwood(a) => serialize_atwood(out, a),
        Motion::TablePulley(p) => serialize_table_pulley(out, p),
    }
}

fn serialize_mru(out: &mut String, m: &Mru) {
    out.push_str("motion = \"mru\"\n");
    out.push_str(&format!("x0 = {}\n", m.x0));
    out.push_str(&format!("v = {}\n\n", m.v));
}

fn serialize_mruv(out: &mut String, m: &Mruv) {
    out.push_str("motion = \"mruv\"\n");
    out.push_str(&format!("x0 = {}\n", m.x0));
    out.push_str(&format!("v = {}\n", m.v0));
    out.push_str(&format!("a = {}\n\n", m.a));
}

fn serialize_mvl(out: &mut String, m: &Mvl) {
    out.push_str("motion = \"mvl\"\n");
    out.push_str(&format!("y0 = {}\n", m.y0));
    out.push_str(&format!("v0 = {}\n", m.v0));
    out.push_str(&format!("g = {}\n\n", m.g));
}

fn serialize_dynamics(out: &mut String, d: &BlockDynamics) {
    out.push_str("motion = \"dynamics\"\n");
    out.push_str(&format!("mass = {}\n", d.mass));
    out.push_str(&format!("theta = {}\n", d.theta));
    out.push_str(&format!("mu_s = {}\n", d.mu_s));
    out.push_str(&format!("mu_k = {}\n", d.mu_k));
    out.push_str(&format!("f_app = {}\n", d.f_app));
    out.push_str(&format!("gravity = {}\n", d.gravity));
    out.push_str(&format!("x0 = {}\n", d.x0));
    out.push_str(&format!("v0 = {}\n\n", d.v0));
}

fn serialize_atwood(out: &mut String, a: &AtwoodMachine) {
    out.push_str("motion = \"atwood\"\n");
    out.push_str(&format!("m1 = {}\n", a.m1));
    out.push_str(&format!("m2 = {}\n", a.m2));
    out.push_str(&format!("gravity = {}\n", a.g));
    out.push_str(&format!("s0 = {}\n", a.s0));
    out.push_str(&format!("v0 = {}\n\n", a.v0));
}

fn serialize_table_pulley(out: &mut String, p: &TablePulleySystem) {
    out.push_str("motion = \"table_pulley\"\n");
    out.push_str(&format!("m1 = {}\n", p.m1));
    out.push_str(&format!("m2 = {}\n", p.m2));
    out.push_str(&format!("mu_s = {}\n", p.mu_s));
    out.push_str(&format!("mu_k = {}\n", p.mu_k));
    out.push_str(&format!("gravity = {}\n", p.g));
    out.push_str(&format!("s0 = {}\n", p.s0));
    out.push_str(&format!("v0 = {}\n\n", p.v0));
}

fn serialize_rope(out: &mut String, idx: usize, rope: &ParticleRope) {
    out.push_str(&format!("[rope.rope_{}]\n", idx + 1));
    out.push_str(&format!("length = {}\n", rope.total_length));
    out.push_str(&format!("mass = {}\n", rope.total_mass));
    out.push_str(&format!("node_count = {}\n", rope.nodes.len()));
    out.push_str(&format!("passes = {}\n", rope.relaxation_passes));
    if let (Some(first), Some(last)) = (rope.nodes.first(), rope.nodes.last()) {
        out.push_str(&format!("p0_x = {}\n", first.pos[0]));
        out.push_str(&format!("p0_y = {}\n", first.pos[1]));
        out.push_str(&format!("p1_x = {}\n", last.pos[0]));
        out.push_str(&format!("p1_y = {}\n", last.pos[1]));
        out.push_str(&format!("p1_pinned = {}\n", last.pinned));
    }
    if let Some(sy) = rope.surface_y {
        out.push_str(&format!("surface_y = {}\n", sy));
        out.push_str(&format!("friction_mu = {}\n", rope.friction_mu));
    }
    out.push('\n');
}

fn commit_pending_drafts(
    scene: &mut Scene,
    body_draft: &mut Option<BodyDraft>,
    rope_draft: &mut Option<RopeDraft>,
) -> Result<(), String> {
    if let Some(draft) = body_draft.take() {
        if scene.bodies.len() >= MAX_BODIES_COUNT {
            return Err("Scene exceeds maximum allowed bodies (256)".to_string());
        }
        if scene.bodies.iter().any(|b| b.id == draft.id) {
            return Err(format!("Duplicate body ID '{}'", draft.id));
        }
        scene.add_body(draft.into_body()?);
    }
    if let Some(draft) = rope_draft.take() {
        scene.add_rope(draft.into_rope()?);
    }
    Ok(())
}

fn dispatch_kv(
    scene: &mut Scene,
    body_draft: &mut Option<BodyDraft>,
    rope_draft: &mut Option<RopeDraft>,
    key: &str,
    val: &str,
) -> Result<(), String> {
    if let Some(draft) = body_draft.as_mut() {
        apply_body_kv(draft, key, val)
    } else if let Some(draft) = rope_draft.as_mut() {
        apply_rope_kv(draft, key, val)
    } else {
        apply_scene_kv(scene, key, val)
    }
}

fn apply_scene_kv(scene: &mut Scene, key: &str, val: &str) -> Result<(), String> {
    match key {
        "name" => scene.name = val.to_string(),
        "gravity" => scene.gravity = parse_non_negative_f64(val, "Gravity")?,
        _ => {}
    }
    Ok(())
}

fn apply_body_kv(draft: &mut BodyDraft, key: &str, val: &str) -> Result<(), String> {
    match key {
        "motion" => draft.motion_type = val.to_string(),
        "x0" | "y0" | "s0" => draft.x0 = parse_finite_f64(val, key)?,
        "v" | "v0" => draft.v = parse_finite_f64(val, key)?,
        "a" => draft.a = parse_finite_f64(val, "a")?,
        "g" | "gravity" => draft.g = parse_non_negative_f64(val, "Gravity")?,
        "mass" | "m" => draft.mass = parse_positive_f64(val, "Mass")?,
        "m1" => draft.m1 = parse_positive_f64(val, "m1")?,
        "m2" => draft.m2 = parse_positive_f64(val, "m2")?,
        "theta" | "angle" => draft.theta = parse_non_negative_f64(val, "Angle")?,
        "mu_s" => draft.mu_s = parse_non_negative_f64(val, "mu_s")?,
        "mu_k" => draft.mu_k = parse_non_negative_f64(val, "mu_k")?,
        "f_app" | "f" => draft.f_app = parse_finite_f64(val, "Applied force")?,
        _ => {}
    }
    Ok(())
}

fn apply_rope_kv(draft: &mut RopeDraft, key: &str, val: &str) -> Result<(), String> {
    match key {
        "length" => draft.length = parse_positive_f64(val, "Rope length")?,
        "mass" => draft.mass = parse_positive_f64(val, "Rope mass")?,
        "node_count" => {
            draft.node_count = val
                .parse()
                .map_err(|e| format!("Invalid node count: {}", e))?
        }
        "passes" => draft.passes = val.parse().map_err(|e| format!("Invalid passes: {}", e))?,
        "p0_x" => draft.p0[0] = parse_finite_f64(val, "p0_x")?,
        "p0_y" => draft.p0[1] = parse_finite_f64(val, "p0_y")?,
        "p1_x" => draft.p1[0] = parse_finite_f64(val, "p1_x")?,
        "p1_y" => draft.p1[1] = parse_finite_f64(val, "p1_y")?,
        "p1_pinned" => draft.p1_pinned = val == "true" || val == "1",
        "surface_y" => draft.surface_y = Some(parse_finite_f64(val, "surface_y")?),
        "friction_mu" => draft.friction_mu = parse_non_negative_f64(val, "friction_mu")?,
        _ => {}
    }
    Ok(())
}

impl SceneRepository for KinFileStorage {
    fn load(&self, path: &str) -> Result<Scene, String> {
        let meta = fs::metadata(path).map_err(|e| format!("Could not read metadata: {}", e))?;
        if meta.len() > MAX_FILE_SIZE_BYTES {
            return Err("File exceeds maximum allowed size (1 MiB)".to_string());
        }

        let content =
            fs::read_to_string(path).map_err(|e| format!("Could not read file contents: {}", e))?;
        self.parse_str(&content)
    }

    fn save(&self, path: &str, scene: &Scene) -> Result<(), String> {
        let mut file = fs::File::create(path).map_err(|e| format!("Could not open file: {}", e))?;
        let content = self.serialize_scene(scene);
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Could not write scene: {}", e))
    }
}
