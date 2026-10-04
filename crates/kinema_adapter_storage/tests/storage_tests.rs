use kinema_adapter_storage::KinFileStorage;
use kinema_domain::Motion1D;

#[test]
fn test_parse_valid_kin_scene() {
    let raw = r#"
# Sample file
[scene]
name = "Two cars meeting"
gravity = 9.80665

[body.car_a]
kind = "vehicle"
motion = "mru"
x0 = 0.0
v = 15.0

[body.car_b]
kind = "vehicle"
motion = "mruv"
x0 = 100.0
v = -10.0
a = 2.0
"#;
    let storage = KinFileStorage::new();
    let scene = storage.parse_str(raw).expect("Failed to parse valid .kin");
    assert_eq!(scene.name, "Two cars meeting");
    assert_eq!(scene.bodies.len(), 2);
    assert_eq!(scene.bodies[0].id, "car_a");
    assert_eq!(scene.bodies[0].motion.velocity_at(0.0), 15.0);
    assert_eq!(scene.bodies[1].id, "car_b");
    assert_eq!(scene.bodies[1].motion.position_at(0.0), 100.0);
    assert_eq!(scene.bodies[1].motion.acceleration_at(0.0), 2.0);

    // Roundtrip test
    let serialized = storage.serialize_scene(&scene);
    let roundtripped = storage.parse_str(&serialized).expect("Roundtrip parse");
    assert_eq!(roundtripped.bodies.len(), 2);
    assert_eq!(roundtripped.bodies[1].motion.acceleration_at(0.0), 2.0);
}

#[test]
fn test_reject_nan_gravity() {
    let raw = "[scene]\ngravity = NaN\n";
    let storage = KinFileStorage::new();
    assert!(storage.parse_str(raw).is_err());
}

#[test]
fn test_parse_and_roundtrip_mvl_scene() {
    let raw = r#"
[scene]
name = "Free Fall Drop"
gravity = 9.81

[body.ball]
kind = "vehicle"
motion = "mvl"
y0 = 20.0
v0 = 0.0
g = 9.81
"#;
    let storage = KinFileStorage::new();
    let scene = storage.parse_str(raw).expect("Failed to parse Mvl scene");
    assert_eq!(scene.name, "Free Fall Drop");
    assert_eq!(scene.bodies.len(), 1);
    let ball = &scene.bodies[0];
    assert_eq!(ball.id, "ball");
    assert_eq!(ball.motion.position_at(0.0), 20.0);
    assert_eq!(ball.motion.velocity_at(0.0), 0.0);
    assert_eq!(ball.motion.acceleration_at(0.0), -9.81);

    // Serialization roundtrip
    let serialized = storage.serialize_scene(&scene);
    assert!(serialized.contains("motion = \"mvl\""));
    assert!(serialized.contains("y0 = 20"));
    let roundtripped = storage.parse_str(&serialized).expect("Mvl roundtrip");
    assert_eq!(roundtripped.bodies.len(), 1);
    assert_eq!(roundtripped.bodies[0].motion.position_at(0.0), 20.0);
}

#[test]
fn test_reject_negative_gravity_in_scene() {
    let raw = "[scene]\ngravity = -9.81\n";
    let storage = KinFileStorage::new();
    let err = storage.parse_str(raw).unwrap_err();
    assert!(
        err.contains("negative"),
        "expected negative error, got {err}"
    );
}

#[test]
fn test_reject_negative_gravity_in_body() {
    let raw = r#"
[body.drop]
motion = "mvl"
y0 = 10.0
v0 = 0.0
g = -9.81
"#;
    let storage = KinFileStorage::new();
    let err = storage.parse_str(raw).unwrap_err();
    assert!(
        err.contains("negative"),
        "expected negative error, got {err}"
    );
}

#[test]
fn test_reject_empty_body_id() {
    let raw = "[body. ]\nmotion = \"mru\"\nx0 = 0.0\nv = 10.0\n";
    let storage = KinFileStorage::new();
    assert!(storage.parse_str(raw).is_err());
}

