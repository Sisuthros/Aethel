//! Fail-closed symbolic interpreter for validated Aethel IR.
//!
//! The interpreter is intentionally conservative. It never manufactures a
//! `Verified` value from a generic call or a failed effect invocation.

use std::collections::HashMap;

use aethel_ir::lower::*;
use aethel_syntax::span::Span;
use anyhow::Result;

/// Runtime value tracked by the symbolic interpreter.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Tuple(Vec<Value>),
    Array(Vec<Value>),
    Claim {
        inner: Box<Value>,
        provenance: String,
    },
    Verified {
        inner: Box<Value>,
        policy: String,
        provenance: String,
    },
    Struct {
        name: String,
        fields: HashMap<String, Value>,
    },
    Error(String),
}

impl Value {
    pub fn is_claim(&self) -> bool {
        matches!(self, Self::Claim { .. })
    }

    pub fn is_verified(&self) -> bool {
        matches!(self, Self::Verified { .. })
    }

    pub fn unwrap_inner(&self) -> &Self {
        match self {
            Self::Claim { inner, .. } | Self::Verified { inner, .. } => inner.as_ref(),
            other => other,
        }
    }
}

/// One attempted effect invocation.
#[derive(Debug, Clone)]
pub struct EffectTrace {
    pub span: Span,
    pub effect_name: String,
    pub argument: Value,
    pub was_verified: bool,
    pub error: Option<String>,
}

