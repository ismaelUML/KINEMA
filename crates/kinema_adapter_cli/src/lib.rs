//! Driving Adapter: Headless CLI runner for batch verification and checks.

use kinema_domain::motion::Motion1D;
use kinema_domain::Scene;

pub struct CliRunner;

impl CliRunner {
    pub fn summarize_scene(scene: &Scene, time: f64) -> Vec<String> {
        let mut report = Vec::new();
        report.push(format!("Scene: {} (t = {:.3} s)", scene.name, time));
        for body in &scene.bodies {
            let x = body.motion.position_at(time);
            let v = body.motion.velocity_at(time);
            report.push(format!(" - Body {}: x = {:.2} m, v = {:.2} m/s", body.name, x, v));
        }
        report
    }
}
