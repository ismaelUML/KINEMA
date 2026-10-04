use kinema_domain::motion::{Motion, Mru, Mruv};
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
}

impl BodyDraft {
    fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            motion_type: "mru".to_string(),
            x0: 0.0,
            v: 0.0,
            a: 0.0,
        }
    }

    fn into_body(self) -> Body {
        let motion: Motion = if self.motion_type == "mruv" || self.a.abs() > 1e-12 {
            Mruv::new(self.x0, self.v, self.a).into()
        } else {
            Mru::new(self.x0, self.v).into()
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
                    current_draft = Some(BodyDraft::new(stripped));
                }
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                apply_kv_pair(&mut scene, &mut current_draft, key.trim(), val.trim().trim_matches('"'))?;
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
            out.push_str(&format!("[body.{}]\n", body.id));
            out.push_str("kind = \"vehicle\"\n");
            match &body.motion {
                Motion::Mru(m) => {
                    out.push_str("motion = \"mru\"\n");
                    out.push_str(&format!("x0 = {}\n", m.x0));
                    out.push_str(&format!("v = {}\n\n", m.v));
                }
                Motion::Mruv(m) => {
                    out.push_str("motion = \"mruv\"\n");
                    out.push_str(&format!("x0 = {}\n", m.x0));
                    out.push_str(&format!("v = {}\n", m.v0));
                    out.push_str(&format!("a = {}\n\n", m.a));
                }
            }
        }

        out
    }
}

fn commit_body_draft(scene: &mut Scene, draft: BodyDraft) -> Result<(), String> {
    if scene.bodies.len() >= MAX_BODIES_COUNT {
        return Err("Scene exceeds maximum allowed bodies (256)".to_string());
    }
    scene.add_body(draft.into_body());
    Ok(())
}

fn apply_kv_pair(
    scene: &mut Scene,
    draft: &mut Option<BodyDraft>,
    key: &str,
    val: &str,
) -> Result<(), String> {
    match key {
        "name" => scene.name = val.to_string(),
        "gravity" => scene.gravity = parse_finite_f64(val, "gravity")?,
        "motion" => {
            if let Some(d) = draft.as_mut() {
                d.motion_type = val.to_string();
            }
        }
        "x0" => {
            if let Some(d) = draft.as_mut() {
                d.x0 = parse_finite_f64(val, "x0")?;
            }
        }
        "v" | "v0" => {
            if let Some(d) = draft.as_mut() {
                d.v = parse_finite_f64(val, "v")?;
            }
        }
        "a" => {
            if let Some(d) = draft.as_mut() {
                d.a = parse_finite_f64(val, "a")?;
            }
        }
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
