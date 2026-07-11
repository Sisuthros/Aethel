//! HIR lowering from AST.

pub mod lower;
pub mod resolve;

pub use lower::*;
pub use resolve::*;