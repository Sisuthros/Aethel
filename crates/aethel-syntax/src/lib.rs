//! Aethel syntax crate: lexer, parser, AST, and diagnostics.

// Diagnostic constructors intentionally mirror their stable public code names.
#![allow(non_snake_case)]

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
