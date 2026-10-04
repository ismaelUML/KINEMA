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
