use kinema_domain::dynamics::BlockDynamics;
use kinema_domain::motion::{Motion, Mru, Mruv, Mvl};
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
            theta: 0.0,
            mu_s: 0.5,
            mu_k: 0.3,
            f_app: 0.0,
        }
    }

    fn into_body(self) -> Body {
        let motion: Motion = match self.motion_type.as_str() {
            "dynamics" => BlockDynamics::new(self.mass, self.theta, self.mu_s, self.mu_k)
                .with_applied_force(self.f_app)
                .with_initial_state(self.x0, self.v)
                .with_gravity(self.g)
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
        Body::new(&self.id, &self.id, motion)
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
        let mut current_draft: Option<BodyDraft> = None;

        for raw_line in content.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let section = line[1..line.len() - 1].trim();
                if let Some(draft) = current_draft.take() {
                    commit_body_draft(&mut scene, draft)?;
                }
                if let Some(stripped) = section.strip_prefix("body.") {
                    let id = stripped.trim();
                    if id.is_empty() {
                        return Err("Body ID cannot be empty".to_string());
                    }
                    current_draft = Some(BodyDraft::new(id));
                }
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                dispatch_kv_pair(
                    &mut scene,
                    &mut current_draft,
                    key.trim(),
                    val.trim().trim_matches('"'),
                )?;
            }
        }

        if let Some(draft) = current_draft {
            commit_body_draft(&mut scene, draft)?;
        }

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

fn commit_body_draft(scene: &mut Scene, draft: BodyDraft) -> Result<(), String> {
    if scene.bodies.len() >= MAX_BODIES_COUNT {
        return Err("Scene exceeds maximum allowed bodies (256)".to_string());
    }
    if scene.bodies.iter().any(|b| b.id == draft.id) {
        return Err(format!("Duplicate body ID '{}'", draft.id));
    }
    scene.add_body(draft.into_body());
    Ok(())
}

fn dispatch_kv_pair(
    scene: &mut Scene,
    draft: &mut Option<BodyDraft>,
    key: &str,
    val: &str,
) -> Result<(), String> {
    if draft.is_some() {
        apply_body_kv(draft.as_mut().unwrap(), key, val)
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
        "x0" | "y0" => draft.x0 = parse_finite_f64(val, key)?,
        "v" | "v0" => draft.v = parse_finite_f64(val, key)?,
        "a" => draft.a = parse_finite_f64(val, "a")?,
        "g" | "gravity" => draft.g = parse_non_negative_f64(val, "Gravity")?,
        "mass" | "m" => draft.mass = parse_positive_f64(val, "Mass")?,
        "theta" | "angle" => draft.theta = parse_non_negative_f64(val, "Angle")?,
        "mu_s" => draft.mu_s = parse_non_negative_f64(val, "mu_s")?,
        "mu_k" => draft.mu_k = parse_non_negative_f64(val, "mu_k")?,
        "f_app" | "f" => draft.f_app = parse_finite_f64(val, "Applied force")?,
        _ => {}
    }
    Ok(())
}

impl SceneRepository for KinFileStorage {
    fn load(&self, path: &str) -> Result<Scene, String> {
        let meta = fs::metadata(path).map_err(|e| format!("Could not read metadata: {}", e))?;
        if meta.len() > MAX_FILE_SIZE_BYTES {
            return Err("File exceeds 1 MiB limit".to_string());
        }

        let content =
            fs::read_to_string(path).map_err(|e| format!("Could not read file: {}", e))?;
        self.parse_str(&content)
    }

    fn save(&self, path: &str, scene: &Scene) -> Result<(), String> {
        let content = self.serialize_scene(scene);
        let mut file =
            fs::File::create(path).map_err(|e| format!("Could not create file: {}", e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Could not write file: {}", e))?;
        Ok(())
    }
}
