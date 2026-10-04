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
