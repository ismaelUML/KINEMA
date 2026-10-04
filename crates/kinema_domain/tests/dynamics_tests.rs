use kinema_domain::dynamics::{BlockDynamics, FrictionState};
use kinema_domain::motion::{Motion1D, ParametricLaw};

#[test]
fn test_horizontal_friction_static_threshold() {
    let mass = 5.0;
    let mu_s = 0.5;
    let mu_k = 0.3;
    let g = 9.81;

    // Normal force N = 5 * 9.81 = 49.05 N
    // Max static friction = 0.5 * 49.05 = 24.525 N
    // Below threshold: F_app = 20.0 N < 24.525 N
    let block_rest = BlockDynamics::horizontal(mass, mu_s, mu_k, 20.0).with_gravity(g);
    let fbd_rest = block_rest.free_body_diagram(0.0);
    assert_eq!(fbd_rest.state, FrictionState::Static);
    assert_eq!(fbd_rest.net_force, 0.0);
    assert_eq!(fbd_rest.friction, -20.0); // Exactly counteracts 20 N applied force
    assert_eq!(block_rest.acceleration_at_velocity(0.0), 0.0);
    assert_eq!(block_rest.position_at(10.0), 0.0);
    assert_eq!(block_rest.velocity_at(10.0), 0.0);

    // Above threshold: F_app = 30.0 N > 24.525 N
    let block_slide = BlockDynamics::horizontal(mass, mu_s, mu_k, 30.0).with_gravity(g);
    let fbd_slide = block_slide.free_body_diagram(0.0);
    assert_eq!(fbd_slide.state, FrictionState::Kinetic);
    // Kinetic friction fk = 0.3 * 49.05 = 14.715 N opposing motion
    assert!((fbd_slide.friction - (-14.715)).abs() < 1e-6);
    // Net force = 30.0 - 14.715 = 15.285 N
    assert!((fbd_slide.net_force - 15.285).abs() < 1e-6);
    // Acceleration a = 15.285 / 5 = 3.057 m/s^2
    let expected_a = 15.285 / 5.0;
    assert!((block_slide.acceleration_at(0.0) - expected_a).abs() < 1e-6);
    assert!((block_slide.velocity_at(2.0) - (expected_a * 2.0)).abs() < 1e-6);
    assert!((block_slide.position_at(2.0) - (0.5 * expected_a * 4.0)).abs() < 1e-6);
}

#[test]
fn test_incline_plane_sliding_angle_criterion() {
    let mass = 2.0;
    let mu_s = 0.6;
    let mu_k = 0.4;
    let g = 9.81;

    // Angle theta = 30 deg: tan(30) ≈ 0.57735 < mu_s (0.6) => static equilibrium
    let theta_30 = 30.0_f64.to_radians();
    let block_30 = BlockDynamics::new(mass, theta_30, mu_s, mu_k).with_gravity(g);
    let fbd_30 = block_30.free_body_diagram(0.0);
    assert_eq!(fbd_30.state, FrictionState::Static);
    assert_eq!(fbd_30.net_force, 0.0);
    assert_eq!(block_30.acceleration_at(0.0), 0.0);

    // Angle theta = 45 deg: tan(45) = 1.0 > mu_s (0.6) => slides down the incline
    let theta_45 = 45.0_f64.to_radians();
    let block_45 = BlockDynamics::new(mass, theta_45, mu_s, mu_k).with_gravity(g);
    let fbd_45 = block_45.free_body_diagram(0.0);
    assert_eq!(fbd_45.state, FrictionState::Kinetic);
    // Slide down incline: a = g * (sin(45) - mu_k * cos(45)) directed down (-x)
    let expected_a = -g * (theta_45.sin() - mu_k * theta_45.cos());
    assert!((block_45.acceleration_at(0.0) - expected_a).abs() < 1e-6);
    assert!(expected_a < 0.0);
}

#[test]
fn test_semi_implicit_euler_integrator_accuracy_bound() {
    let mass = 5.0;
    let mu_s = 0.5;
    let mu_k = 0.3;
    let g = 9.81;
    let block = BlockDynamics::horizontal(mass, mu_s, mu_k, 30.0).with_gravity(g);

    let dt: f64 = 1.0 / 240.0;
    let total_time: f64 = 2.0;
    let steps = (total_time / dt).round() as usize;

    let mut pos = 0.0;
    let mut vel = 0.0;

    for _ in 0..steps {
        let (next_pos, next_vel, _, _) = block.step_euler(pos, vel, dt);
        pos = next_pos;
        vel = next_vel;
    }

    let exact_pos = block.position_at(total_time);
    let exact_vel = block.velocity_at(total_time);

    // Semi-implicit Euler is exact in velocity for constant acceleration
    assert!((vel - exact_vel).abs() < 1e-9);
    // Position error is strictly bounded by O(dt)
    let pos_error = (pos - exact_pos).abs();
    assert!(
        pos_error < 0.02,
        "Integrator position error {} must be strictly < 0.02 m",
        pos_error
    );
}

#[test]
fn test_friction_stick_slip_zero_crossing_halt() {
    // Sliding block with v0 = 5 m/s, no driving force, stops under friction
    let mass = 2.0;
    let mu_s = 0.6;
    let mu_k = 0.4;
    let g = 9.81;
    let block = BlockDynamics::horizontal(mass, mu_s, mu_k, 0.0)
        .with_gravity(g)
        .with_initial_state(0.0, 5.0);

    // Stopping time t_s = v0 / (mu_k * g) = 5 / (0.4 * 9.81) ≈ 1.2742 s
    let stopping_t = block.stopping_time().expect("should have stopping time");
    assert!((stopping_t - (5.0 / (0.4 * 9.81))).abs() < 1e-4);

    // Prior to stopping: decelerating
    assert!(block.velocity_at(0.5) > 0.0);
    assert_eq!(block.acceleration_at(0.5), -0.4 * 9.81);

    // After stopping time: stuck at rest!
    assert_eq!(block.velocity_at(stopping_t + 1.0), 0.0);
    assert_eq!(block.acceleration_at(stopping_t + 1.0), 0.0);

    // Check with Euler step: simulate across stopping time
    let dt = 1.0 / 240.0;
    let mut pos = 0.0;
    let mut vel = 5.0;
    let total_steps = 400; // ~ 1.66 s, well beyond stopping time

    for _ in 0..total_steps {
        let (np, nv, _, _) = block.step_euler(pos, vel, dt);
        pos = np;
        vel = nv;
    }

    // Velocity must be exactly 0.0 with no numerical chatter
    assert_eq!(vel, 0.0);
    let fbd = block.free_body_diagram(vel);
    assert_eq!(fbd.state, FrictionState::Static);
    assert_eq!(fbd.net_force, 0.0);
}

#[test]
fn test_parametric_law_mutation_and_validation() {
    let mut block = BlockDynamics::default();
    assert!(block.set_parameter("mass", 10.0).is_ok());
    assert_eq!(block.mass, 10.0);
    assert!(block.set_parameter("mass", -1.0).is_err());

    assert!(block.set_parameter("theta_deg", 30.0).is_ok());
    assert!((block.theta - 30.0_f64.to_radians()).abs() < 1e-6);
    assert!(block.set_parameter("theta_deg", 95.0).is_err());

    assert!(block.set_parameter("mu_s", 0.7).is_ok());
    assert!(block.set_parameter("mu_k", 0.4).is_ok());
    assert!(block.set_parameter("mu_s", -0.1).is_err());
    assert!(block.set_parameter("f_app", 50.0).is_ok());
    assert_eq!(block.f_app, 50.0);
}
