//! Aethel IR interpreter — run-to-completion evaluation.
//!
//! Walks the lowered IR and simulates execution, tracking
//! claim→verified flow at runtime. This enables:
//!
//! - **Policy enforcement testing** — verify that unverified claims
//!   cannot cross effect boundaries at runtime
//! - **Symbolic execution** — track data provenance through the program
//! - **Effect tracing** — record every effect invocation with its
//!   associated claim/verified status
//!
//! # Architecture
//!
//! ```text
//! IrModule
//!   └── IrItem::Fn(fn_def)
//!         └── IrBlock { stmts, tail }
//!               ├── IrStmt::Let { binding }
//!               ├── IrStmt::Expr { effect_call }
//!               ├── IrStmt::If { cond, then, else }
//!               └── IrExpr::EffectCall { claim, effect }
//! ```
//!
//! The evaluator walks statements in order, evaluating expressions
//! recursively. Every effect call is recorded in a trace for later
//! policy verification.

use std::collections::HashMap;

use aethel_ir::lower::*;
use aethel_syntax::span::Span;
use anyhow::{Context, Result};

// ──────────────────────────────────────────────
//  Value model
// ──────────────────────────────────────────────

/// Runtime value during interpretation.
#[derive(Debug, Clone)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    /// A `Claim<T>` — unverified model output.
    Claim {
        inner: Box<Value>,
        provenance: String,
    },
    /// A `Verified<T, Policy>` — verified via some policy.
    Verified {
        inner: Box<Value>,
        policy: String,
        provenance: String,
    },
    /// A structured value (struct instance).
    Struct {
        name: String,
        fields: HashMap<String, Value>,
    },
    /// Error sentinel — evaluation failed at this point.
    Error(String),
}

impl Value {
    pub fn is_claim(&self) -> bool {
        matches!(self, Value::Claim { .. })
    }

    pub fn is_verified(&self) -> bool {
        matches!(self, Value::Verified { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error(_))
    }

    /// Extract the inner value, unwrapping Claim/Verified wrappers.
    pub fn unwrap_inner(&self) -> &Value {
        match self {
            Value::Claim { inner, .. } | Value::Verified { inner, .. } => inner.as_ref(),
            other => other,
        }
    }
}

// ──────────────────────────────────────────────
//  Effect trace
// ──────────────────────────────────────────────

/// A single effect invocation recorded during evaluation.
#[derive(Debug, Clone)]
pub struct EffectTrace {
    pub span: Span,
    pub effect_name: String,
    /// The claim value passed to the effect.
    pub argument: Value,
    /// Whether the argument was verified before crossing the boundary.
    pub was_verified: bool,
    /// If verification failed, the reason.
    pub error: Option<String>,
}

/// Complete evaluation result.
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub return_value: Value,
    pub effect_trace: Vec<EffectTrace>,
    /// Map of variable names to their final values.
    pub final_env: HashMap<String, Value>,
    pub verified_count: usize,
    pub claim_count: usize,
    pub policy_violations: Vec<String>,
}

// ──────────────────────────────────────────────
//  Environment
// ──────────────────────────────────────────────

