//! Pure Core: Physics entities, laws, and meeting solvers.
//! This crate contains zero external dependencies and depends solely on std.

pub mod motion;
pub mod meeting;
pub mod scene;

pub use motion::Motion1D;
pub use meeting::{meeting_times, Quadratic, Roots};
pub use scene::Scene;
