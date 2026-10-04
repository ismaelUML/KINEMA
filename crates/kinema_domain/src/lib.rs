//! Pure Core: Physics entities, laws, and meeting solvers.
//! This crate contains zero external dependencies and depends solely on std.

pub mod meeting;
pub mod motion;
pub mod scene;

pub use meeting::{meeting_times, Quadratic, Roots};
pub use motion::Motion1D;
pub use scene::Scene;
