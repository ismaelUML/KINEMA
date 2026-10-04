use kinema_domain::dynamics::FrictionState;
use kinema_domain::motion::{Motion1D, ParametricLaw};
use kinema_domain::pulley::{AtwoodMachine, TablePulleySystem};

#[test]
fn test_atwood_machine_analytic_equations() {
    let atwood = AtwoodMachine::new(2.0, 3.0)
        .expect("valid masses")
        .with_gravity(9.81);

    // a = (3 - 2) * 9.81 / 5 = 1.962 m/s²
    let expected_a = 1.962;
    assert!((atwood.acceleration() - expected_a).abs() < 1e-9);

    // T = 2 * 2 * 3 * 9.81 / 5 = 23.544 N
    let expected_t = 23.544;
    assert!((atwood.tension() - expected_t).abs() < 1e-9);

    // After 2.0 seconds from rest:
    // v(t) = a * t = 3.924 m/s
    // s(t) = 0.5 * a * t² = 0.5 * 1.962 * 4 = 3.924 m
    assert!((atwood.velocity_at(2.0) - 3.924).abs() < 1e-9);
    assert!((atwood.position_at(2.0) - 3.924).abs() < 1e-9);
    assert!((atwood.mass1_position_at(2.0) - (-3.924)).abs() < 1e-9);
    assert!((atwood.mass2_position_at(2.0) - 3.924).abs() < 1e-9);
}

#[test]
fn test_atwood_semi_implicit_euler_integrator_bound() {
    let atwood = AtwoodMachine::new(2.0, 3.0)
        .expect("valid masses")
        .with_gravity(9.81);

    let dt = 1.0 / 240.0;
    let mut num_s = 0.0;
    let mut num_v = 0.0;

    for _ in 0..480 {
        let (ns, nv, _) = atwood.step_euler(num_s, num_v, dt);
        num_s = ns;
        num_v = nv;
    }

    let exact_v = atwood.velocity_at(2.0);
    let exact_s = atwood.position_at(2.0);

    // Under constant acceleration, Semi-Implicit Euler velocity is exact to floating-point precision
    assert!((num_v - exact_v).abs() < 1e-9);
    // Position error is strictly bounded: |num_s - exact_s| < 0.01 m
    assert!((num_s - exact_s).abs() < 0.01);
}

#[test]
fn test_table_pulley_static_threshold() {
    // m1 = 10 kg, m2 = 2 kg, mu_s = 0.5, mu_k = 0.3
    // Pulling force = m2 * g = 19.62 N
    // Max static friction = mu_s * m1 * g = 0.5 * 10 * 9.81 = 49.05 N
    // 19.62 <= 49.05 => Static equilibrium!
    let table = TablePulleySystem::new(10.0, 2.0, 0.5, 0.3)
        .expect("valid system")
        .with_gravity(9.81);

    assert!(table.is_static_at_rest());
    assert_eq!(table.friction_state(), FrictionState::Static);
    assert_eq!(table.acceleration(), 0.0);
    assert!((table.tension() - 19.62).abs() < 1e-9);
    assert!((table.friction_force() - 19.62).abs() < 1e-9);

    let (next_s, next_v, next_a, state) = table.step_euler(0.0, 0.0, 1.0 / 240.0);
    assert_eq!(next_s, 0.0);
    assert_eq!(next_v, 0.0);
    assert_eq!(next_a, 0.0);
    assert_eq!(state, FrictionState::Static);
}

#[test]
fn test_table_pulley_kinetic_acceleration() {
    // Increase m2 to 6 kg:
    // Pulling force = 6 * 9.81 = 58.86 N > 49.05 N => Kinetic motion!
    let table = TablePulleySystem::new(10.0, 6.0, 0.5, 0.3)
        .expect("valid system")
        .with_gravity(9.81);

    assert!(!table.is_static_at_rest());
    assert_eq!(table.friction_state(), FrictionState::Kinetic);

    // a = (58.86 - 0.3 * 10 * 9.81) / 16 = (58.86 - 29.43) / 16 = 1.839375 m/s²
    let expected_a = 1.839375;
    assert!((table.acceleration() - expected_a).abs() < 1e-6);

    // T = m1 * (a + mu_k * g) = 10 * (1.839375 + 2.943) = 47.82375 N
    let expected_t = 47.82375;
    assert!((table.tension() - expected_t).abs() < 1e-6);
    assert!((table.friction_force() - 29.43).abs() < 1e-6);
}

#[test]
fn test_pulley_parameters_and_mutation() {
    let mut atwood = AtwoodMachine::new(1.0, 2.0).expect("valid");
    assert!(atwood.set_parameter("m1", 3.0).is_ok());
    assert!(atwood.set_parameter("m1", -1.0).is_err());
    assert!(atwood.set_parameter("invalid", 5.0).is_err());
    assert!(!atwood.formula_text().is_empty());

    let mut table = TablePulleySystem::new(5.0, 2.0, 0.4, 0.2).expect("valid");
    assert!(table.set_parameter("mu_s", 0.6).is_ok());
    assert!(table.set_parameter("mu_s", -0.1).is_err());
    assert!(table.set_parameter("m2", 0.0).is_err());
}
