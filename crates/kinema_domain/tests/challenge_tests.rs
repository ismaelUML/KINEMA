use kinema_domain::{
    AtwoodMachine, BlockDynamics, Body, Challenge, Mru, Mruv, Mvl, Scene,
};

#[test]
fn test_challenge_catalog_contains_items() {
    let catalog = Challenge::catalog();
    assert_eq!(catalog.len(), 5);
    assert_eq!(catalog[0].id, "challenge_mru_intercept");
    assert_eq!(catalog[1].id, "challenge_mruv_braking");
    assert_eq!(catalog[2].id, "challenge_moon_drop");
    assert_eq!(catalog[3].id, "challenge_atwood_acc");
    assert_eq!(catalog[4].id, "challenge_coulomb_friction");
}

#[test]
fn test_challenge_mru_intercept_evaluation() {
    let catalog = Challenge::catalog();
    let challenge = &catalog[0];

    let mut scene = Scene::new("Test");
    // Body A: x(0) = 0, v = 10
    // Body B: x(0) = 40, v = 0 -> Meet at t = 4s, x = 40m (within [4, 5]s, but pos 40 not in [50, 70])
    scene.add_body(Body::new("carA", "Car A", Mru::new(0.0, 10.0)));
    scene.add_body(Body::new("carB", "Car B", Mru::new(40.0, 0.0)));

    let status = challenge.evaluate(&scene);
    assert!(!status.is_solved());

    // Adjust Car A: v = 15 m/s, Car B: x0 = 10, v = 2.5 m/s
    // x_A = 15 t, x_B = 10 + 2.5 t => 12.5 t = 10 => not quite
    // Let's make meeting exactly at t = 4.0s, x = 60.0m:
    // Car A: x0 = 0, v = 15 => at t=4, x = 60
    // Car B: x0 = 40, v = 5 => at t=4, x = 60
    let mut scene2 = Scene::new("Solved Scene");
    scene2.add_body(Body::new("carA", "Car A", Mru::new(0.0, 15.0)));
    scene2.add_body(Body::new("carB", "Car B", Mru::new(40.0, 5.0)));

    let status2 = challenge.evaluate(&scene2);
    assert!(status2.is_solved());
}

#[test]
fn test_challenge_mruv_braking_evaluation() {
    let catalog = Challenge::catalog();
    let challenge = &catalog[1];

    // car2: v0 = 20 m/s, a = -5 m/s^2 => stopping distance d = v0^2 / (2|a|) = 400 / 10 = 40.0m (within [38, 42])
    let mut scene = Scene::new("Braking Scene");
    scene.add_body(Body::new("car2", "Car 2", Mruv::new(0.0, 20.0, -5.0)));

    let status = challenge.evaluate(&scene);
    assert!(status.is_solved());
}

#[test]
fn test_challenge_moon_drop_evaluation() {
    let catalog = Challenge::catalog();
    let challenge = &catalog[2];

    // hammer: y0 = 20.25 m, v0 = 0, g = 1.62
    // t_impact = sqrt(2 * 20.25 / 1.62) = sqrt(25) = 5.0s (within [4.8, 5.2])
    let mut scene = Scene::new("Moon Scene");
    scene.add_body(Body::new("hammer", "Hammer", Mvl::new(20.25, 0.0, 1.62)));

    let status = challenge.evaluate(&scene);
    assert!(status.is_solved());
}

#[test]
fn test_challenge_atwood_acc_evaluation() {
    let catalog = Challenge::catalog();
    let challenge = &catalog[3];

    // m1 = 10, m2 = 15.1, g = 9.81
    // a = (15.1 - 10) / 25.1 * 9.81 = 5.1 / 25.1 * 9.81 ≈ 1.993 m/s² (within [1.9, 2.1])
    let mut scene = Scene::new("Atwood Scene");
    let atwood = AtwoodMachine::new(10.0, 15.1)
        .expect("valid atwood")
        .with_gravity(9.81);
    scene.add_body(Body::new("atwood_sys", "Atwood", atwood));

    let status = challenge.evaluate(&scene);
    assert!(status.is_solved());
}

#[test]
fn test_challenge_friction_threshold_evaluation() {
    let catalog = Challenge::catalog();
    let challenge = &catalog[4];

    // block: m = 50, theta = 0, mu_s = 0.5, mu_k = 0.3
    // F_roz_max = 0.5 * 50 * 9.80665 = 245.166 N
    // F_app = 260 N => F_roz_kin = 0.3 * 50 * 9.80665 = 147.1 N => F_net = 260 - 147.1 = 112.9 N >= 5.0 N
    let mut scene = Scene::new("Friction Scene");
    let block = BlockDynamics::horizontal(50.0, 0.5, 0.3, 260.0);
    scene.add_body(Body::new("block", "Block", block));

    let status = challenge.evaluate(&scene);
    assert!(status.is_solved());
}
