use kinema_domain::meeting::{meeting_times, Quadratic, Roots};
use kinema_domain::motion::{Motion1D, Mru};

#[test]
fn test_mru_evaluation() {
    let car = Mru::new(10.0, 5.0);
    assert_eq!(car.position_at(0.0), 10.0);
    assert_eq!(car.position_at(2.0), 20.0);
    assert_eq!(car.velocity_at(2.0), 5.0);
    assert_eq!(car.acceleration_at(2.0), 0.0);
}

#[test]
fn test_meeting_solver_two_cars_preset() {
    // Car A: x0 = 0, v = 15
    // Car B: x0 = 100, v0 = -10, a = 2
    // 0.5*(0 - 2)*t^2 + (15 - (-10))*t + (0 - 100) = 0
    // -t^2 + 25t - 100 = 0 -> roots t = 5 s, t = 20 s
    let q = Quadratic::new(-1.0, 25.0, -100.0);
    let roots = meeting_times(q);
    match roots {
        Roots::Two((r1, r2)) => {
            assert!((r1 - 5.0).abs() < 1e-9);
            assert!((r2 - 20.0).abs() < 1e-9);
        }
        _ => panic!("Expected two roots: 5s and 20s, got {:?}", roots),
    }
}

#[test]
fn test_m1_linear_meeting_solver_relative_error() {
    use kinema_domain::meeting::{analyze_mru_meeting, MeetingOutcome};

    // Canonical M1 preset: Car A (0m, 15m/s), Car B (100m, -10m/s)
    let car_a = Mru::new(0.0, 15.0);
    let car_b = Mru::new(100.0, -10.0);

    let outcome = analyze_mru_meeting(&car_a, &car_b);
    match outcome {
        MeetingOutcome::Single(inst) => {
            let expected_t = 4.0;
            let rel_error = (inst.time - expected_t).abs() / expected_t;
            assert!(
                rel_error < 1e-9,
                "Relative error {} must be strictly < 1e-9 per M1 exit criteria",
                rel_error
            );
            assert_eq!(inst.position, 60.0);
            assert!(!inst.is_past);
            assert_eq!(outcome.diagnostic_message(), "Bodies meet at one instant");
        }
        _ => panic!("Expected single meeting at t=4s, got {:?}", outcome),
    }
}

#[test]
fn test_m1_meeting_edge_cases_and_diagnostics() {
    use kinema_domain::meeting::{analyze_mru_meeting, MeetingOutcome};

    // Coincide always: same x0 and same v
    let a1 = Mru::new(50.0, 20.0);
    let b1 = Mru::new(50.0, 20.0);
    let outcome1 = analyze_mru_meeting(&a1, &b1);
    assert_eq!(outcome1, MeetingOutcome::CoincideAlways);
    assert_eq!(outcome1.diagnostic_message(), "Bodies coincide for all time");

    // Never meet: different x0 and parallel v
    let a2 = Mru::new(0.0, 20.0);
    let b2 = Mru::new(50.0, 20.0);
    let outcome2 = analyze_mru_meeting(&a2, &b2);
    assert_eq!(outcome2, MeetingOutcome::NeverMeet);
    assert_eq!(outcome2.diagnostic_message(), "Bodies never meet");

    // Met in past: t* < 0
    let a3 = Mru::new(100.0, 15.0);
    let b3 = Mru::new(0.0, -10.0);
    let outcome3 = analyze_mru_meeting(&a3, &b3);
    match outcome3 {
        MeetingOutcome::Single(inst) => {
            assert!(inst.is_past);
            assert_eq!(inst.time, -4.0);
            assert_eq!(outcome3.diagnostic_message(), "Bodies met in the past");
        }
        _ => panic!("Expected meeting in the past, got {:?}", outcome3),
    }
}

#[test]
fn test_parametric_law_formatting_and_mutation() {
    use kinema_domain::motion::ParametricLaw;

    let mut car = Mru::new(0.0, 15.0);
    assert_eq!(car.formula_text(), "x(t) = 0.00 + 15.00 · t");

    car.set_parameter("v", -10.0).expect("v should be valid");
    car.set_parameter("x0", 100.0).expect("x0 should be valid");
    assert_eq!(car.formula_text(), "x(t) = 100.00 - 10.00 · t");
    assert!(car.set_parameter("unknown", 42.0).is_err());
}