/// Evaluation environment — scope chain of bindings.
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
        self.scopes.pop();
    }

    pub fn bind(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v);
            }
        }
        None
    }

    pub fn snapshot(&self) -> HashMap<String, Value> {
        let mut all = HashMap::new();
        for scope in &self.scopes {
            all.extend(scope.iter().map(|(k, v)| (k.clone(), v.clone())));
        }
        all
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────
//  Evaluator
// ──────────────────────────────────────────────

/// The Aethel IR evaluator.
pub struct Evaluator {
    env: Env,
    trace: Vec<EffectTrace>,
    violations: Vec<String>,
    verified_count: usize,
    claim_count: usize,
}

impl Evaluator {
    pub fn new() -> Self {
        Self {
            env: Env::new(),
            trace: Vec::new(),
            violations: Vec::new(),
            verified_count: 0,
            claim_count: 0,
        }
    }

    // ── Module entry ─────────────────────────────

    /// Evaluate an entire IR module.
    /// Returns the result of the `main` or `entry` function, if any.
    pub fn eval_module(&mut self, module: &IrModule) -> Result<EvalResult> {
        // Build a function table
        let mut functions: HashMap<String, &IrFnDef> = HashMap::new();
        for item in &module.items {
            if let IrItem::Fn(f) = item {
                functions.insert(f.name.clone(), f);
            }
        }

        // Find entry point
        let entry_name = if functions.contains_key("main") {
            "main"
        } else if functions.contains_key("entry") {
            "entry"
        } else {
            // No entry point — evaluate all function bodies in order
            for item in &module.items {
                if let IrItem::Fn(f) = item {
                    if let Some(body) = &f.body {
                        self.eval_block(body)?;
                    }
                }
            }
            return Ok(self.finalise());
        };

        // Evaluate the entry function
        if let Some(entry_fn) = functions.get(entry_name) {
            if let Some(body) = &entry_fn.body {
                self.eval_block(body)?;
            }
        }

        Ok(self.finalise())
    }

    // ── Block evaluation ─────────────────────────

    fn eval_block(&mut self, block: &IrBlock) -> Result<Option<Value>> {
        self.env.push_scope();

        for stmt in &block.stmts {
            match stmt {
                IrStmt::Let {
                    name, init, ..
                } => {
                    let value = if let Some(init_expr) = init {
                        self.eval_expr(init_expr)?
                    } else {
                        Value::Unit
                    };
                    self.env.bind(name.clone(), value);
                }
                IrStmt::Expr { expr, .. } => {
                    self.eval_expr(expr)?;
                }
                IrStmt::Return { expr, .. } => {
                    let value = if let Some(e) = expr {
                        self.eval_expr(e)?
                    } else {
                        Value::Unit
                    };
                    self.env.pop_scope();
                    return Ok(Some(value));
                }
                IrStmt::If {
                    cond,
                    then_branch,
                    else_branch, ..
                } => {
                    let cond_val = self.eval_expr(cond)?;
                    let is_truthy = matches!(cond_val, Value::Bool(true));
                    if is_truthy {
                        self.eval_block(then_branch)?;
                    } else if let Some(else_stmt) = else_branch {
                        self.eval_stmt(else_stmt)?;
                    }
                }
                _ => {} // While, For, Match, Break, Continue — stub
            }
        }

        // Evaluate tail expression
        let tail_val = if let Some(tail) = &block.tail {
            Some(self.eval_expr(tail)?)
        } else {
            None
        };

        self.env.pop_scope();
        Ok(tail_val)
    }

    fn eval_stmt(&mut self, stmt: &IrStmt) -> Result<Option<Value>> {
        match stmt {
            IrStmt::Expr { expr, .. } => {
                self.eval_expr(expr)?;
                Ok(None)
            }
            IrStmt::Return { expr, .. } => {
                let value = if let Some(e) = expr {
                    self.eval_expr(e)?
                } else {
                    Value::Unit
                };
                Ok(Some(value))
            }
            IrStmt::If {
                cond,
                then_branch,
                else_branch, ..
            } => {
                let cond_val = self.eval_expr(cond)?;
                let is_truthy = matches!(cond_val, Value::Bool(true));
                if is_truthy {
                    self.eval_block(then_branch)?;
                } else if let Some(else_stmt) = else_branch {
                    self.eval_stmt(else_stmt)?;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    // ── Expression evaluation ────────────────────

    fn eval_expr(&mut self, expr: &IrExpr) -> Result<Value> {
        match expr {
            IrExpr::Literal { lit, .. } => Ok(self.eval_literal(lit)),

            IrExpr::Path { path, .. } => {
                let name = path.segments.last()
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                self.env.get(&name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("unbound variable: {name}"))
            }

            IrExpr::Call { callee: _, args, .. } => {
                // For now, simple function evaluation
                self.claim_count += 1;
                let mut arg_values = Vec::new();
                for arg in args {
                    arg_values.push(self.eval_expr(arg)?);
                }
                // Return a generic verified value
                Ok(Value::Claim {
                    inner: Box::new(Value::Unit),
                    provenance: format!("call_expr"),
                })
            }

            IrExpr::MethodCall { span, receiver, method, .. } => {
                let receiver_val = self.eval_expr(receiver)?;
                // Method calls are effect boundaries
                let is_verified = receiver_val.is_verified();
                if !is_verified {
                    self.violations.push(format!(
                        "unverified call to `{}`: argument is Claim, not Verified",
                        method,
                    ));
                }

                self.trace.push(EffectTrace {
                    span: span.clone(),
                    effect_name: method.clone(),
                    argument: receiver_val.clone(),
                    was_verified: is_verified,
                    error: if is_verified { None } else { Some("unverified claim".into()) },
                });

                if is_verified {
                    self.verified_count += 1;
                }
                self.claim_count += 1;

                Ok(Value::Verified {
                    inner: Box::new(Value::Unit),
                    policy: "effects".into(),
                    provenance: format!("method:{method}"),
                })
            }

            IrExpr::Field { base: _, field, .. } => {
                // Field access on struct — return unit for now
                Ok(Value::Unit)
            }

            IrExpr::Unary { expr: inner, .. } => {
                self.eval_expr(inner)
            }

            IrExpr::Binary { left, right, .. } => {
                let _l = self.eval_expr(left)?;
                let _r = self.eval_expr(right)?;
                Ok(Value::Bool(true))
            }

            IrExpr::Block { block, .. } => {
                let result = self.eval_block(block)?;
                Ok(result.unwrap_or(Value::Unit))
            }
            // Stub handlers for remaining expression types
            _ => Ok(Value::Unit),
        }
    }

    fn eval_literal(&self, lit: &IrLiteral) -> Value {
        match lit {
            IrLiteral::Unit { .. } => Value::Unit,
            IrLiteral::Bool { value, .. } => Value::Bool(*value),
            IrLiteral::Int { value, .. } => Value::Int(*value),
            IrLiteral::Float { value, .. } => Value::Float(*value),
            IrLiteral::String { value, .. } => Value::String(value.clone()),
        }
    }

    fn value_type_name(&self, val: &Value) -> &'static str {
        match val {
            Value::Claim { .. } => "Claim",
            Value::Verified { .. } => "Verified",
            Value::Unit => "()",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Struct { name: _, .. } => "struct",
            Value::Error(_) => "error",
        }
    }

    // ── Finalisation ─────────────────────────────

    fn finalise(&self) -> EvalResult {
        EvalResult {
            return_value: Value::Unit,
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