#[test]
fn test_reject_duplicate_body_id() {
    let raw = r#"
[body.car]
motion = "mru"
x0 = 0.0
v = 10.0

[body.car]
motion = "mru"
x0 = 50.0
v = 20.0
"#;
    let storage = KinFileStorage::new();
    let err = storage.parse_str(raw).unwrap_err();
    assert!(err.contains("Duplicate body ID 'car'"));
}

#[test]
fn test_reject_infinite_body_coordinate() {
    let raw = "[body.bad]\nmotion = \"mru\"\nx0 = inf\nv = 10.0\n";
    let storage = KinFileStorage::new();
    assert!(storage.parse_str(raw).is_err());
}

#[test]
fn test_reject_exceeding_max_bodies() {
    let storage = KinFileStorage::new();
    let mut raw = String::from("[scene]\nname = \"Crowded\"\n");
    for i in 0..257 {
        raw.push_str(&format!(
            "[body.b{i}]\nmotion = \"mru\"\nx0 = 0.0\nv = 1.0\n"
        ));
    }
    let err = storage.parse_str(&raw).unwrap_err();
    assert!(err.contains("maximum allowed bodies"));
}

#[test]
fn test_parse_and_roundtrip_dynamics_scene() {
    let raw = r#"
[scene]
name = "Block on rough incline"
gravity = 9.81

[body.crate]
kind = "vehicle"
motion = "dynamics"
mass = 5.0
theta = 0.5235987755982988
mu_s = 0.5
mu_k = 0.3
f_app = 25.0
gravity = 9.81
x0 = 0.0
v0 = 0.0
"#;
    let storage = KinFileStorage::new();
    let scene = storage.parse_str(raw).expect("parse dynamics scene");
    assert_eq!(scene.bodies.len(), 1);
    let b = &scene.bodies[0];
    assert_eq!(b.id, "crate");
    assert_eq!(b.motion.position_at(0.0), 0.0);

    // Roundtrip
    let serialized = storage.serialize_scene(&scene);
    assert!(serialized.contains("motion = \"dynamics\""));
    assert!(serialized.contains("mass = 5"));
    assert!(serialized.contains("mu_s = 0.5"));
    let roundtripped = storage.parse_str(&serialized).expect("roundtrip parse");
    assert_eq!(roundtripped.bodies.len(), 1);
}

#[test]
fn test_reject_non_positive_mass() {
    let raw = "[body.bad]\nmotion = \"dynamics\"\nmass = -2.0\n";
    let storage = KinFileStorage::new();
    let err = storage.parse_str(raw).unwrap_err();
    assert!(err.contains("positive"));
}

#[test]
fn test_reject_negative_friction() {
    let raw = "[body.bad]\nmotion = \"dynamics\"\nmu_s = -0.5\n";
    let storage = KinFileStorage::new();
    let err = storage.parse_str(raw).unwrap_err();
    assert!(err.contains("negative"));
}

#[test]
fn test_parse_and_roundtrip_atwood_scene() {
    let raw = r#"
[scene]
name = "Atwood Machine Demo"
gravity = 9.81

[body.atwood_1]
motion = "atwood"
m1 = 2.0
m2 = 3.0
gravity = 9.81
s0 = 0.0
v0 = 0.0
"#;
    let storage = KinFileStorage::new();
    let scene = storage.parse_str(raw).expect("parse atwood");
    assert_eq!(scene.bodies.len(), 1);
    let tension = scene.bodies[0]
        .motion
        .pulley_tension()
        .expect("has tension");
    assert!((tension - 23.544).abs() < 1e-4);

    let serialized = storage.serialize_scene(&scene);
    let roundtripped = storage.parse_str(&serialized).expect("roundtrip parse");
    assert_eq!(roundtripped.bodies.len(), 1);
    assert!((roundtripped.bodies[0].motion.pulley_tension().unwrap() - 23.544).abs() < 1e-4);
}

