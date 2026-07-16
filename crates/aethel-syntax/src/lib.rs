//! Aethel syntax crate: lexer, parser, AST, and diagnostics.

// Diagnostic constructors intentionally mirror their stable public code names.
#![allow(non_snake_case)]
// Legacy parser scaffolding is retained while the sound HIR checker replaces it incrementally.
#![allow(dead_code, unreachable_patterns, unused_imports, unused_variables)]
#![allow(clippy::manual_unwrap_or_default, clippy::unnecessary_map_or)]

pub mod ast;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod span;

pub use ast::*;
pub use diagnostic::*;
pub use lexer::*;
pub use parser::*;
pub use span::*;
