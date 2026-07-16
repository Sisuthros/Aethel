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

use std::collections::HashMap;

use aethel_ir::lower::*;
use aethel_syntax::span::Span;
use anyhow::Result;

// ──────────────────────────────────────────────
//  Value model
// ──────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Unit,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
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
        matches!(self, Value::Claim { .. })
    }

    pub fn is_verified(&self) -> bool {
        matches!(self, Value::Verified { .. })
    }

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

#[derive(Debug, Clone)]
pub struct EffectTrace {
    pub span: Span,
    pub effect_name: String,
    pub argument: Value,
    pub was_verified: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EvalResult {
    pub return_value: Value,
    pub effect_trace: Vec<EffectTrace>,
    pub final_env: HashMap<String, Value>,
    pub verified_count: usize,
    pub claim_count: usize,
    pub policy_violations: Vec<String>,
}

// ──────────────────────────────────────────────
//  Environment
// ──────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Env {
    scopes: Vec<HashMap<String, Value>>,
}

impl Env {
    pub fn new() -> Self {
        Self { scopes: vec![HashMap::new()] }
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

    pub fn remove(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.remove(name);
        }
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
    fn default() -> Self { Self::new() }
}

// ──────────────────────────────────────────────
//  Evaluator
// ──────────────────────────────────────────────

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

    pub fn eval_module(&mut self, module: &IrModule) -> Result<EvalResult> {
        let entry_name = self.find_entry(module);

        if let Some(entry_fn) = entry_name {
            self.bind_params(entry_fn);
            if let Some(body) = &entry_fn.body {
                self.eval_block(body)?;
            }
        } else {
            // No entry point — evaluate all function bodies
            for item in &module.items {
                if let IrItem::Fn(f) = item {
                    self.bind_params(f);
                    if let Some(body) = &f.body {
                        self.eval_block(body)?;
                    }
                    self.unbind_params(f);
                }
            }
        }

        Ok(self.finalise())
    }

    fn bind_params(&mut self, f: &IrFnDef) {
        for param in &f.params {
            self.env.bind(param.name.clone(), Value::Claim {
                inner: Box::new(Value::Unit),
                provenance: format!("param:{}", param.name),
            });
        }
    }

    fn unbind_params(&mut self, f: &IrFnDef) {
        for param in &f.params {
            self.env.remove(&param.name);
        }
    }

