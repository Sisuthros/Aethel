//! IR (Intermediate Representation) - typed, lowered HIR.

// The IR module still carries compatibility imports used by the legacy lowering path.
#![allow(unused_imports)]

pub mod lower;

pub use lower::*;
