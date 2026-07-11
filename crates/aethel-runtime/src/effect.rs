//! Effect runtime support.

use aethel_ir::lower::*;
use aethel_syntax::span::Span;
use anyhow::Result;

/// Effect handler trait.
pub trait EffectHandler {
    fn handle(&self, effect: &str, operation: &str, args: &[Value]) -> Result<Value>;
}

/// Runtime values.
#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Tuple(Vec<Value>),
    Struct(String, Vec<(String, Value)>),
    Enum(String, String, Vec<Value>),
    Claim(Box<Value>),
    Verified(Box<Value>, String), // value, policy
}

/// Effect definition for runtime.
#[derive(Debug, Clone)]
pub struct EffectDef {
    pub name: String,
    pub operations: Vec<OperationDef>,
}

#[derive(Debug, Clone)]
pub struct OperationDef {
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub ret_type: Option<IrType>,
}