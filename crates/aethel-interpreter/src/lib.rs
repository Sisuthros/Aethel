//! Fail-closed interpreter for validated Aethel IR.

// The symbolic interpreter prioritizes explicit fail-closed semantics while
// production effect handlers remain outside this crate.

pub mod sound_eval;
pub use sound_eval as eval;
pub use sound_eval::*;