#[test]
fn test_parse_and_roundtrip_table_pulley_scene() {
    let raw = r#"
[scene]
name = "Table Pulley Demo"
gravity = 9.81

[body.pulley_1]
motion = "table_pulley"
m1 = 10.0
m2 = 6.0
mu_s = 0.5
mu_k = 0.3
gravity = 9.81
s0 = 0.0
v0 = 0.0
"#;
    let storage = KinFileStorage::new();
    let scene = storage.parse_str(raw).expect("parse table pulley");
    assert_eq!(scene.bodies.len(), 1);
    let tension = scene.bodies[0]
        .motion
        .pulley_tension()
        .expect("has tension");
    assert!((tension - 47.82375).abs() < 1e-4);

    let serialized = storage.serialize_scene(&scene);
    let roundtripped = storage.parse_str(&serialized).expect("roundtrip parse");
    assert_eq!(roundtripped.bodies.len(), 1);
    assert!((roundtripped.bodies[0].motion.pulley_tension().unwrap() - 47.82375).abs() < 1e-4);
}

#[test]
fn test_parse_and_roundtrip_rope_scene() {
    let raw = r#"
[scene]
name = "Hanging Catenary Rope"
gravity = 9.80665

[rope.rope_1]
length = 3.0
mass = 1.5
node_count = 24
passes = 12
p0_x = 0.0
p0_y = 4.0
p1_x = 2.5
p1_y = 4.0
p1_pinned = true
surface_y = 0.0
friction_mu = 0.2
"#;
    let storage = KinFileStorage::new();
    let scene = storage.parse_str(raw).expect("parse rope");
    assert_eq!(scene.ropes.len(), 1);
    let rope = &scene.ropes[0];
    assert_eq!(rope.nodes.len(), 24);
    assert_eq!(rope.relaxation_passes, 12);
    assert!(rope.nodes[23].pinned);
    assert_eq!(rope.surface_y, Some(0.0));
    assert_eq!(rope.friction_mu, 0.2);

    let serialized = storage.serialize_scene(&scene);
    let roundtripped = storage.parse_str(&serialized).expect("roundtrip parse");
    assert_eq!(roundtripped.ropes.len(), 1);
    assert_eq!(roundtripped.ropes[0].nodes.len(), 24);
    assert!(roundtripped.ropes[0].nodes[23].pinned);
}

#[test]
fn test_png_export_success_and_valid_structure() {
    use kinema_adapter_storage::PngCanvasExporter;
    use kinema_domain::motion::Mru;
    use kinema_domain::scene::{Body, Scene};
    use kinema_ports::{CancellationToken, ImageExporter};
    use std::fs;

    let mut scene = Scene::new("Export Test Scene");
    let car = Body::new("car1", "Car 1", Mru::new(0.0, 10.0));
    scene.add_body(car);

    let exporter = PngCanvasExporter::new(320, 240);
    let token = CancellationToken::new();
    let out_path = std::env::temp_dir().join("kinema_test_export.png");
    let out_str = out_path.to_str().unwrap();

    let result = exporter.export_png(&scene, 2.0, out_str, &token);
    assert!(result.is_ok(), "PNG export failed: {:?}", result);

    let bytes = fs::read(&out_path).expect("Read exported PNG");
    assert!(bytes.len() > 64);
    // Check PNG signature: 89 50 4E 47 0D 0A 1A 0A
    assert_eq!(&bytes[0..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    // Check IHDR chunk
    assert_eq!(&bytes[12..16], b"IHDR");
    let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    assert_eq!(width, 320);
    assert_eq!(height, 240);
    assert_eq!(bytes[24], 8); // bit depth 8
    assert_eq!(bytes[25], 2); // color type RGB

    // Check IEND chunk at end
    let len = bytes.len();
    assert_eq!(&bytes[len - 8..len - 4], b"IEND");

    let _ = fs::remove_file(&out_path);
}

#[test]
fn test_png_export_cancellation_cleans_up_file() {
    use kinema_adapter_storage::PngCanvasExporter;
    use kinema_domain::scene::Scene;
    use kinema_ports::{CancellationToken, ImageExporter};

    let scene = Scene::new("Cancelled Scene");
    let exporter = PngCanvasExporter::new(640, 480);
    let token = CancellationToken::new();
    token.cancel(); // Cancel immediately

    let out_path = std::env::temp_dir().join("kinema_cancelled_export.png");
    let out_str = out_path.to_str().unwrap();

    let result = exporter.export_png(&scene, 0.0, out_str, &token);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Export cancelled by user");

    // Must clean up any temporary or target files
    assert!(!out_path.exists(), "Target file must not exist after cancellation");
}

