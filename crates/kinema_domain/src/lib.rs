//! Pure Core: Physics entities, laws, and meeting solvers.
//! This crate contains zero external dependencies and depends solely on std.

pub mod meeting;
pub mod motion;
pub mod scene;

pub use meeting::{
    analyze_meeting, analyze_mru_meeting, meeting_times, MeetingInstant, MeetingOutcome, Quadratic,
    Roots,
};
pub use motion::{Motion, Motion1D, Mru, Mruv, ParametricLaw};
pub use scene::{Body, Scene};
