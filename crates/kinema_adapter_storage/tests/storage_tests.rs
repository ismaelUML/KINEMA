use kinema_adapter_storage::KinFileStorage;

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
motion = "mru"
x0 = 100.0
v = -10.0
"#;
    let storage = KinFileStorage::new();
    let scene = storage.parse_str(raw).expect("Failed to parse valid .kin");
    assert_eq!(scene.name, "Two cars meeting");
    assert_eq!(scene.bodies.len(), 2);
    assert_eq!(scene.bodies[0].id, "car_a");
    assert_eq!(scene.bodies[0].motion.v, 15.0);
    assert_eq!(scene.bodies[1].id, "car_b");
    assert_eq!(scene.bodies[1].motion.x0, 100.0);
}

#[test]
fn test_reject_nan_gravity() {
    let raw = "[scene]\ngravity = NaN\n";
    let storage = KinFileStorage::new();
    assert!(storage.parse_str(raw).is_err());
}