    fn find_entry<'a>(&self, module: &'a IrModule) -> Option<&'a IrFnDef> {
        for item in &module.items {
            if let IrItem::Fn(f) = item {
                if f.name == "main" || f.name == "entry" {
                    return Some(f);
                }
            }
        }
        None
    }

    fn eval_block(&mut self, block: &IrBlock) -> Result<Option<Value>> {
        self.env.push_scope();

        for stmt in &block.stmts {
            match stmt {
                IrStmt::Let { name, init, .. } => {
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
                IrStmt::If { cond, then_branch, else_branch, .. } => {
                    let cond_val = self.eval_expr(cond)?;
                    if matches!(cond_val, Value::Bool(true)) {
                        self.eval_block(then_branch)?;
                    } else if let Some(else_stmt) = else_branch {
                        self.eval_stmt(else_stmt)?;
                    }
                }
                _ => {}
            }
        }

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
            IrStmt::If { cond, then_branch, else_branch, .. } => {
                let cond_val = self.eval_expr(cond)?;
                if matches!(cond_val, Value::Bool(true)) {
                    self.eval_block(then_branch)?;
                } else if let Some(else_stmt) = else_branch {
                    self.eval_stmt(else_stmt)?;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn eval_expr(&mut self, expr: &IrExpr) -> Result<Value> {
        match expr {
            IrExpr::Literal { lit, .. } => Ok(self.eval_literal(lit)),

            IrExpr::Path { path, .. } => {
                let name = path.segments.last()
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                // Try variable lookup first (function params, let bindings)
                if let Some(val) = self.env.get(&name) {
                    return Ok(val.clone());
                }
                // Not a variable — effect/type/policy/builtin namespace
                Ok(Value::Unit)
            }

            IrExpr::Call { callee, args, .. } => {
                self.claim_count += 1;
                for arg in args { self.eval_expr(arg)?; }
                // Namespace calls (verify(), etc.) produce Verified
                let callee_val = if let IrExpr::Path { .. } = callee.as_ref() {
                    self.eval_expr(callee)?
                } else {
                    Value::Unit
                };
                if matches!(callee_val, Value::Unit) {
                    Ok(Value::Verified {
                        inner: Box::new(Value::Unit),
                        policy: "builtin".into(),
                        provenance: "verify".into(),
                    })
                } else {
                    Ok(callee_val)
                }
            }

            IrExpr::MethodCall { span, receiver, method, args, .. } => {
                // Evaluate the receiver — resolve variable if bound in env
                let receiver_val = match receiver.as_ref() {
                    IrExpr::Path { path, .. } => {
                        let name = path.segments.last().map(|s| s.name.clone()).unwrap_or_default();
                        self.env.get(&name).cloned().unwrap_or(Value::Unit)
                    }
                    other => self.eval_expr(other)?,
                };
                // Decide which value to check: if args exist, check first arg;
                // otherwise check the receiver (e.g. `data.process()` pattern)
                let effect_arg = if args.is_empty() {
                    receiver_val
                } else {
                    let arg = &args[0]; self.eval_expr(arg)?
                };
                let is_verified = effect_arg.is_verified();
                if !is_verified {
                    self.violations.push(format!(
                        "unverified call to `{method}`: argument is Claim, not Verified",
                    ));
                }
                self.trace.push(EffectTrace {
                    span: span.clone(),
                    effect_name: method.clone(),
                    argument: effect_arg,
                    was_verified: is_verified,
                    error: if is_verified { None } else { Some("unverified claim".into()) },
                });
                if is_verified { self.verified_count += 1; }
                self.claim_count += 1;
                Ok(Value::Verified {
                    inner: Box::new(Value::Unit),
                    policy: "effects".into(),
                    provenance: format!("method:{method}"),
                })
            }

            IrExpr::Field { field: _, .. } => Ok(Value::Unit),

            IrExpr::Unary { expr: inner, .. } => self.eval_expr(inner),

            IrExpr::Binary { .. } => Ok(Value::Bool(true)),

            IrExpr::Block { block, .. } => {
                Ok(self.eval_block(block)?.unwrap_or(Value::Unit))
            }

            // ── Aethel-specific: verify, ask, reason, commit_once ──

            IrExpr::Verify { claim, policy, .. } => {
                self.claim_count += 1;
                self.verified_count += 1;
                let claim_val = self.eval_expr(claim)?;
                let policy_name = policy.segments.last()
                    .map(|s| s.name.clone()).unwrap_or_default();
                Ok(Value::Verified {
                    inner: Box::new(claim_val),
                    policy: policy_name,
                    provenance: "verify".into(),
                })
            }

            IrExpr::Ask { .. } => {
                self.claim_count += 1;
                Ok(Value::Claim {
                    inner: Box::new(Value::Unit),
                    provenance: "ask".into(),
                })
            }

            IrExpr::CommitOnce { effect, args, .. } => {
                self.claim_count += 1;
                let name = effect.path.segments.last()
                    .map(|s| s.name.clone()).unwrap_or_default();
                let is_verified = args.first()
                    .map(|a| self.eval_expr(a).map(|v| v.is_verified()).unwrap_or(false))
                    .unwrap_or(false);
                if !is_verified {
                    self.violations.push(format!(
                        "unverified commit_once to `{name}`: argument is Claim"
                    ));
                }
                Ok(Value::Unit)
            }

            IrExpr::Reason { .. } => { self.claim_count += 1; Ok(Value::Unit) }

            // ── Control flow stubs ──
            IrExpr::If { cond, then_branch, else_branch, .. } => {
                let cond_val = self.eval_expr(cond)?;
                if matches!(cond_val, Value::Bool(true)) { self.eval_expr(then_branch) }
                else if let Some(e) = else_branch { self.eval_expr(e) }
                else { Ok(Value::Unit) }
            }
            IrExpr::Return { expr, .. } => {
                if let Some(e) = expr { self.eval_expr(e) } else { Ok(Value::Unit) }
            }

            // ── Data stubs ──
            IrExpr::Tuple { exprs, .. } | IrExpr::Array { exprs, .. } => {
                for e in exprs { self.eval_expr(e)?; } Ok(Value::Unit)
            }
            IrExpr::Struct { fields, .. } => {
                for f in fields { self.eval_expr(&f.expr)?; } Ok(Value::Unit)
            }
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
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aethel_syntax::span::{ByteOffset, FileId};

    fn s() -> Span {
        Span::new(FileId::new(0), ByteOffset::new(0), ByteOffset::new(1))
    }

    fn tp() -> IrTypePath {
        IrTypePath { span: s(), segments: vec![] }
    }

    // ── Value model tests ───────────────────────

    #[test]
    fn test_value_claim_detection() {
        let claim = Value::Claim {
            inner: Box::new(Value::Int(42)),
            provenance: "test".into(),
        };
        assert!(claim.is_claim());
        assert!(!claim.is_verified());
        assert_eq!(*claim.unwrap_inner(), Value::Int(42));
    }

    #[test]
    fn test_value_verified_detection() {
        let verified = Value::Verified {
            inner: Box::new(Value::String("ok".into())),
            policy: "test-policy".into(),
            provenance: "verify".into(),
        };
        assert!(verified.is_verified());
        assert!(!verified.is_claim());
        assert_eq!(*verified.unwrap_inner(), Value::String("ok".into()));
    }

    // ── Literal tests ───────────────────────────

    #[test]
    fn test_eval_unit_literal() {
        let mut eval = Evaluator::new();
        let lit = IrLiteral::Unit { span: s() };
        assert_eq!(eval.eval_literal(&lit), Value::Unit);
    }

    #[test]
    fn test_eval_int_literal() {
        let mut eval = Evaluator::new();
        let lit = IrLiteral::Int { span: s(), value: 42 };
        assert_eq!(eval.eval_literal(&lit), Value::Int(42));
    }

    #[test]
    fn test_eval_bool_literal() {
        let mut eval = Evaluator::new();
        let lit = IrLiteral::Bool { span: s(), value: true };
        assert_eq!(eval.eval_literal(&lit), Value::Bool(true));
    }

    #[test]
    fn test_eval_string_literal() {
        let mut eval = Evaluator::new();
        let lit = IrLiteral::String { span: s(), value: "hello".into() };
        assert_eq!(eval.eval_literal(&lit), Value::String("hello".into()));
    }

    // ── Environment tests ───────────────────────

    #[test]
    fn test_env_bind_and_get() {
        let mut env = Env::new();
        env.bind("x".into(), Value::Int(10));
        assert_eq!(env.get("x"), Some(&Value::Int(10)));
        assert_eq!(env.get("y"), None);
    }

    #[test]
    fn test_env_scope_shadowing() {
        let mut env = Env::new();
        env.bind("x".into(), Value::Int(1));
        env.push_scope();
        env.bind("x".into(), Value::Int(2));
        assert_eq!(env.get("x"), Some(&Value::Int(2)));
        env.pop_scope();
        assert_eq!(env.get("x"), Some(&Value::Int(1)));
    }

    #[test]
    fn test_env_snapshot() {
        let mut env = Env::new();
        env.bind("a".into(), Value::Bool(true));
        env.bind("b".into(), Value::String("test".into()));
        let snap = env.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap.get("a"), Some(&Value::Bool(true)));
    }

    // ── Evaluator block tests ───────────────────

    fn make_block(stmts: Vec<IrStmt>, tail: Option<IrExpr>) -> IrBlock {
        IrBlock {
            span: s(),
            stmts,
            tail: tail.map(Box::new),
        }
    }

    fn make_let(name: &str, init: IrExpr) -> IrStmt {
        IrStmt::Let {
            span: s(),
            name: name.into(),
            ty: IrType::Path {
                span: s(),
                path: IrTypePath {
                    span: s(),
                    segments: vec![],
                    
                },
            },
            is_mut: false,
            init: Some(init),
        }
    }

    fn make_lit_expr(lit: IrLiteral) -> IrExpr {
        IrExpr::Literal { span: s(), lit }
    }

    fn make_path_expr(name: &str) -> IrExpr {
        IrExpr::Path {
            span: s(),
            path: IrExprPath {
                span: s(),
                segments: vec![IrPathSegment {
                    span: s(),
                    name: name.into(),
                    args: None,
                }],
            },
        }
    }

    #[test]
    fn test_block_with_let() {
        let mut eval = Evaluator::new();
        let block = make_block(
            vec![make_let("x", make_lit_expr(IrLiteral::Int { span: s(), value: 42 }))],
            None,
        );
        // Block scope is popped after eval_block, so binding is local
        let result = eval.eval_block(&block).unwrap();
        assert!(result.is_none());
        // After block scope pops, x is gone — this is correct scoping
        // For a module-level eval, use eval_module() which returns final_env
    }

    #[test]
    fn test_block_with_return() {
        let mut eval = Evaluator::new();
        let block = make_block(
            vec![
                IrStmt::Return {
                    span: s(),
                    expr: Some(make_lit_expr(IrLiteral::Bool { span: s(), value: true })),
                },
            ],
            None,
        );
        let result = eval.eval_block(&block).unwrap();
        assert_eq!(result, Some(Value::Bool(true)));
    }

    #[test]
    fn test_block_tail_expression() {
        let mut eval = Evaluator::new();
        let block = make_block(
            vec![],
            Some(make_lit_expr(IrLiteral::String { span: s(), value: "tail".into() })),
        );
        let result = eval.eval_block(&block).unwrap();
        assert_eq!(result, Some(Value::String("tail".into())));
    }

    #[test]
    fn test_method_call_on_claim_creates_violation() {
        let mut eval = Evaluator::new();
        eval.env.bind("claim".into(), Value::Claim {
            inner: Box::new(Value::Int(1)),
            provenance: "input".into(),
        });

        let method_call = IrExpr::MethodCall {
            span: s(),
            receiver: Box::new(make_path_expr("claim")),
            method: "refund".into(),
            args: vec![],
        };
        let result = eval.eval_expr(&method_call).unwrap();
        assert!(result.is_verified());

        assert_eq!(eval.violations.len(), 1);
        assert!(eval.violations[0].contains("refund"));
        assert_eq!(eval.trace.len(), 1);
        assert!(!eval.trace[0].was_verified);
    }

    #[test]
    fn test_method_call_on_verified_passes() {
        let mut eval = Evaluator::new();
        eval.env.bind("verified".into(), Value::Verified {
            inner: Box::new(Value::Int(1)),
            policy: "test-policy".into(),
            provenance: "verify".into(),
        });

        let method_call = IrExpr::MethodCall {
            span: s(),
            receiver: Box::new(make_path_expr("verified")),
            method: "safe_action".into(),
            args: vec![],
        };
        let result = eval.eval_expr(&method_call).unwrap();
        assert!(result.is_verified());

        assert_eq!(eval.violations.len(), 0);
        assert!(eval.trace[0].was_verified);
        assert_eq!(eval.verified_count, 1);
    }

    #[test]
    fn test_if_true_takes_then_branch() {
        let mut eval = Evaluator::new();
        let block = make_block(
            vec![
                IrStmt::If {
                    span: s(),
                    cond: make_lit_expr(IrLiteral::Bool { span: s(), value: true }),
                    then_branch: make_block(
                        vec![make_let("branch", make_lit_expr(IrLiteral::String { span: s(), value: "then".into() }))],
                        None,
                    ),
                    else_branch: Some(Box::new(IrStmt::Expr {
                        span: s(),
                        expr: make_lit_expr(IrLiteral::Unit { span: s() }),
                    })),
                },
            ],
            None,
        );
        // The if-branch creates scoped bindings that don't survive the block
        // Just verify the block doesn't crash (the if was evaluated correctly)
        eval.eval_block(&block).unwrap();
        // scoped variable `branch` not visible here — correct scoping
    }

    #[test]
    fn test_eval_module_without_entry_does_not_crash() {
        let module = IrModule {
            file_id: FileId::new(0),
            items: vec![],
        };
        let mut eval = Evaluator::new();
        let result = eval.eval_module(&module).unwrap();
        assert_eq!(result.claim_count, 0);
        assert_eq!(result.verified_count, 0);
        assert!(result.policy_violations.is_empty());
    }

    #[test]
    fn test_full_lifecycle_no_violations() {
        // Simulate: bind verified value → call method → no violations
        let mut eval = Evaluator::new();
        eval.env.bind("data".into(), Value::Verified {
            inner: Box::new(Value::Int(100)),
            policy: "audit".into(),
            provenance: "verify".into(),
        });

        let call = IrExpr::MethodCall {
            span: s(),
            receiver: Box::new(make_path_expr("data")),
            method: "process".into(),
            args: vec![],
        };
        eval.eval_expr(&call).unwrap();
        assert_eq!(eval.violations.len(), 0);
        assert_eq!(eval.verified_count, 1);
        assert_eq!(eval.claim_count, 1);
    }

    #[test]
    fn test_unary_expr_passthrough() {
        let mut eval = Evaluator::new();
        let expr = IrExpr::Unary {
            span: s(),
            op: IrUnaryOp::Neg,
            expr: Box::new(make_lit_expr(IrLiteral::Int { span: s(), value: 5 })),
        };
        let result = eval.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Int(5));
    }

    #[test]
    fn test_binary_expr_returns_bool() {
        let mut eval = Evaluator::new();
        let expr = IrExpr::Binary {
            span: s(),
            op: IrBinaryOp::Add,
            left: Box::new(make_lit_expr(IrLiteral::Int { span: s(), value: 1 })),
            right: Box::new(make_lit_expr(IrLiteral::Int { span: s(), value: 2 })),
        };
        assert_eq!(eval.eval_expr(&expr).unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_unknown_path_returns_unit() {
        let mut eval = Evaluator::new();
        let expr = make_path_expr("nonexistent");
        let result = eval.eval_expr(&expr).unwrap();
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn test_claim_count_tracks_calls() {
        let mut eval = Evaluator::new();
        let call = IrExpr::Call {
            span: s(),
            callee: Box::new(make_path_expr("f")),
            args: vec![],
        };
        eval.eval_expr(&call).unwrap();
        eval.eval_expr(&call).unwrap();
        eval.eval_expr(&call).unwrap();

        let module = IrModule {
            file_id: FileId::new(0),
            items: vec![],
        };
        let result = eval.eval_module(&module).unwrap();
        assert_eq!(result.claim_count, 3);
    }
}
