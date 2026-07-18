//! Aethel Check — sound HIR type checking and semantic IR.

pub mod sound_checker;
pub use sound_checker as checker;

// Compatibility-only HIR/IR conversion helpers. The sound checker owns semantics.
#[allow(unused_imports, unused_variables)]
pub mod types;
