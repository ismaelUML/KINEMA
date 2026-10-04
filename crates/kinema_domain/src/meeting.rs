use crate::motion::{Motion1D, Mru};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Degree {
    Constant,
    Linear,
    Quadratic,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quadratic {
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl Quadratic {
    pub fn new(a: f64, b: f64, c: f64) -> Self {
        Self { a, b, c }
    }

    pub fn degree(&self) -> Degree {
        const EPS: f64 = 1e-12;
        if self.a.abs() > EPS {
            Degree::Quadratic
        } else if self.b.abs() > EPS {
            Degree::Linear
        } else {
            Degree::Constant
        }
    }

    pub fn discriminant(&self) -> f64 {
        self.b * self.b - 4.0 * self.a * self.c
    }

    pub fn vertex_t(&self) -> f64 {
        -self.b / (2.0 * self.a)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Roots {
    None,
    One(f64),
    Two((f64, f64)),
    Infinite,
}

pub fn meeting_times(q: Quadratic) -> Roots {
    match q.degree() {
        Degree::Constant => solve_constant(q),
        Degree::Linear => solve_linear(q),
        Degree::Quadratic => solve_quadratic(q),
    }
}

fn solve_constant(q: Quadratic) -> Roots {
    const EPS: f64 = 1e-12;
    if q.c.abs() < EPS {
        Roots::Infinite
    } else {
        Roots::None
    }
}

fn solve_linear(q: Quadratic) -> Roots {
    Roots::One(-q.c / q.b)
}

pub fn solve_quadratic(q: Quadratic) -> Roots {
    match q.discriminant().partial_cmp(&0.0) {
        Some(Ordering::Less) => Roots::None,
        Some(Ordering::Equal) => Roots::One(q.vertex_t()),
        Some(Ordering::Greater) => Roots::Two(stable_pair(q)),
        None => Roots::None,
    }
}

/// Why: textbook formula (-b +- sqrt(D))/(2a) suffers catastrophic cancellation when b^2 >> 4ac.
/// Stable citardauq/Muller form computes q = -0.5 * (b + sign(b) * sqrt(D)) and roots x1 = q/a, x2 = c/q.
fn stable_pair(q: Quadratic) -> (f64, f64) {
    let d = q.discriminant().sqrt();
    let sign_b = if q.b >= 0.0 { 1.0 } else { -1.0 };
    let temp = -0.5 * (q.b + sign_b * d);
    let r1 = temp / q.a;
    let r2 = q.c / temp;
    if r1 <= r2 {
        (r1, r2)
    } else {
        (r2, r1)
    }
}

/// Instant in time and space where two bodies collide or cross trajectories.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeetingInstant {
    pub time: f64,
    pub position: f64,
    pub is_past: bool,
}

impl MeetingInstant {
    pub fn new(time: f64, position: f64) -> Self {
        Self {
            time,
            position,
            is_past: time < 0.0,
        }
    }
}

/// Outcome of attempting to solve when two bodies meet.
#[derive(Debug, Clone, PartialEq)]
pub enum MeetingOutcome {
    CoincideAlways,
    NeverMeet,
    Single(MeetingInstant),
    Dual(MeetingInstant, MeetingInstant),
}

impl MeetingOutcome {
    /// Concise human-readable text for the UI inspector.
    pub fn diagnostic_message(&self) -> &'static str {
        match self {
            MeetingOutcome::CoincideAlways => "Bodies coincide for all time",
            MeetingOutcome::NeverMeet => "Bodies never meet",
            MeetingOutcome::Single(inst) => single_diagnostic(inst.is_past),
            MeetingOutcome::Dual(i1, i2) => dual_diagnostic(i1.is_past, i2.is_past),
        }
    }
}

fn single_diagnostic(is_past: bool) -> &'static str {
    if is_past {
        "Bodies met in the past"
    } else {
        "Bodies meet at one instant"
    }
}

fn dual_diagnostic(i1_past: bool, i2_past: bool) -> &'static str {
    if i1_past && i2_past {
        "Bodies met twice in the past"
    } else if i1_past || i2_past {
        "Bodies meet twice (one in the past)"
    } else {
        "Bodies meet at two instants"
    }
}

/// Solves 1D meeting between any two motions (MRU or MRUV):
/// 0.5 * (a_a - a_b) * t^2 + (v0_a - v0_b) * t + (x0_a - x0_b) = 0.
pub fn analyze_meeting(
    a: &(impl Motion1D + ?Sized),
    b: &(impl Motion1D + ?Sized),
) -> MeetingOutcome {
    let a_lead = 0.5 * (a.acceleration_at(0.0) - b.acceleration_at(0.0));
    let b_linear = a.velocity_at(0.0) - b.velocity_at(0.0);
    let c_const = a.position_at(0.0) - b.position_at(0.0);
    let q = Quadratic::new(a_lead, b_linear, c_const);

    match meeting_times(q) {
        Roots::Infinite => MeetingOutcome::CoincideAlways,
        Roots::None => MeetingOutcome::NeverMeet,
        Roots::One(t) => {
            let x = a.position_at(t);
            MeetingOutcome::Single(MeetingInstant::new(t, x))
        }
        Roots::Two((t1, t2)) => {
            let x1 = a.position_at(t1);
            let x2 = a.position_at(t2);
            MeetingOutcome::Dual(MeetingInstant::new(t1, x1), MeetingInstant::new(t2, x2))
        }
    }
}

/// Specialized helper for two MRU motions (backwards compatible).
pub fn analyze_mru_meeting(a: &Mru, b: &Mru) -> MeetingOutcome {
    analyze_meeting(a, b)
}
