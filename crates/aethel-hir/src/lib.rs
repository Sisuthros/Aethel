//! HIR lowering from AST.

// Resolver scaffolding contains intentionally unused placeholders until module linking lands.
#![allow(unused_imports, unused_variables)]

pub mod lower;
pub mod resolve;

pub use lower::*;
pub use resolve::*;
