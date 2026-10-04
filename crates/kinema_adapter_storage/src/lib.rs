//! Driven Adapter: .kin Scene File Storage with input hardening.

use kinema_domain::motion::Mru;
use kinema_domain::scene::{Body, Scene};
use kinema_ports::SceneRepository;
use std::fs;
use std::io::Write;

pub const MAX_FILE_SIZE_BYTES: u64 = 1024 * 1024; // 1 MiB
pub const MAX_BODIES_COUNT: usize = 256;

#[derive(Default)]
pub struct KinFileStorage;

impl KinFileStorage {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_str(&self, content: &str) -> Result<Scene, String> {
        let mut scene = Scene::default();
        let mut current_body_id: Option<String> = None;
        let mut current_x0 = 0.0;
        let mut current_v = 0.0;

        for raw_line in content.lines() {
            let line = raw_line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') {
                let section = &line[1..line.len() - 1].trim();
                if let Some(id) = current_body_id.take() {
                    if scene.bodies.len() >= MAX_BODIES_COUNT {
                        return Err("Scene exceeds maximum allowed bodies (256)".to_string());
                    }
                    scene.add_body(Body {
                        id: id.clone(),
                        name: id,
                        motion: Mru::new(current_x0, current_v),
                    });
                    current_x0 = 0.0;
                    current_v = 0.0;
                }

                if let Some(stripped) = section.strip_prefix("body.") {
                    current_body_id = Some(stripped.to_string());
                }
                continue;
            }

            if let Some((key, val)) = line.split_once('=') {
                let key = key.trim();
                let val = val.trim().trim_matches('"');
                match key {
                    "name" => scene.name = val.to_string(),
                    "gravity" => {
                        let g: f64 = val.parse().map_err(|e| format!("Invalid gravity: {}", e))?;
                        if g.is_nan() || g.is_infinite() {
                            return Err("Gravity must be finite".to_string());
                        }
                        scene.gravity = g;
                    }
                    "x0" => {
                        let x0: f64 = val.parse().map_err(|e| format!("Invalid x0: {}", e))?;
                        if x0.is_nan() || x0.is_infinite() {
                            return Err("x0 must be finite".to_string());
                        }
                        current_x0 = x0;
                    }
                    "v" => {
                        let v: f64 = val.parse().map_err(|e| format!("Invalid v: {}", e))?;
                        if v.is_nan() || v.is_infinite() {
                            return Err("v must be finite".to_string());
                        }
                        current_v = v;
                    }
                    _ => {} // Unknown keys treated as warnings / ignored per section 5.7
                }
            }
        }

        if let Some(id) = current_body_id {
            if scene.bodies.len() >= MAX_BODIES_COUNT {
                return Err("Scene exceeds maximum allowed bodies (256)".to_string());
            }
            scene.add_body(Body {
                id: id.clone(),
                name: id,
                motion: Mru::new(current_x0, current_v),
            });
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
            out.push_str("motion = \"mru\"\n");
            out.push_str(&format!("x0 = {}\n", body.motion.x0));
            out.push_str(&format!("v = {}\n\n", body.motion.v));
        }

        out
    }
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
