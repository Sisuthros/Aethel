//! Fail-closed interpreter for validated Aethel IR.

pub mod sound_eval;
pub use sound_eval as eval;
pub use sound_eval::*;