/// Complete symbolic evaluation result.
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub return_value: Value,
    pub effect_trace: Vec<EffectTrace>,
    pub final_env: HashMap<String, Value>,
    pub verified_count: usize,
    pub claim_count: usize,
    pub policy_violations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn bind(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    pub fn remove(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.remove(name);
        }
    }

    pub fn snapshot(&self) -> HashMap<String, Value> {
        let mut result = HashMap::new();
        for scope in &self.scopes {
            result.extend(
                scope
                    .iter()
                    .map(|(name, value)| (name.clone(), value.clone())),
            );
        }
        result
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
enum Flow {
    Continue(Option<Value>),
    Return(Value),
}

/// Run-to-completion symbolic evaluator.
pub struct Evaluator {
    env: Env,
    trace: Vec<EffectTrace>,
    violations: Vec<String>,
    verified_count: usize,
    claim_count: usize,
    return_value: Value,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            env: Env::new(),
            trace: Vec::new(),
            violations: Vec::new(),
            verified_count: 0,
            claim_count: 0,
            return_value: Value::Unit,
        }
    }

    pub fn eval_module(&mut self, module: &IrModule) -> Result<EvalResult> {
        if let Some(entry) = self.find_entry(module) {
            self.eval_function(entry)?;
        } else {
            for item in &module.items {
                if let IrItem::Fn(function) = item {
                    self.eval_function(function)?;
                }
            }
        }
        Ok(self.finalise())
    }

    fn eval_function(&mut self, function: &IrFnDef) -> Result<()> {
        self.env.push_scope();
        self.bind_params(function);
        if let Some(body) = &function.body {
            match self.eval_block(body)? {
                Flow::Return(value) | Flow::Continue(Some(value)) => self.return_value = value,
                Flow::Continue(None) => self.return_value = Value::Unit,
            }
        }
        self.env.pop_scope();
        Ok(())
    }

    fn bind_params(&mut self, function: &IrFnDef) {
        for param in &function.params {
            let value = match &param.ty {
                IrType::Claim { .. } => Value::Claim {
                    inner: Box::new(Value::Unit),
                    provenance: format!("param:{}", param.name),
                },
                IrType::Verified { policy, .. } => Value::Verified {
                    inner: Box::new(Value::Unit),
                    policy: format_type_policy(policy),
                    provenance: format!("trusted-param:{}", param.name),
                },
                _ => Value::Unit,
            };
            self.env.bind(param.name.clone(), value);
        }
    }

    fn find_entry<'a>(&self, module: &'a IrModule) -> Option<&'a IrFnDef> {
        module.items.iter().find_map(|item| match item {
            IrItem::Fn(function) if function.name == "main" || function.name == "entry" => {
                Some(function)
            }
            _ => None,
        })
    }

    fn eval_block(&mut self, block: &IrBlock) -> Result<Flow> {
        self.env.push_scope();
        for statement in &block.stmts {
            match self.eval_stmt(statement)? {
                Flow::Continue(_) => {}
                Flow::Return(value) => {
                    self.env.pop_scope();
                    return Ok(Flow::Return(value));
                }
            }
        }
        let tail = block
            .tail
            .as_ref()
            .map(|expr| self.eval_expr(expr))
            .transpose()?;
        self.env.pop_scope();
        Ok(Flow::Continue(tail))
    }

    fn eval_stmt(&mut self, statement: &IrStmt) -> Result<Flow> {
        match statement {
            IrStmt::Let { name, init, .. } => {
                let value = init
                    .as_ref()
                    .map(|expr| self.eval_expr(expr))
                    .transpose()?
                    .unwrap_or(Value::Unit);
                self.env.bind(name.clone(), value);
                Ok(Flow::Continue(None))
            }
            IrStmt::Expr { expr, .. } => {
                self.eval_expr(expr)?;
                Ok(Flow::Continue(None))
            }
            IrStmt::Return { expr, .. } => {
                let value = expr
                    .as_ref()
                    .map(|expr| self.eval_expr(expr))
                    .transpose()?
                    .unwrap_or(Value::Unit);
                Ok(Flow::Return(value))
            }
            IrStmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => match self.eval_expr(cond)? {
                Value::Bool(true) => self.eval_block(then_branch),
                Value::Bool(false) => {
                    if let Some(statement) = else_branch {
                        self.eval_stmt(statement)
                    } else {
                        Ok(Flow::Continue(None))
                    }
                }
                other => {
                    self.record_violation(format!(
                        "non-boolean condition reached runtime: {other:?}"
                    ));
                    Ok(Flow::Continue(Some(Value::Error(
                        "non-boolean condition".into(),
                    ))))
                }
            },
            IrStmt::While { cond, body, .. } => {
                // Symbolic interpreter executes at most one loop iteration.
                if matches!(self.eval_expr(cond)?, Value::Bool(true)) {
                    if let Flow::Return(value) = self.eval_block(body)? {
                        return Ok(Flow::Return(value));
                    }
                }
                Ok(Flow::Continue(None))
            }
            IrStmt::For { iter, body, .. } => {
                let values = match self.eval_expr(iter)? {
                    Value::Array(values) | Value::Tuple(values) => values,
                    _ => Vec::new(),
                };
                if !values.is_empty() {
                    if let Flow::Return(value) = self.eval_block(body)? {
                        return Ok(Flow::Return(value));
                    }
                }
                Ok(Flow::Continue(None))
            }
            IrStmt::Match {
                scrutinee, arms, ..
            } => {
                self.eval_expr(scrutinee)?;
                if let Some(arm) = arms.first() {
                    let value = self.eval_expr(&arm.body)?;
                    Ok(Flow::Continue(Some(value)))
                } else {
                    Ok(Flow::Continue(None))
                }
            }
            IrStmt::Block { block, .. } => self.eval_block(block),
        }
    }

    fn eval_expr(&mut self, expr: &IrExpr) -> Result<Value> {
        match expr {
            IrExpr::Literal { lit, .. } => Ok(self.eval_literal(lit)),
            IrExpr::Path { path, .. } => {
                let name = path_name(path);
                Ok(self
                    .env
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| Value::Error(format!("unresolved runtime value `{name}`"))))
            }
            IrExpr::Call { args, .. } => {
                for arg in args {
                    self.eval_expr(arg)?;
                }
                Ok(Value::Error(
                    "generic function execution is not implemented".into(),
                ))
            }
            IrExpr::MethodCall {
                span,
                receiver,
                method,
                args,
                ..
            } => self.eval_effect_call(*span, receiver, method, args),
            IrExpr::Field { base, field, .. } => match self.eval_expr(base)? {
                Value::Struct { fields, .. } => Ok(fields
                    .get(field)
                    .cloned()
                    .unwrap_or_else(|| Value::Error(format!("missing field `{field}`")))),
                _ => Ok(Value::Error(format!(
                    "field access `{field}` on non-struct"
                ))),
            },
            IrExpr::Index { base, index, .. } => {
                let index = match self.eval_expr(index)? {
                    Value::Int(value) if value >= 0 => value as usize,
                    _ => {
                        return Ok(Value::Error(
                            "array index must be a non-negative int".into(),
                        ))
                    }
                };
                match self.eval_expr(base)? {
                    Value::Array(values) | Value::Tuple(values) => Ok(values
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| Value::Error("index out of bounds".into()))),
                    _ => Ok(Value::Error("indexing non-array value".into())),
                }
            }
            IrExpr::Unary { op, expr, .. } => {
                let value = self.eval_expr(expr)?;
                Ok(match (op, value) {
                    (IrUnaryOp::Neg, Value::Int(value)) => Value::Int(-value),
                    (IrUnaryOp::Neg, Value::Float(value)) => Value::Float(-value),
                    (IrUnaryOp::Not, Value::Bool(value)) => Value::Bool(!value),
                    (_, value) => Value::Error(format!("invalid unary operation on {value:?}")),
                })
            }
            IrExpr::Binary {
                op, left, right, ..
            } => {
                let left = self.eval_expr(left)?;
                let right = self.eval_expr(right)?;
                Ok(eval_binary(op, left, right))
            }
            IrExpr::Block { block, .. } => match self.eval_block(block)? {
                Flow::Return(value) | Flow::Continue(Some(value)) => Ok(value),
                Flow::Continue(None) => Ok(Value::Unit),
            },
            IrExpr::Verify {
                span,
                claim,
                policy,
            } => self.eval_verify(*span, claim, policy),
            IrExpr::Ask { .. } => {
                self.claim_count += 1;
                Ok(Value::Claim {
                    inner: Box::new(Value::Unit),
                    provenance: "ask".into(),
                })
            }
            IrExpr::CommitOnce { span, effect, args } => {
                let name = type_path_name(&effect.path);
                let value = args
                    .first()
                    .map(|arg| self.eval_expr(arg))
                    .transpose()?
                    .unwrap_or(Value::Error("missing commit_once argument".into()));
                self.record_effect(*span, name, value)
            }
            IrExpr::Reason { prompt, .. } => {
                self.claim_count += 1;
                Ok(Value::Claim {
                    inner: Box::new(Value::String(prompt.clone())),
                    provenance: "reason".into(),
                })
            }
            IrExpr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => match self.eval_expr(cond)? {
                Value::Bool(true) => self.eval_expr(then_branch),
                Value::Bool(false) => else_branch
                    .as_ref()
                    .map(|expr| self.eval_expr(expr))
                    .transpose()
                    .map(|value| value.unwrap_or(Value::Unit)),
                _ => Ok(Value::Error("non-boolean if condition".into())),
            },
            IrExpr::Return { expr, .. } | IrExpr::Break { expr, .. } => expr
                .as_ref()
                .map(|expr| self.eval_expr(expr))
                .transpose()
                .map(|value| value.unwrap_or(Value::Unit)),
            IrExpr::Continue { .. } => Ok(Value::Unit),
            IrExpr::Tuple { exprs, .. } => Ok(Value::Tuple(
                exprs
                    .iter()
                    .map(|expr| self.eval_expr(expr))
                    .collect::<Result<Vec<_>>>()?,
            )),
            IrExpr::Array { exprs, .. } => Ok(Value::Array(
                exprs
                    .iter()
                    .map(|expr| self.eval_expr(expr))
                    .collect::<Result<Vec<_>>>()?,
            )),
            IrExpr::Struct { path, fields, .. } => Ok(Value::Struct {
                name: type_path_name(path),
                fields: fields
                    .iter()
                    .map(|field| Ok((field.name.clone(), self.eval_expr(&field.expr)?)))
                    .collect::<Result<HashMap<_, _>>>()?,
            }),
            IrExpr::Let { init, .. } => self.eval_expr(init),
            IrExpr::Match {
                scrutinee, arms, ..
            } => {
                self.eval_expr(scrutinee)?;
                arms.first()
                    .map(|arm| self.eval_expr(&arm.body))
                    .transpose()
                    .map(|value| value.unwrap_or(Value::Unit))
            }
            IrExpr::New { args, .. } => {
                for arg in args {
                    self.eval_expr(arg)?;
                }
                Ok(Value::Unit)
            }
        }
    }

    fn eval_effect_call(
        &mut self,
        span: Span,
        receiver: &IrExpr,
        method: &str,
        args: &[IrExpr],
    ) -> Result<Value> {
        let argument = if let Some(first) = args.first() {
            self.eval_expr(first)?
        } else {
            self.eval_expr(receiver)?
        };
        self.record_effect(span, method.to_string(), argument)
    }

    fn record_effect(&mut self, span: Span, name: String, argument: Value) -> Result<Value> {
        let verified = argument.is_verified();
        let error = (!verified).then(|| "unverified claim".to_string());
        self.trace.push(EffectTrace {
            span,
            effect_name: name.clone(),
            argument: argument.clone(),
            was_verified: verified,
            error: error.clone(),
        });
        if verified {
            self.verified_count += 1;
            Ok(Value::Unit)
        } else {
            let message = format!("unverified effect `{name}` blocked at runtime");
            self.record_violation(message.clone());
            Ok(Value::Error(message))
        }
    }

    fn eval_verify(&mut self, span: Span, claim: &IrExpr, policy: &IrTypePath) -> Result<Value> {
        let value = self.eval_expr(claim)?;
        match value {
            Value::Claim { inner, provenance } => {
                self.verified_count += 1;
                Ok(Value::Verified {
                    inner,
                    policy: type_path_name(policy),
                    provenance: format!("verify:{provenance}"),
                })
            }
            other => {
                let message = format!("verify received non-Claim value at {span}: {other:?}");
                self.record_violation(message.clone());
                Ok(Value::Error(message))
            }
        }
    }

    fn record_violation(&mut self, message: String) {
        self.violations.push(message);
    }

    fn eval_literal(&self, literal: &IrLiteral) -> Value {
        match literal {
            IrLiteral::Unit { .. } => Value::Unit,
            IrLiteral::Bool { value, .. } => Value::Bool(*value),
            IrLiteral::Int { value, .. } => Value::Int(*value),
            IrLiteral::Float { value, .. } => Value::Float(*value),
            IrLiteral::String { value, .. } => Value::String(value.clone()),
        }
    }

    fn finalise(&self) -> EvalResult {
        EvalResult {
            return_value: self.return_value.clone(),
            effect_trace: self.trace.clone(),
            final_env: self.env.snapshot(),
            verified_count: self.verified_count,
            claim_count: self.claim_count,
            policy_violations: self.violations.clone(),
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

fn path_name(path: &IrExprPath) -> String {
    path.segments
        .last()
        .map_or_else(String::new, |segment| segment.name.clone())
}

fn type_path_name(path: &IrTypePath) -> String {
    path.segments
        .last()
        .map_or_else(String::new, |segment| segment.name.clone())
}

fn format_type_policy(policy: &IrType) -> String {
    match policy {
        IrType::Path { path, .. } => type_path_name(path),
        _ => "unknown-policy".into(),
    }
}

fn eval_binary(op: &IrBinaryOp, left: Value, right: Value) -> Value {
    match (op, left, right) {
        (IrBinaryOp::Add, Value::Int(left), Value::Int(right)) => Value::Int(left + right),
        (IrBinaryOp::Sub, Value::Int(left), Value::Int(right)) => Value::Int(left - right),
        (IrBinaryOp::Mul, Value::Int(left), Value::Int(right)) => Value::Int(left * right),
        (IrBinaryOp::Div, Value::Int(_), Value::Int(0)) => Value::Error("division by zero".into()),
        (IrBinaryOp::Div, Value::Int(left), Value::Int(right)) => Value::Int(left / right),
        (IrBinaryOp::Rem, Value::Int(_), Value::Int(0)) => Value::Error("remainder by zero".into()),
        (IrBinaryOp::Rem, Value::Int(left), Value::Int(right)) => Value::Int(left % right),
        (IrBinaryOp::Eq, left, right) => Value::Bool(left == right),
        (IrBinaryOp::Ne, left, right) => Value::Bool(left != right),
        (IrBinaryOp::Lt, Value::Int(left), Value::Int(right)) => Value::Bool(left < right),
        (IrBinaryOp::Le, Value::Int(left), Value::Int(right)) => Value::Bool(left <= right),
        (IrBinaryOp::Gt, Value::Int(left), Value::Int(right)) => Value::Bool(left > right),
        (IrBinaryOp::Ge, Value::Int(left), Value::Int(right)) => Value::Bool(left >= right),
        (IrBinaryOp::And, Value::Bool(left), Value::Bool(right)) => Value::Bool(left && right),
        (IrBinaryOp::Or, Value::Bool(left), Value::Bool(right)) => Value::Bool(left || right),
        (IrBinaryOp::Assign, _, right)
        | (IrBinaryOp::AddAssign, _, right)
        | (IrBinaryOp::SubAssign, _, right)
        | (IrBinaryOp::MulAssign, _, right)
        | (IrBinaryOp::DivAssign, _, right)
        | (IrBinaryOp::RemAssign, _, right) => right,
        (op, left, right) => Value::Error(format!(
            "unsupported binary operation {op:?} for {left:?} and {right:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aethel_syntax::span::{ByteOffset, FileId};

    fn span() -> Span {
        Span::new(FileId::new(0), ByteOffset::new(0), ByteOffset::new(1))
    }

    fn path(name: &str) -> IrExpr {
        IrExpr::Path {
            span: span(),
            path: IrExprPath {
                span: span(),
                segments: vec![IrPathSegment {
                    span: span(),
                    name: name.into(),
                    args: None,
                }],
            },
        }
    }

    fn literal_int(value: i64) -> IrExpr {
        IrExpr::Literal {
            span: span(),
            lit: IrLiteral::Int {
                span: span(),
                value,
            },
        }
    }

    #[test]
    fn generic_call_never_mints_verified_value() {
        let mut evaluator = Evaluator::new();
        let call = IrExpr::Call {
            span: span(),
            callee: Box::new(path("anything")),
            args: vec![],
        };
        assert!(matches!(
            evaluator.eval_expr(&call).unwrap(),
            Value::Error(_)
        ));
    }

    #[test]
    fn failed_effect_returns_error_not_verified() {
        let mut evaluator = Evaluator::new();
        evaluator.env.bind(
            "claim".into(),
            Value::Claim {
                inner: Box::new(Value::Int(1)),
                provenance: "test".into(),
            },
        );
        let call = IrExpr::MethodCall {
            span: span(),
            receiver: Box::new(path("claim")),
            method: "charge".into(),
            args: vec![],
        };
        assert!(matches!(
            evaluator.eval_expr(&call).unwrap(),
            Value::Error(_)
        ));
        assert_eq!(evaluator.violations.len(), 1);
    }

    #[test]
    fn verify_rejects_non_claim() {
        let mut evaluator = Evaluator::new();
        let verify = IrExpr::Verify {
            span: span(),
            claim: Box::new(literal_int(7)),
            policy: IrTypePath::single("Policy"),
        };
        assert!(matches!(
            evaluator.eval_expr(&verify).unwrap(),
            Value::Error(_)
        ));
        assert_eq!(evaluator.violations.len(), 1);
    }

    #[test]
    fn verified_effect_passes() {
        let mut evaluator = Evaluator::new();
        evaluator.env.bind(
            "verified".into(),
            Value::Verified {
                inner: Box::new(Value::Int(1)),
                policy: "Policy".into(),
                provenance: "test".into(),
            },
        );
        let call = IrExpr::MethodCall {
            span: span(),
            receiver: Box::new(path("verified")),
            method: "charge".into(),
            args: vec![],
        };
        assert_eq!(evaluator.eval_expr(&call).unwrap(), Value::Unit);
        assert!(evaluator.violations.is_empty());
        assert!(evaluator.trace[0].was_verified);
    }

    #[test]
    fn binary_add_is_not_fake_boolean() {
        let mut evaluator = Evaluator::new();
        let expr = IrExpr::Binary {
            span: span(),
            op: IrBinaryOp::Add,
            left: Box::new(literal_int(2)),
            right: Box::new(literal_int(3)),
        };
        assert_eq!(evaluator.eval_expr(&expr).unwrap(), Value::Int(5));
    }
}
