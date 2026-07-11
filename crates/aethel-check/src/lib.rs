//! Type checking and epistemic type rules for Aethel.

pub mod checker;
pub mod epistemic;
pub mod types;

pub use checker::*;
pub use epistemic::*;
pub use types::*;