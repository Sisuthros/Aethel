//! Main type checker orchestration.

use aethel_hir::lower::HirModule;
use aethel_ir::lower::*;
use aethel_syntax::diagnostic::{Diagnostics, DiagnosticCode, DiagnosticSeverity};
use aethel_syntax::span::{FileId, Span};
use aethel_effects::EffectRegistry;
use crate::types::{HirExprSpan, check_assignable};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Type checking context.
pub struct CheckContext {
    pub file_id: FileId,
    pub diagnostics: Diagnostics,
    pub effect_registry: EffectRegistry,
    pub type_env: TypeEnvironment,
    pub policy_registry: PolicyRegistry,
    pub current_fn_return_type: Option<IrType>,
}

/// Type environment for checking with proper scoping.
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    pub variables: IndexMap<String, VariableInfo>,
    pub type_defs: IndexMap<String, TypeDefinition>,
    pub scopes: Vec<Vec<String>>,
}

impl TypeEnvironment {
    pub fn enter_scope(&mut self) {
        self.scopes.push(Vec::new());
    }

    pub fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            for name in scope {
                self.variables.shift_remove(&name);
            }
        }
    }

    pub fn add_variable(&mut self, name: String, info: VariableInfo) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.push(name.clone());
        }
        self.variables.insert(name, info);
    }

    pub fn get_variable(&self, name: &str) -> Option<&VariableInfo> {
        self.variables.get(name)
    }
}

#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub ty: IrType,
    pub is_mut: bool,
    pub is_linear: bool,
}

#[derive(Debug, Clone)]
pub struct TypeDefinition {
    pub kind: TypeDefKind,
    pub generics: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum TypeDefKind {
    Struct { fields: IndexMap<String, IrType> },
    Enum { variants: IndexMap<String, Vec<IrType>> },
    TypeAlias { ty: IrType },
    Policy { claims: IndexMap<String, PolicyClaim> },
}

#[derive(Debug, Clone)]
pub struct PolicyClaim {
    pub ty: IrType,
    pub evidence: Vec<EvidenceKind>,
}

#[derive(Debug, Clone)]
pub enum EvidenceKind {
    SignedAttestation,
    CryptographicProof,
    AuditLog,
    HumanReview,
    Custom(String),
}

/// Policy registry for epistemic types.
#[derive(Debug, Default)]
pub struct PolicyRegistry {
    pub policies: IndexMap<String, PolicyDefinition>,
}

#[derive(Debug, Clone)]
pub struct PolicyDefinition {
    pub name: String,
    pub claims: IndexMap<String, PolicyClaim>,
}

impl CheckContext {
    pub fn new(file_id: FileId) -> Self {
        Self {
            file_id,
            diagnostics: Diagnostics::new(),
            effect_registry: EffectRegistry::default(),
            type_env: TypeEnvironment::default(),
            policy_registry: PolicyRegistry::default(),
            current_fn_return_type: None,
        }
    }

    pub fn error(&mut self, code: DiagnosticCode, message: &str, span: Span) {
        use aethel_syntax::diagnostic::DiagnosticBuilder;
        let diag = DiagnosticBuilder::error(code, message).primary_label(span, "here").build();
        self.diagnostics.push(diag);
    }

    pub fn note(&mut self, code: DiagnosticCode, message: &str, span: Span) {
        use aethel_syntax::diagnostic::DiagnosticBuilder;
        let diag = DiagnosticBuilder::note_severity(code, message).primary_label(span, "here").build();
        self.diagnostics.push(diag);
    }
}

/// Check a module and produce IR with full type checking.
/// Uses type structure (Claim<T> vs Verified<T, Policy>) not function names.
pub fn check_module(module: &aethel_syntax::ast::Module, file_id: FileId) -> (IrModule, Diagnostics) {
    let mut ctx = CheckContext::new(file_id);

    // Phase 1: Collect effect definitions from source declarations
    collect_effect_defs_from_source(&mut ctx, module);
    // Phase 2: Collect policy definitions from source declarations
    collect_policy_defs_from_source(&mut ctx, module);
    // Phase 3: Collect struct/type definitions from source
    collect_type_defs_from_source(&mut ctx, module);
    // Phase 4: Type-check each function body
    check_function_bodies(&mut ctx, module);

    // Phase 5: Lower AST items to IR items for the interpreter
    let ir_items = collect_ir_items(module);

    let ir_module = IrModule {
        file_id,
        items: ir_items,
    };

    (ir_module, ctx.diagnostics)
}

/// Check a HIR module and produce IR with full type checking.
pub fn check_hir_module(module: &aethel_hir::lower::HirModule, file_id: FileId) -> (IrModule, Diagnostics) {
    let mut ctx = CheckContext::new(file_id);

    // Phase 1: Collect effect definitions from HIR
    for item in &module.items {
        if let aethel_hir::lower::HirItem::Effect(e) = item {
            let ops: Vec<aethel_effects::EffectOperation> = e.operations.iter().map(|op| {
                aethel_effects::EffectOperation {
                    name: op.name.clone(),
                    params: op.params.iter().map(|p| aethel_effects::EffectParam {
                        name: p.name.clone(),
                        ty: crate::types::lower_hir_type(&p.ty),
                    }).collect(),
                    ret_type: op.ret_type.as_ref().map(|t| crate::types::lower_hir_type(t)),
                }
            }).collect();
            ctx.effect_registry.effects.insert(e.name.clone(), aethel_effects::EffectDefinition {
                name: e.name.clone(),
                operations: ops,
            });
        }
    }

    // Phase 2: Collect policy definitions
    for item in &module.items {
        collect_policies(&mut ctx, item);
    }

    // Phase 3: Collect type definitions
    for item in &module.items {
        collect_type_defs(&mut ctx, item);
    }

    // Phase 4: Type-check and lower each item to IR
    let mut ir_items: Vec<IrItem> = Vec::new();
    for item in &module.items {
        if let Some(ir_item) = check_item(&mut ctx, item) {
            ir_items.push(ir_item);
        }
    }

    let ir_module = IrModule {
        file_id,
        items: ir_items,
    };

    (ir_module, ctx.diagnostics)
}

/// Collect effect definitions from source AST items.
fn collect_effect_defs_from_source(ctx: &mut CheckContext, module: &aethel_syntax::ast::Module) {
    for item in &module.items {
        if let aethel_syntax::ast::Item::Effect(e) = item {
            let ops: Vec<aethel_effects::EffectOperation> = e.operations.iter().map(|op| {
                aethel_effects::EffectOperation {
                    name: op.name.name.clone(),
                    params: op.params.iter().map(|p| aethel_effects::EffectParam {
                        name: p.name.name.clone(),
                        ty: ir_type_from_ast_type(&p.ty),
                    }).collect(),
                    ret_type: op.ret_type.as_ref().map(|t| ir_type_from_ast_type(t)),
                }
            }).collect();
            
            ctx.effect_registry.effects.insert(e.name.name.clone(), aethel_effects::EffectDefinition {
                name: e.name.name.clone(),
                operations: ops,
            });
        }
    }
}

/// Collect policy definitions from source.
fn collect_policy_defs_from_source(ctx: &mut CheckContext, module: &aethel_syntax::ast::Module) {
    for item in &module.items {
        if let aethel_syntax::ast::Item::Policy(p) = item {
            let mut claims = IndexMap::new();
            for claim in &p.claims {
                let evidence: Vec<EvidenceKind> = claim.evidence.iter().map(|e| match &e.kind {
                    aethel_syntax::ast::EvidenceKind::SignedAttestation => EvidenceKind::SignedAttestation,
                    aethel_syntax::ast::EvidenceKind::CryptographicProof => EvidenceKind::CryptographicProof,
                    aethel_syntax::ast::EvidenceKind::AuditLog => EvidenceKind::AuditLog,
                    aethel_syntax::ast::EvidenceKind::HumanReview => EvidenceKind::HumanReview,
                    aethel_syntax::ast::EvidenceKind::Custom(s) => EvidenceKind::Custom(s.clone()),
                }).collect();
                claims.insert(claim.name.name.clone(), PolicyClaim {
                    ty: ir_type_from_ast_type(&claim.ty),
                    evidence,
                });
            }
            ctx.policy_registry.policies.insert(p.name.name.clone(), PolicyDefinition {
                name: p.name.name.clone(),
                claims,
            });
        }
    }
}

/// Collect struct/type definitions from source.
fn collect_type_defs_from_source(ctx: &mut CheckContext, module: &aethel_syntax::ast::Module) {
    for item in &module.items {
        if let aethel_syntax::ast::Item::Struct(s) = item {
            let mut fields = IndexMap::new();
            for field in &s.fields {
                fields.insert(field.name.name.clone(), ir_type_from_ast_type(&field.ty));
            }
            ctx.type_env.type_defs.insert(s.name.name.clone(), TypeDefinition {
                kind: TypeDefKind::Struct { fields },
                generics: s.generics.iter().map(|g| g.name.name.clone()).collect(),
            });
        }
    }
}

/// Convert an AST type to a display string for diagnostics.
fn ast_type_to_string(ty: &aethel_syntax::ast::Type) -> String {
    use aethel_syntax::ast::Type;
    match ty {
        Type::Unit { .. } => "()".to_string(),
        Type::Never { .. } => "!".to_string(),
        Type::Bool { .. } => "bool".to_string(),
        Type::Int { .. } => "int".to_string(),
        Type::Float { .. } => "float".to_string(),
        Type::String { .. } => "string".to_string(),
        Type::Path { path, .. } => path.segments.iter().map(|s| s.name.name.clone()).collect::<Vec<_>>().join("::"),
        Type::Owned { ty, .. } => format!("owned {}", ast_type_to_string(ty)),
        Type::Ref { ty, .. } => format!("&{}", ast_type_to_string(ty)),
        Type::Claim { ty, .. } => format!("Claim<{}>", ast_type_to_string(ty)),
        Type::Verified { ty, policy, .. } => format!("Verified<{}, {}>", ast_type_to_string(ty), ast_type_to_string(policy)),
        Type::Tuple { types, .. } => format!("({})", types.iter().map(ast_type_to_string).collect::<Vec<_>>().join(", ")),
        Type::Array { ty, .. } => format!("[{}]", ast_type_to_string(ty)),
        Type::Fn { params, ret, .. } => format!("fn({}) -> {}", params.iter().map(ast_type_to_string).collect::<Vec<_>>().join(", "), ast_type_to_string(ret)),
    }
}

/// Convert AST type to IR type for type env
fn ir_type_from_ast_type(ty: &aethel_syntax::ast::Type) -> IrType {
    use aethel_syntax::ast::Type;
    use aethel_syntax::span::Spanned;
    let span = ty.span();
    match ty {
        Type::Unit { .. } => IrType::Unit { span },
        Type::Never { .. } => IrType::Never { span },
        Type::Bool { .. } => IrType::Bool { span },
        Type::Int { .. } => IrType::Int { span },
        Type::Float { .. } => IrType::Float { span },
        Type::String { .. } => IrType::String { span },
        Type::Path { path, .. } => {
            let name = path.segments.first().map(|s| s.name.name.clone()).unwrap_or_default();
            IrType::Path { span, path: IrTypePath {
                span,
                segments: vec![IrPathSegment { span, name, args: None }],
            }}
        }
        Type::Owned { ty, .. } => IrType::Owned { span, ty: Box::new(ir_type_from_ast_type(ty)) },
        Type::Ref { ty, is_mut, .. } => IrType::Ref { span, is_mut: *is_mut, ty: Box::new(ir_type_from_ast_type(ty)) },
        Type::Claim { ty, .. } => IrType::Claim { span, ty: Box::new(ir_type_from_ast_type(ty)) },
        Type::Verified { ty, policy, .. } => IrType::Verified { span, ty: Box::new(ir_type_from_ast_type(ty)), policy: Box::new(ir_type_from_ast_type(policy)) },
        Type::Tuple { types, .. } => IrType::Tuple { span, types: types.iter().map(ir_type_from_ast_type).collect() },
        Type::Array { ty, .. } => IrType::Array { span, ty: Box::new(ir_type_from_ast_type(ty)), size: None },
        Type::Fn { params, ret, .. } => IrType::Fn { span, params: params.iter().map(ir_type_from_ast_type).collect(), ret: Box::new(ir_type_from_ast_type(ret)), effects: crate::types::lower_effect_set(&aethel_hir::lower::HirEffectSet { span: aethel_syntax::span::Span::new(FileId::new(0), aethel_syntax::span::ByteOffset(0), aethel_syntax::span::ByteOffset(0)), effects: vec![] }) },
    }
}

/// Check function bodies for epistemic violations based on TYPES, not names.
fn check_function_bodies(ctx: &mut CheckContext, module: &aethel_syntax::ast::Module) {
    use aethel_syntax::ast::Item;

    for item in &module.items {
        if let Item::Fn(f) = item {
            // Set current function's return type for checking return statements
            ctx.current_fn_return_type = f.ret_type.as_ref().map(|t| ir_type_from_ast_type(t));

            ctx.type_env.enter_scope();
            // Add parameters to type environment
            for param in &f.params {
                ctx.type_env.add_variable(
                    param.name.name.clone(),
                    VariableInfo {
                        ty: ir_type_from_ast_type(&param.ty),
                        is_mut: param.is_mut,
                        is_linear: false,
                    },
                );
            }
            if let Some(body) = &f.body {
                check_block_for_epistemic_violations(ctx, body, &f.effects.effects);
            }
            ctx.type_env.exit_scope();
            ctx.current_fn_return_type = None;
        }
    }
}

fn check_block_for_epistemic_violations(
    ctx: &mut CheckContext,
    block: &aethel_syntax::ast::Block,
    declared_effects: &[aethel_syntax::ast::EffectRef],
) {
    use aethel_syntax::ast::{Stmt, Expr};
    ctx.type_env.enter_scope();

    // Check all statements
    for stmt in &block.stmts {
        match stmt {
            Stmt::Expr { expr, .. } => {
                check_expr_for_epistemic_violations(ctx, expr, declared_effects);
            }
            Stmt::Return { expr, span, .. } => {
                // TYPE CHECK: return expression vs function return type
                if let (Some(ret_ty), Some(ret_expr)) = (&ctx.current_fn_return_type, expr) {
                    let deref: &aethel_syntax::ast::Expr = ret_expr;
                    if let Expr::Path { path, .. } = deref {
                        let name = path.segments.last().map(|s| s.name.name.as_str()).unwrap_or("");
                        if let Some(var_info) = ctx.type_env.variables.get(name) {
                            if let Err(msg) = check_assignable(&var_info.ty, ret_ty) {
                                ctx.error(
                                    aethel_syntax::diagnostic::codes::TYPE_MISMATCH(),
                                    &format!("type mismatch in `return`: {}", msg),
                                    *span,
                                );
                            }
                        }
                    }
                    check_expr_for_epistemic_violations(ctx, ret_expr, declared_effects);
                } else if let Some(expr) = expr {
                    check_expr_for_epistemic_violations(ctx, expr, declared_effects);
                }
            }
            Stmt::Let { name, ty, init, is_mut, span } => {
                let inferred_ty = if let Some(init) = init {
                    let inferred = infer_expr_return_type(ctx, init, declared_effects);
                    // Check the init expression for epistemic violations
                    check_expr_for_epistemic_violations(ctx, init, declared_effects);
                    inferred
                } else {
                    None
                };
                let resolved_ty = inferred_ty.or_else(|| ty.as_ref().map(|t| ir_type_from_ast_type(t)))
                    .unwrap_or(IrType::Unit { span: *span });
                ctx.type_env.add_variable(name.name.clone(), VariableInfo {
                    ty: resolved_ty,
                    is_mut: *is_mut,
                    is_linear: false,
                });
            }
            Stmt::If { cond, then_branch, else_branch, .. } => {
                check_expr_for_epistemic_violations(ctx, cond, declared_effects);
                check_block_for_epistemic_violations(ctx, then_branch, declared_effects);
                if let Some(else_stmt) = else_branch {
                    check_stmt_for_epistemic_violations(ctx, else_stmt, declared_effects);
                }
            }
            Stmt::While { cond, body, .. } => {
                check_expr_for_epistemic_violations(ctx, cond, declared_effects);
                check_block_for_epistemic_violations(ctx, body, declared_effects);
            }
            Stmt::Block { block, .. } => {
                check_block_for_epistemic_violations(ctx, block, declared_effects);
            }
            _ => {}
        }
    }

    // Check tail expression
    if let Some(tail) = &block.tail {
        check_expr_for_epistemic_violations(ctx, tail, declared_effects);
    }
    ctx.type_env.exit_scope();
}

fn check_stmt_for_epistemic_violations(
    ctx: &mut CheckContext,
    stmt: &aethel_syntax::ast::Stmt,
    declared_effects: &[aethel_syntax::ast::EffectRef],
) {
    use aethel_syntax::ast::Stmt;
    match stmt {
        Stmt::Expr { expr, .. } => {
            check_expr_for_epistemic_violations(ctx, expr, declared_effects);
        }
        Stmt::Block { block, .. } => {
            check_block_for_epistemic_violations(ctx, block, declared_effects);
        }
        _ => {}
    }
}

fn check_expr_for_epistemic_violations(
    ctx: &mut CheckContext,
    expr: &aethel_syntax::ast::Expr,
    declared_effects: &[aethel_syntax::ast::EffectRef],
) {
    use aethel_syntax::ast::Expr;

    match expr {
        Expr::MethodCall { receiver, method, args, span } => {
            // Check if this is a method call on an effect
            // First check if the receiver name matches a declared effect
            // OR if any registered effect has this operation
            let receiver_name = if let Expr::Path { path, .. } = receiver.as_ref() {
                path.segments.first().map(|s| s.name.name.as_str())
            } else {
                None
            };

            // Check against ALL registered effects (not just declared)
            // This handles both `PaymentGateway.refund()` and `payments.refund()` patterns
            let found_effect_op = {
                let effect_registry = &ctx.effect_registry;
                receiver_name.and_then(|rname| {
                    if let Some(ef) = effect_registry.get(rname) {
                        return ef.operations.iter().find(|o| o.name == method.name);
                    }
                    for (_, ef) in &effect_registry.effects {
                        if let Some(op) = ef.operations.iter().find(|o| o.name == method.name) {
                            return Some(op);
                        }
                    }
                    None
                })
            };

            if let Some(op) = found_effect_op {
                let op_params = op.params.clone();
                let op_name = op.name.clone();
                let effect_name = receiver_name.map(|s| s.to_string()).unwrap_or_default();
                let expected_args = op_params.len();
                drop(op);

                // TYPE CHECK: argument count matches parameter count (before borrow issues)
                if args.len() != expected_args {
                    ctx.error(
                        aethel_syntax::diagnostic::codes::TYPE_MISMATCH(),
                        &format!(
                            "effect `{}.{}` expects {} argument(s), got {}",
                            effect_name, method.name, expected_args, args.len()
                        ),
                        *span,
                    );
                }

                // Check each argument against the operation's parameter types
                for (i, arg) in args.iter().enumerate() {
                    if i < op_params.len() {
                        let param_ty = &op_params[i];
                        // Check if param type requires Verified<T, Policy>
                        let needs_verified = matches!(param_ty.ty, aethel_ir::lower::IrType::Verified { .. });
                        if needs_verified {
                            if let Expr::Path { path: arg_path, .. } = arg {
                                if let Some(arg_name) = arg_path.segments.first().map(|s| s.name.name.as_str()) {
                                    if let Some(var_info) = ctx.type_env.get_variable(arg_name) {
                                        if matches!(var_info.ty, IrType::Claim { .. }) {
                                            ctx.error(
                                                aethel_syntax::diagnostic::codes::EPISTEMIC_CLAIM_NOT_VERIFIED(),
                                                &format!("unverified claim cannot authorize `{}.{}`", receiver_name.unwrap_or("?"), method.name),
                                                *span,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Recurse into sub-expressions
            check_expr_for_epistemic_violations(ctx, receiver, declared_effects);
            for arg in args {
                check_expr_for_epistemic_violations(ctx, arg, declared_effects);
            }
        }
        Expr::Block { block, .. } => {
            check_block_for_epistemic_violations(ctx, block, declared_effects);
        }
        Expr::Call { callee, args, .. } => {
            check_expr_for_epistemic_violations(ctx, callee, declared_effects);
            for arg in args {
                check_expr_for_epistemic_violations(ctx, arg, declared_effects);
            }
        }
        Expr::If { cond, then_branch, else_branch, .. } => {
            check_expr_for_epistemic_violations(ctx, cond, declared_effects);
            check_expr_for_epistemic_violations(ctx, then_branch, declared_effects);
            if let Some(else_branch) = else_branch {
                check_expr_for_epistemic_violations(ctx, else_branch, declared_effects);
            }
        }
        Expr::Let { init, .. } => {
            check_expr_for_epistemic_violations(ctx, init, declared_effects);
        }
        Expr::Return { expr, .. } => {
            if let Some(expr) = expr {
                check_expr_for_epistemic_violations(ctx, expr, declared_effects);
            }
        }
        // verify(claim, Policy) - check claim is Claim<T> and policy exists
        Expr::Verify { claim, policy, span } => {
            let policy_name = policy.segments.first().map(|s| s.name.name.as_str()).unwrap_or("");
            if !policy_name.is_empty() && !ctx.policy_registry.policies.contains_key(policy_name) {
                ctx.error(
                    aethel_syntax::diagnostic::codes::UNDEFINED_TYPE(),
                    &format!("policy `{policy_name}` is not defined"),
                    *span,
                );
            }
            // verify() first argument must be a Claim variable
            match claim.as_ref() {
                Expr::Path { path, .. } => {
                    let name = path.segments.last().map(|s| s.name.name.as_str()).unwrap_or("");
                    if let Some(var_info) = ctx.type_env.get_variable(name) {
                        if !matches!(var_info.ty, IrType::Claim { .. }) {
                            ctx.error(
                                aethel_syntax::diagnostic::codes::TYPE_MISMATCH(),
                                &format!("verify() expects Claim<T>, not a bare value"),
                                *span,
                            );
                        }
                    }
                }
                _ => {
                    ctx.error(
                        aethel_syntax::diagnostic::codes::TYPE_MISMATCH(),
                        &format!("verify() expects a Claim variable as argument"),
                        *span,
                    );
                }
            }
            check_expr_for_epistemic_violations(ctx, claim, declared_effects);
        }
        Expr::Tuple { exprs, .. } => {
            for e in exprs {
                check_expr_for_epistemic_violations(ctx, e, declared_effects);
            }
        }
        Expr::Binary { left, right, .. } => {
            check_expr_for_epistemic_violations(ctx, left, declared_effects);
            check_expr_for_epistemic_violations(ctx, right, declared_effects);
        }
        Expr::Unary { expr, .. } => {
            check_expr_for_epistemic_violations(ctx, expr, declared_effects);
        }
        _ => {}
    }
}

/// Infer the return type of an expression for type environment tracking.
/// This enables `let x = verify(claim, Policy)` to give x the Verified type.
fn infer_expr_return_type(
    ctx: &mut CheckContext,
    expr: &aethel_syntax::ast::Expr,
    declared_effects: &[aethel_syntax::ast::EffectRef],
) -> Option<IrType> {
    use aethel_syntax::ast::Expr;
    match expr {
        // verify(claim, Policy) - check that policy exists before inferring type
        Expr::Verify { claim, policy, span } => {
            // First verify the policy exists
            let policy_name = policy.segments.first()
                .map(|s| s.name.name.as_str()).unwrap_or("").to_string();
            if !policy_name.is_empty() && !ctx.policy_registry.policies.contains_key(&policy_name) {
                return None;
            }
            // Get the inner type T from Claim<T>
            if let Expr::Path { path: claim_path, .. } = claim.as_ref() {
                if let Some(claim_name) = claim_path.segments.first().map(|s| s.name.name.as_str()) {
                    if let Some(var_info) = ctx.type_env.get_variable(claim_name) {
                        if let IrType::Claim { ty: inner_ty, .. } = &var_info.ty {
                            // Build Verified<T, Policy>
                            let policy_path = policy.segments.first()
                                .map(|s| s.name.name.clone())
                                .unwrap_or_default();
                            let policy_ir = IrType::Path {
                                span: *span,
                                path: IrTypePath {
                                    span: *span,
                                    segments: vec![IrPathSegment {
                                        span: *span,
                                        name: policy_path,
                                        args: None,
                                    }],
                                },
                            };
                            return Some(IrType::Verified {
                                span: *span,
                                ty: inner_ty.clone(),
                                policy: Box::new(policy_ir),
                            });
                        }
                    }
                }
            }
            // Fallback: Verified<_, _>
            let default_policy = policy.segments.first()
                .map(|s| s.name.name.clone())
                .unwrap_or_default();
            Some(IrType::Verified {
                span: *span,
                ty: Box::new(IrType::Path {
                    span: *span,
                    path: IrTypePath {
                        span: *span,
                        segments: vec![IrPathSegment {
                            span: *span,
                            name: "unknown".to_string(),
                            args: None,
                        }],
                    },
                }),
                policy: Box::new(IrType::Path {
                    span: *span,
                    path: IrTypePath {
                        span: *span,
                        segments: vec![IrPathSegment {
                            span: *span,
                            name: default_policy,
                            args: None,
                        }],
                    },
                }),
            })
        }
        // Method calls on effects: look up return type from effect registry
        Expr::MethodCall { receiver, method, .. } => {
            let receiver_name = if let Expr::Path { path, .. } = receiver.as_ref() {
                path.segments.first().map(|s| s.name.name.as_str())
            } else {
                None
            };
            // Try to find the effect by receiver name; if not found, search all effects by operation name
            let result = receiver_name.and_then(|rname| {
                ctx.effect_registry.get(rname).and_then(|ef| {
                    ef.operations.iter().find(|o| o.name == method.name)
                        .and_then(|op| op.ret_type.clone())
                })
            });
            // Fallback: search all registered effects by operation name
            result.or_else(|| {
                for (_, ef) in &ctx.effect_registry.effects {
                    if let Some(op) = ef.operations.iter().find(|o| o.name == method.name) {
                        return op.ret_type.clone();
                    }
                }
                None
            })
        }
        // Path expression: look up variable type from environment
        Expr::Path { path, .. } => {
            let name = path.segments.last().map(|s| s.name.name.as_str()).unwrap_or("");
            if let Some(var_info) = ctx.type_env.variables.get(name) {
                Some(var_info.ty.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn collect_type_defs(ctx: &mut CheckContext, item: &aethel_hir::lower::HirItem) {
    use aethel_hir::lower::HirItem;
    match item {
        HirItem::Struct(s) => {
            let mut fields = IndexMap::new();
            for field in &s.fields {
                fields.insert(field.name.clone(), crate::types::lower_hir_type(&field.ty));
            }
            ctx.type_env.type_defs.insert(s.name.clone(), TypeDefinition {
                kind: TypeDefKind::Struct { fields },
                generics: s.generics.iter().map(|g| g.name.clone()).collect(),
            });
        }
        HirItem::Enum(e) => {
            let mut variants = IndexMap::new();
            for variant in &e.variants {
                let types: Vec<_> = variant.fields.iter()
                    .map(|f| match f {
                        aethel_hir::lower::HirEnumField::Tuple { ty, .. } => crate::types::lower_hir_type(&ty),
                        aethel_hir::lower::HirEnumField::Named { ty, .. } => crate::types::lower_hir_type(&ty),
                    })
                    .collect();
                variants.insert(variant.name.clone(), types);
            }
            ctx.type_env.type_defs.insert(e.name.clone(), TypeDefinition {
                kind: TypeDefKind::Enum { variants },
                generics: e.generics.iter().map(|g| g.name.clone()).collect(),
            });
        }
        HirItem::TypeAlias(t) => {
            ctx.type_env.type_defs.insert(t.name.clone(), TypeDefinition {
                kind: TypeDefKind::TypeAlias { ty: crate::types::lower_hir_type(&t.ty) },
                generics: t.generics.iter().map(|g| g.name.clone()).collect(),
            });
        }
        HirItem::Policy(p) => {
            let mut claims = IndexMap::new();
            for claim in &p.claims {
                let evidence: Vec<_> = claim.evidence.iter().map(|e| match &e.kind {
                    aethel_hir::lower::HirEvidenceKind::SignedAttestation => EvidenceKind::SignedAttestation,
                    aethel_hir::lower::HirEvidenceKind::CryptographicProof => EvidenceKind::CryptographicProof,
                    aethel_hir::lower::HirEvidenceKind::AuditLog => EvidenceKind::AuditLog,
                    aethel_hir::lower::HirEvidenceKind::HumanReview => EvidenceKind::HumanReview,
                    aethel_hir::lower::HirEvidenceKind::Custom(s) => EvidenceKind::Custom(s.clone()),
                }).collect();
                claims.insert(claim.name.clone(), PolicyClaim {
                    ty: crate::types::lower_hir_type(&claim.ty),
                    evidence,
                });
            }
            ctx.type_env.type_defs.insert(p.name.clone(), TypeDefinition {
                kind: TypeDefKind::Policy { claims },
                generics: p.generics.iter().map(|g| g.name.clone()).collect(),
            });
        }
        _ => {}
    }
}

fn collect_policies(ctx: &mut CheckContext, item: &aethel_hir::lower::HirItem) {
    // Policies are collected in collect_type_defs
}

fn check_item(ctx: &mut CheckContext, item: &aethel_hir::lower::HirItem) -> Option<aethel_ir::lower::IrItem> {
    use aethel_hir::lower::HirItem;
    use aethel_ir::lower::IrItem;

    match item {
        HirItem::Fn(f) => check_fn(ctx, f).map(IrItem::Fn),
        HirItem::Struct(s) => Some(IrItem::Struct(lower_struct(s))),
        HirItem::Enum(e) => Some(IrItem::Enum(lower_enum(e))),
        HirItem::TypeAlias(t) => Some(IrItem::TypeAlias(lower_type_alias(t))),
        HirItem::Use(u) => Some(IrItem::Use(lower_use(u))),
        HirItem::Mod(m) => Some(IrItem::Mod(lower_mod(m))),
        HirItem::Policy(p) => Some(IrItem::Policy(lower_policy(p))),
        HirItem::Effect(_) => None, // effects are for boundary, registered separately
    }
}

fn check_fn(ctx: &mut CheckContext, f: &aethel_hir::lower::HirFnDef) -> Option<aethel_ir::lower::IrFnDef> {
    // Check function body if present
    let body = if let Some(body) = &f.body {
        Some(check_block(ctx, body)?)
    } else {
        None
    };

    Some(aethel_ir::lower::IrFnDef {
        span: f.span,
        name: f.name.clone(),
        generics: f.generics.iter().map(|g| aethel_ir::lower::IrGenericParam {
            span: g.span,
            name: g.name.clone(),
            bounds: g.bounds.iter().map(|b| aethel_ir::lower::IrTypeBound {
                span: b.span,
                path: lower_type_path(&b.path),
            }).collect(),
        }).collect(),
        params: f.params.iter().map(|p| aethel_ir::lower::IrParam {
            span: p.span,
            name: p.name.clone(),
            ty: crate::types::lower_hir_type(&p.ty),
            is_mut: p.is_mut,
        }).collect(),
        ret_type: f.ret_type.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or(aethel_ir::lower::IrType::Unit { span: f.span }),
        effects: lower_effect_set(&f.effects),
        body,
        is_pub: f.is_pub,
    })
}

fn check_block(ctx: &mut CheckContext, block: &aethel_hir::lower::HirBlock) -> Option<aethel_ir::lower::IrBlock> {
    ctx.type_env.enter_scope();
    let mut stmts = Vec::new();
    for stmt in &block.stmts {
        stmts.push(check_stmt(ctx, stmt)?);
    }
    let tail = block.tail.as_ref().and_then(|e| check_expr(ctx, e)).map(Box::new);
    ctx.type_env.exit_scope();

    Some(aethel_ir::lower::IrBlock {
        span: block.span,
        stmts,
        tail,
    })
}

fn check_stmt(ctx: &mut CheckContext, stmt: &aethel_hir::lower::HirStmt) -> Option<aethel_ir::lower::IrStmt> {
    use aethel_hir::lower::HirStmt;
    use aethel_ir::lower::IrStmt;

    match stmt {
        HirStmt::Let { span, name, ty, is_mut, init } => {
            let init_ir = init.as_ref().and_then(|e| check_expr(ctx, e));
            let ty_ir = ty.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or_else(|| {
                // Infer from init
                aethel_ir::lower::IrType::Unit { span: *span }
            });
            ctx.type_env.variables.insert(name.clone(), VariableInfo {
                ty: ty_ir.clone(),
                is_mut: *is_mut,
                is_linear: false,
            });
            Some(IrStmt::Let { span: *span, name: name.clone(), ty: ty_ir, is_mut: *is_mut, init: init_ir })
        }
        HirStmt::Expr { span, expr } => {
            Some(IrStmt::Expr { span: *span, expr: check_expr(ctx, expr)? })
        }
        HirStmt::Return { span, expr } => {
            Some(IrStmt::Return { span: *span, expr: expr.as_ref().and_then(|e| check_expr(ctx, e)) })
        }
        HirStmt::If { span, cond, then_branch, else_branch } => {
            Some(IrStmt::If {
                span: *span,
                cond: check_expr(ctx, cond)?,
                then_branch: check_block(ctx, then_branch)?,
                else_branch: else_branch.as_ref().and_then(|b| check_stmt(ctx, b)).map(Box::new),
            })
        }
        HirStmt::While { span, cond, body } => {
            Some(IrStmt::While {
                span: *span,
                cond: check_expr(ctx, cond)?,
                body: check_block(ctx, body)?,
            })
        }
        HirStmt::For { span, pat, iter, body } => {
            Some(IrStmt::For {
                span: *span,
                pat: check_pat(ctx, pat)?,
                iter: check_expr(ctx, iter)?,
                body: check_block(ctx, body)?,
            })
        }
        HirStmt::Match { span, scrutinee, arms } => {
            Some(IrStmt::Match {
                span: *span,
                scrutinee: check_expr(ctx, scrutinee)?,
                arms: arms.iter().map(|a| check_match_arm(ctx, a)).collect::<Option<Vec<_>>>()?,
            })
        }
        HirStmt::Block { span, block } => {
            Some(IrStmt::Block { span: *span, block: check_block(ctx, block)? })
        }
    }
}

fn check_expr(ctx: &mut CheckContext, expr: &aethel_hir::lower::HirExpr) -> Option<aethel_ir::lower::IrExpr> {
    use aethel_hir::lower::HirExpr;
    use aethel_ir::lower::IrExpr;

    let span = expr.span();

    match expr {
        HirExpr::Literal { lit, .. } => {
            Some(IrExpr::Literal { span, lit: lower_literal(lit) })
        }
        HirExpr::Path { path, .. } => {
            let ir_path = IrExpr::Path { span, path: lower_expr_path(path) };
            Some(ir_path)
        }
        HirExpr::Tuple { exprs, .. } => {
            Some(IrExpr::Tuple { span, exprs: exprs.iter().map(|e| check_expr(ctx, e)).collect::<Option<Vec<_>>>()? })
        }
        HirExpr::Array { exprs, .. } => {
            Some(IrExpr::Array { span, exprs: exprs.iter().map(|e| check_expr(ctx, e)).collect::<Option<Vec<_>>>()? })
        }
        HirExpr::Struct { path, fields, base, .. } => {
            Some(IrExpr::Struct {
                span,
                path: lower_type_path(path),
                fields: fields.iter().map(|f| {
                    let expr = check_expr(ctx, &f.expr)?;
                    Some(aethel_ir::lower::IrStructExprField {
                        span: f.span,
                        name: f.name.clone(),
                        expr,
                    })
                }).collect::<Option<Vec<_>>>()?,
                base: base.as_ref().and_then(|b| check_expr(ctx, b)).map(Box::new),
            })
        }
        HirExpr::Call { callee, args, .. } => {
            Some(IrExpr::Call {
                span,
                callee: Box::new(check_expr(ctx, callee)?),
                args: args.iter().map(|a| check_expr(ctx, a)).collect::<Option<Vec<_>>>()?,
            })
        }
        HirExpr::MethodCall { receiver, method, args, .. } => {
            Some(IrExpr::MethodCall {
                span,
                receiver: Box::new(check_expr(ctx, receiver)?),
                method: method.clone(),
                args: args.iter().map(|a| check_expr(ctx, a)).collect::<Option<Vec<_>>>()?,
            })
        }
        HirExpr::Field { base, field, .. } => {
            Some(IrExpr::Field {
                span,
                base: Box::new(check_expr(ctx, base)?),
                field: field.clone(),
            })
        }
        HirExpr::Index { base, index, .. } => {
            Some(IrExpr::Index {
                span,
                base: Box::new(check_expr(ctx, base)?),
                index: Box::new(check_expr(ctx, index)?),
            })
        }
        HirExpr::Unary { op, expr, .. } => {
            Some(IrExpr::Unary {
                span,
                op: lower_unary_op(op),
                expr: Box::new(check_expr(ctx, expr)?),
            })
        }
        HirExpr::Binary { op, left, right, .. } => {
            Some(IrExpr::Binary {
                span,
                op: lower_binary_op(op),
                left: Box::new(check_expr(ctx, left)?),
                right: Box::new(check_expr(ctx, right)?),
            })
        }
        HirExpr::If { cond, then_branch, else_branch, .. } => {
            Some(IrExpr::If {
                span,
                cond: Box::new(check_expr(ctx, cond)?),
                then_branch: Box::new(check_expr(ctx, then_branch)?),
                else_branch: else_branch.as_ref().and_then(|e| check_expr(ctx, e)).map(Box::new),
            })
        }
        HirExpr::Match { scrutinee, arms, .. } => {
            Some(IrExpr::Match {
                span,
                scrutinee: Box::new(check_expr(ctx, scrutinee)?),
                arms: arms.iter().map(|a| check_match_arm(ctx, a)).collect::<Option<Vec<_>>>()?,
            })
        }
        HirExpr::Block { block, .. } => {
            Some(IrExpr::Block { span, block: check_block(ctx, block)? })
        }
        HirExpr::Let { pat, ty, is_mut, init, .. } => {
            Some(IrExpr::Let {
                span,
                pat: check_pat(ctx, pat)?,
                ty: ty.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or(aethel_ir::lower::IrType::Unit { span }),
                is_mut: *is_mut,
                init: Box::new(check_expr(ctx, init)?),
            })
        }
        HirExpr::Return { expr, .. } => {
            Some(IrExpr::Return {
                span,
                expr: expr.as_ref().and_then(|e| check_expr(ctx, e)).map(Box::new),
            })
        }
        HirExpr::Break { expr, .. } => {
            Some(IrExpr::Break {
                span,
                expr: expr.as_ref().and_then(|e| check_expr(ctx, e)).map(Box::new),
            })
        }
        HirExpr::Continue { .. } => {
            Some(IrExpr::Continue { span })
        }
        HirExpr::Ask { model, goal, input, output_ty, .. } => {
            Some(IrExpr::Ask {
                span,
                model: lower_expr_path(model),
                goal: goal.clone(),
                input: Box::new(check_expr(ctx, input)?),
                output_ty: crate::types::lower_hir_type(&output_ty),
            })
        }
        HirExpr::Verify { span, claim, policy } => {
                    // EPISTEMIC TYPE RULE: Claim<T> -> Verified<T, Policy>
                    // This is where AE-EPISTEMIC-001 is enforced
                    let claim_expr = check_expr(ctx, claim)?;
                    let claim_ty = claim_expr.ty();
            
                    // Check if claim is Claim<T>
                    if let aethel_ir::lower::IrType::Claim { ty: inner, .. } = &claim_ty {
                        // This is a Claim<T> - need to verify it produces Verified<T, Policy>
                        // The verify expression itself should produce Verified<T, Policy>
                        Some(IrExpr::Verify {
                            span: *span,
                            claim: Box::new(claim_expr),
                            policy: lower_type_path(policy),
                        })
                    } else {
                        // Not a Claim - error
                        ctx.error(
                            aethel_syntax::diagnostic::codes::EPISTEMIC_CLAIM_NOT_VERIFIED(),
                            "expected `Claim<T>` as argument to `verify`",
                            claim_expr.span(),
                        );
                        Some(IrExpr::Verify {
                            span: *span,
                            claim: Box::new(claim_expr),
                            policy: lower_type_path(policy),
                        })
                    }
                }
                HirExpr::Reason { span, prompt } => {
                    // AI primitive that generates a Claim<T> - always produces Claim<String> or Claim<T>
                    // The actual type depends on the context, but it's fundamentally an untrusted claim
                    Some(IrExpr::Reason {
                        span: *span,
                        prompt: prompt.clone(),
                    })
                }
                HirExpr::CommitOnce { effect, args, .. } => {
                            // EPISTEMIC TYPE RULE: Effects require Verified<T, Policy> arguments, not raw Claim<T>
                            let ir_args: Option<Vec<_>> = args.iter().map(|a| check_expr(ctx, a)).collect();
                            let ir_args = ir_args?;

                            // Verify that all arguments to effects are Verified<T, Policy>, not raw Claim<T>
                            for arg in &ir_args {
                                let arg_ty = arg.ty();
                                if matches!(arg_ty, aethel_ir::lower::IrType::Claim { .. }) {
                                    ctx.error(
                                        aethel_syntax::diagnostic::codes::EPISTEMIC_UNVERIFIED_EFFECT(),
                                        "Cannot pass an unverified `Claim<T>` to an Effect. It must be verified first.",
                                        span,
                                    );
                                    return None;
                                }
                            }

                            Some(IrExpr::CommitOnce {
                                span,
                                effect: lower_effect_ref(effect),
                                args: ir_args,
                            })
                        }
        HirExpr::New { ty, args, .. } => {
            Some(IrExpr::New {
                span,
                ty: crate::types::lower_hir_type(&ty),
                args: args.iter().map(|a| check_expr(ctx, a)).collect::<Option<Vec<_>>>()?,
            })
        }
    }
}

fn check_match_arm(ctx: &mut CheckContext, arm: &aethel_hir::lower::HirMatchArm) -> Option<aethel_ir::lower::IrMatchArm> {
    Some(aethel_ir::lower::IrMatchArm {
        span: arm.span,
        pat: check_pat(ctx, &arm.pat)?,
        guard: arm.guard.as_ref().and_then(|g| check_expr(ctx, g)),
        body: check_expr(ctx, &arm.body)?,
    })
}

fn check_pat(ctx: &mut CheckContext, pat: &aethel_hir::lower::HirPat) -> Option<aethel_ir::lower::IrPat> {
    use aethel_hir::lower::HirPat;
    use aethel_ir::lower::IrPat;

    match pat {
        HirPat::Wild { span } => Some(IrPat::Wild { span: *span }),
        HirPat::Ident { span, name, is_mut } => {
            ctx.type_env.variables.insert(name.clone(), VariableInfo {
                ty: aethel_ir::lower::IrType::Unit { span: *span },
                is_mut: *is_mut,
                is_linear: false,
            });
            Some(IrPat::Ident { span: *span, name: name.clone(), is_mut: *is_mut })
        }
        HirPat::Literal { span, lit } => Some(IrPat::Literal { span: *span, lit: lower_literal(lit) }),
        HirPat::Tuple { span, pats } => Some(IrPat::Tuple { span: *span, pats: pats.iter().map(|p| check_pat(ctx, p)).collect::<Option<Vec<_>>>()? }),
        HirPat::Struct { span, path, fields } => Some(IrPat::Struct {
            span: *span,
            path: lower_type_path(path),
            fields: fields.iter().map(|f| aethel_ir::lower::IrPatField {
                span: f.span,
                name: f.name.clone(),
                pat: f.pat.as_ref().and_then(|p| check_pat(ctx, p)),
            }).collect(),
        }),
        HirPat::Enum { span, path, fields } => Some(IrPat::Enum {
            span: *span,
            path: lower_type_path(path),
            fields: fields.iter().map(|f| check_pat(ctx, f)).collect::<Option<Vec<_>>>()?,
        }),
        HirPat::Or { span, pats } => Some(IrPat::Or { span: *span, pats: pats.iter().map(|p| check_pat(ctx, p)).collect::<Option<Vec<_>>>()? }),
        HirPat::Ref { span, is_mut, pat } => Some(IrPat::Ref { span: *span, is_mut: *is_mut, pat: Box::new(check_pat(ctx, pat)?) }),
    }
}

// Lowering helpers
fn lower_struct(s: &aethel_hir::lower::HirStructDef) -> aethel_ir::lower::IrStructDef {
    aethel_ir::lower::IrStructDef {
        span: s.span,
        name: s.name.clone(),
        generics: s.generics.iter().map(|g| aethel_ir::lower::IrGenericParam {
            span: g.span,
            name: g.name.clone(),
            bounds: g.bounds.iter().map(|b| aethel_ir::lower::IrTypeBound {
                span: b.span,
                path: lower_type_path(&b.path),
            }).collect(),
        }).collect(),
        fields: s.fields.iter().map(|f| aethel_ir::lower::IrStructField {
            span: f.span,
            name: f.name.clone(),
            ty: crate::types::lower_hir_type(&f.ty),
            is_pub: f.is_pub,
        }).collect(),
        is_pub: s.is_pub,
    }
}

fn lower_enum(e: &aethel_hir::lower::HirEnumDef) -> aethel_ir::lower::IrEnumDef {
    aethel_ir::lower::IrEnumDef {
        span: e.span,
        name: e.name.clone(),
        generics: e.generics.iter().map(|g| aethel_ir::lower::IrGenericParam {
            span: g.span,
            name: g.name.clone(),
            bounds: g.bounds.iter().map(|b| aethel_ir::lower::IrTypeBound {
                span: b.span,
                path: lower_type_path(&b.path),
            }).collect(),
        }).collect(),
        variants: e.variants.iter().map(|v| aethel_ir::lower::IrEnumVariant {
            span: v.span,
            name: v.name.clone(),
            fields: v.fields.iter().map(|f| match f {
                aethel_hir::lower::HirEnumField::Tuple { span, ty } => aethel_ir::lower::IrEnumField::Tuple { span: *span, ty: crate::types::lower_hir_type(&ty) },
                aethel_hir::lower::HirEnumField::Named { span, name, ty } => aethel_ir::lower::IrEnumField::Named { span: *span, name: name.clone(), ty: crate::types::lower_hir_type(&ty) },
            }).collect(),
        }).collect(),
        is_pub: e.is_pub,
    }
}

fn lower_type_alias(t: &aethel_hir::lower::HirTypeAlias) -> aethel_ir::lower::IrTypeAlias {
    aethel_ir::lower::IrTypeAlias {
        span: t.span,
        name: t.name.clone(),
        generics: t.generics.iter().map(|g| aethel_ir::lower::IrGenericParam {
            span: g.span,
            name: g.name.clone(),
            bounds: g.bounds.iter().map(|b| aethel_ir::lower::IrTypeBound {
                span: b.span,
                path: lower_type_path(&b.path),
            }).collect(),
        }).collect(),
        ty: crate::types::lower_hir_type(&t.ty),
        is_pub: t.is_pub,
    }
}

fn lower_use(u: &aethel_hir::lower::HirUseDecl) -> aethel_ir::lower::IrUseDecl {
    aethel_ir::lower::IrUseDecl {
        span: u.span,
        path: lower_use_path(&u.path),
        is_pub: u.is_pub,
    }
}

fn lower_use_path(p: &aethel_hir::lower::HirUsePath) -> aethel_ir::lower::IrUsePath {
    use aethel_hir::lower::HirUsePath;
    use aethel_ir::lower::IrUsePath;
    match p {
        HirUsePath::Simple { span, path } => IrUsePath::Simple { span: *span, path: lower_type_path(path) },
        HirUsePath::Glob { span, prefix } => IrUsePath::Glob { span: *span, prefix: lower_type_path(prefix) },
        HirUsePath::Group { span, prefix, items } => IrUsePath::Group { span: *span, prefix: lower_type_path(prefix), items: items.iter().map(lower_use_path).collect() },
    }
}

fn lower_mod(m: &aethel_hir::lower::HirModDecl) -> aethel_ir::lower::IrModDecl {
    aethel_ir::lower::IrModDecl {
        span: m.span,
        name: m.name.clone(),
        body: m.body.as_ref().map(|_b| {
            aethel_ir::lower::IrModule {
                file_id: m.span.file,
                items: vec![],
            }
        }),
        is_pub: m.is_pub,
    }
}

fn lower_policy(p: &aethel_hir::lower::HirPolicyDef) -> aethel_ir::lower::IrPolicyDef {
    aethel_ir::lower::IrPolicyDef {
        span: p.span,
        name: p.name.clone(),
        generics: p.generics.iter().map(|g| aethel_ir::lower::IrGenericParam {
            span: g.span,
            name: g.name.clone(),
            bounds: g.bounds.iter().map(|b| aethel_ir::lower::IrTypeBound {
                span: b.span,
                path: lower_type_path(&b.path),
            }).collect(),
        }).collect(),
        claims: p.claims.iter().map(|c| aethel_ir::lower::IrPolicyClaim {
            span: c.span,
            name: c.name.clone(),
            ty: crate::types::lower_hir_type(&c.ty),
            evidence: c.evidence.iter().map(|e| aethel_ir::lower::IrEvidenceReq {
                span: e.span,
                kind: match &e.kind {
                    aethel_hir::lower::HirEvidenceKind::SignedAttestation => aethel_ir::lower::IrEvidenceKind::SignedAttestation,
                    aethel_hir::lower::HirEvidenceKind::CryptographicProof => aethel_ir::lower::IrEvidenceKind::CryptographicProof,
                    aethel_hir::lower::HirEvidenceKind::AuditLog => aethel_ir::lower::IrEvidenceKind::AuditLog,
                    aethel_hir::lower::HirEvidenceKind::HumanReview => aethel_ir::lower::IrEvidenceKind::HumanReview,
                    aethel_hir::lower::HirEvidenceKind::Custom(s) => aethel_ir::lower::IrEvidenceKind::Custom(s.clone()),
                },
                description: e.description.clone(),
            }).collect(),
        }).collect(),
        is_pub: p.is_pub,
    }
}

fn lower_type_path(p: &aethel_hir::lower::HirTypePath) -> aethel_ir::lower::IrTypePath {
    aethel_ir::lower::IrTypePath {
        span: p.span,
        segments: p.segments.iter().map(|s| aethel_ir::lower::IrPathSegment {
            span: s.span,
            name: s.name.clone(),
            args: s.args.as_ref().map(|a| aethel_ir::lower::IrGenericArgs {
                span: a.span,
                args: a.args.iter().map(|arg| match arg {
                    aethel_hir::lower::HirGenericArg::Type { span, ty } => aethel_ir::lower::IrGenericArg::Type { span: *span, ty: crate::types::lower_hir_type(&ty) },
                    aethel_hir::lower::HirGenericArg::Const { span, expr } => aethel_ir::lower::IrGenericArg::Const { span: *span, expr: crate::types::lower_hir_expr(expr) },
                }).collect(),
            }),
        }).collect(),
    }
}

fn lower_expr_path(p: &aethel_hir::lower::HirExprPath) -> aethel_ir::lower::IrExprPath {
    aethel_ir::lower::IrExprPath {
        span: p.span,
        segments: p.segments.iter().map(|s| aethel_ir::lower::IrPathSegment {
            span: s.span,
            name: s.name.clone(),
            args: s.args.as_ref().map(|a| aethel_ir::lower::IrGenericArgs {
                span: a.span,
                args: a.args.iter().map(|arg| match arg {
                    aethel_hir::lower::HirGenericArg::Type { span, ty } => aethel_ir::lower::IrGenericArg::Type { span: *span, ty: crate::types::lower_hir_type(&ty) },
                    aethel_hir::lower::HirGenericArg::Const { span, expr } => aethel_ir::lower::IrGenericArg::Const { span: *span, expr: crate::types::lower_hir_expr(expr) },
                }).collect(),
            }),
        }).collect(),
    }
}

fn lower_effect_set(e: &aethel_hir::lower::HirEffectSet) -> aethel_ir::lower::IrEffectSet {
    aethel_ir::lower::IrEffectSet {
        span: e.span,
        effects: e.effects.iter().map(|eff| aethel_ir::lower::IrEffectRef {
            span: eff.span,
            path: lower_type_path(&eff.path),
        }).collect(),
    }
}

fn lower_effect_ref(e: &aethel_hir::lower::HirEffectRef) -> aethel_ir::lower::IrEffectRef {
    aethel_ir::lower::IrEffectRef {
        span: e.span,
        path: lower_type_path(&e.path),
    }
}

fn lower_literal(l: &aethel_hir::lower::HirLiteral) -> aethel_ir::lower::IrLiteral {
    use aethel_hir::lower::HirLiteral;
    use aethel_ir::lower::IrLiteral;
    match l {
        HirLiteral::Unit { span } => IrLiteral::Unit { span: *span },
        HirLiteral::Bool { span, value } => IrLiteral::Bool { span: *span, value: *value },
        HirLiteral::Int { span, value } => IrLiteral::Int { span: *span, value: *value },
        HirLiteral::Float { span, value } => IrLiteral::Float { span: *span, value: *value },
        HirLiteral::String { span, value } => IrLiteral::String { span: *span, value: value.clone() },
    }
}

fn lower_unary_op(op: &aethel_hir::lower::HirUnaryOp) -> aethel_ir::lower::IrUnaryOp {
    use aethel_hir::lower::HirUnaryOp;
    use aethel_ir::lower::IrUnaryOp;
    match op {
        HirUnaryOp::Neg => IrUnaryOp::Neg,
        HirUnaryOp::Not => IrUnaryOp::Not,
        HirUnaryOp::Deref => IrUnaryOp::Deref,
    }
}

fn lower_binary_op(op: &aethel_hir::lower::HirBinaryOp) -> aethel_ir::lower::IrBinaryOp {
    use aethel_hir::lower::HirBinaryOp;
    use aethel_ir::lower::IrBinaryOp;
    match op {
        HirBinaryOp::Add => IrBinaryOp::Add,
        HirBinaryOp::Sub => IrBinaryOp::Sub,
        HirBinaryOp::Mul => IrBinaryOp::Mul,
        HirBinaryOp::Div => IrBinaryOp::Div,
        HirBinaryOp::Rem => IrBinaryOp::Rem,
        HirBinaryOp::Eq => IrBinaryOp::Eq,
        HirBinaryOp::Ne => IrBinaryOp::Ne,
        HirBinaryOp::Lt => IrBinaryOp::Lt,
        HirBinaryOp::Le => IrBinaryOp::Le,
        HirBinaryOp::Gt => IrBinaryOp::Gt,
        HirBinaryOp::Ge => IrBinaryOp::Ge,
        HirBinaryOp::And => IrBinaryOp::And,
        HirBinaryOp::Or => IrBinaryOp::Or,
        HirBinaryOp::Assign => IrBinaryOp::Assign,
        HirBinaryOp::AddAssign => IrBinaryOp::AddAssign,
        HirBinaryOp::SubAssign => IrBinaryOp::SubAssign,
        HirBinaryOp::MulAssign => IrBinaryOp::MulAssign,
        HirBinaryOp::DivAssign => IrBinaryOp::DivAssign,
        HirBinaryOp::RemAssign => IrBinaryOp::RemAssign,
    }
}

// End of lower functions
trait IrExprExt {
    fn ty(&self) -> aethel_ir::lower::IrType;
}

impl IrExprExt for aethel_ir::lower::IrExpr {
    fn ty(&self) -> aethel_ir::lower::IrType {
        use aethel_ir::lower::IrExpr;
        match self {
            IrExpr::Literal { lit, .. } => match lit {
                aethel_ir::lower::IrLiteral::Unit { .. } => aethel_ir::lower::IrType::Unit { span: self.span() },
                aethel_ir::lower::IrLiteral::Bool { .. } => aethel_ir::lower::IrType::Bool { span: self.span() },
                aethel_ir::lower::IrLiteral::Int { .. } => aethel_ir::lower::IrType::Int { span: self.span() },
                aethel_ir::lower::IrLiteral::Float { .. } => aethel_ir::lower::IrType::Float { span: self.span() },
                aethel_ir::lower::IrLiteral::String { .. } => aethel_ir::lower::IrType::String { span: self.span() },
            }
            IrExpr::Path { .. } => aethel_ir::lower::IrType::Path { span: self.span(), path: aethel_ir::lower::IrTypePath { span: self.span(), segments: vec![] } },
            IrExpr::Verify { policy, .. } => aethel_ir::lower::IrType::Verified {
                span: self.span(),
                ty: Box::new(aethel_ir::lower::IrType::Unit { span: self.span() }),
                policy: Box::new(aethel_ir::lower::IrType::Path { span: self.span(), path: policy.clone() }),
            },
            _ => aethel_ir::lower::IrType::Unit { span: self.span() },
        }
    }
}

trait IrExprSpan {
    fn span(&self) -> Span;
}

impl IrExprSpan for aethel_ir::lower::IrExpr {
    fn span(&self) -> Span {
        use aethel_ir::lower::IrExpr;
        match self {
            IrExpr::Literal { span, .. } => *span,
            IrExpr::Path { span, .. } => *span,
            IrExpr::Tuple { span, .. } => *span,
            IrExpr::Array { span, .. } => *span,
            IrExpr::Struct { span, .. } => *span,
            IrExpr::Call { span, .. } => *span,
            IrExpr::MethodCall { span, .. } => *span,
            IrExpr::Field { span, .. } => *span,
            IrExpr::Index { span, .. } => *span,
            IrExpr::Unary { span, .. } => *span,
            IrExpr::Binary { span, .. } => *span,
            IrExpr::If { span, .. } => *span,
            IrExpr::Match { span, .. } => *span,
            IrExpr::Block { span, .. } => *span,
            IrExpr::Let { span, .. } => *span,
            IrExpr::Return { span, .. } => *span,
            IrExpr::Break { span, .. } => *span,
            IrExpr::Continue { span } => *span,
            IrExpr::Ask { span, .. } => *span,
            IrExpr::Verify { span, .. } => *span,
            IrExpr::Reason { span, .. } => *span,
            IrExpr::CommitOnce { span, .. } => *span,
            IrExpr::New { span, .. } => *span,
        }
    }
}

// ── AST-to-IR lowering (for interpreter) ──────────────

/// Collect IR items from an AST module.
fn collect_ir_items(module: &aethel_syntax::ast::Module) -> Vec<IrItem> {
    use aethel_syntax::ast::Item;
    let mut items = Vec::new();
    for item in &module.items {
        match item {
            Item::Fn(f) => items.push(IrItem::Fn(lower_fn(f))),
            Item::Struct(s) => items.push(IrItem::Struct(IrStructDef {
                span: s.span, name: s.name.name.clone(), generics: vec![], fields: vec![], is_pub: false,
            })),
            Item::Enum(_) => {} // skip: IR enum details differ from AST
            Item::TypeAlias(t) => items.push(IrItem::TypeAlias(IrTypeAlias {
                span: t.span, name: t.name.name.clone(), generics: vec![], ty: ir_type_from_ast(&t.ty), is_pub: false,
            })),
            Item::Use(u) => {
                let path_span = match &u.path {
                    aethel_syntax::ast::UsePath::Simple { span, .. } => *span,
                    aethel_syntax::ast::UsePath::Glob { span, .. } => *span,
                    aethel_syntax::ast::UsePath::Group { span, .. } => *span,
                };
                items.push(IrItem::Use(IrUseDecl {
                    span: u.span, is_pub: false,
                    path: IrUsePath::Simple { span: path_span, path: IrTypePath { span: path_span, segments: vec![] } },
                }));
            }
            Item::Mod(m) => items.push(IrItem::Mod(IrModDecl { span: m.span, name: m.name.name.clone(), body: None, is_pub: false })),
            Item::Policy(p) => items.push(IrItem::Policy(IrPolicyDef { span: p.span, name: p.name.name.clone(), generics: vec![], claims: vec![], is_pub: false })),
            Item::Effect(_) => {}
        }
    }
    items
}

fn lower_fn(f: &aethel_syntax::ast::FnDef) -> IrFnDef {
    IrFnDef {
        span: f.span,
        name: f.name.name.clone(),
        generics: f.generics.iter().map(|g| IrGenericParam { span: g.span, name: g.name.name.clone(), bounds: vec![] }).collect(),
        params: f.params.iter().map(|p| IrParam { span: p.span, name: p.name.name.clone(), ty: ir_type_from_ast(&p.ty), is_mut: p.is_mut }).collect(),
        ret_type: f.ret_type.as_ref().map(|t| ir_type_from_ast(t)).unwrap_or(IrType::Unit { span: f.span }),
        effects: IrEffectSet {
            span: f.effects.span,
            effects: f.effects.effects.iter().map(|er| IrEffectRef { span: er.span, path: ir_typ_path(&er.path) }).collect(),
        },
        body: f.body.as_ref().map(|b| lower_block(b)),
        is_pub: f.is_pub,
    }
}

fn lower_block(b: &aethel_syntax::ast::Block) -> IrBlock {
    IrBlock {
        span: b.span,
        stmts: b.stmts.iter().map(|s| lower_stmt(s)).collect(),
        tail: b.tail.as_ref().map(|e| Box::new(lower_expr(e))),
    }
}

fn lower_stmt(s: &aethel_syntax::ast::Stmt) -> IrStmt {
    use aethel_syntax::ast::Stmt;
    match s {
        Stmt::Let { span, name, ty, is_mut, init } => IrStmt::Let {
            span: *span, name: name.name.clone(), ty: ty.as_ref().map(|t| ir_type_from_ast(t)).unwrap_or(IrType::Unit { span: *span }),
            is_mut: *is_mut, init: init.as_ref().map(|e| lower_expr(e)),
        },
        Stmt::Expr { span, expr } => IrStmt::Expr { span: *span, expr: lower_expr(expr) },
        Stmt::Return { span, expr } => IrStmt::Return { span: *span, expr: expr.as_ref().map(|e| lower_expr(e)) },
        Stmt::If { span, cond, then_branch, else_branch } => IrStmt::If {
            span: *span, cond: lower_expr(cond), then_branch: lower_block(then_branch),
            else_branch: else_branch.as_ref().map(|s| Box::new(lower_stmt(s))),
        },
        Stmt::While { span, cond, body } => IrStmt::While { span: *span, cond: lower_expr(cond), body: lower_block(body) },
        Stmt::For { span, pat, iter, body } => IrStmt::For { span: *span, pat: lower_pat(pat), iter: lower_expr(iter), body: lower_block(body) },
        Stmt::Match { span, scrutinee, arms } => IrStmt::Match {
            span: *span, scrutinee: lower_expr(scrutinee),
            arms: arms.iter().map(|a| IrMatchArm { span: a.span, pat: lower_pat(&a.pat), guard: None, body: lower_expr(&a.body) }).collect(),
        },
        Stmt::Block { span, block } => IrStmt::Block { span: *span, block: lower_block(block) },
        Stmt::Use { span, .. } => IrStmt::Expr { span: *span, expr: IrExpr::Literal { span: *span, lit: IrLiteral::Unit { span: *span } } },
    }
}

fn lower_expr(e: &aethel_syntax::ast::Expr) -> IrExpr {
    use aethel_syntax::ast::{Expr, UnaryOp, BinaryOp};
    match e {
        Expr::Literal { span, lit } => IrExpr::Literal { span: *span, lit: lower_lit(lit) },
        Expr::Path { span, path } => IrExpr::Path { span: *span, path: ir_exp_path(path) },
        Expr::Tuple { span, exprs } => IrExpr::Tuple { span: *span, exprs: exprs.iter().map(|e| lower_expr(e)).collect() },
        Expr::Array { span, exprs } => IrExpr::Array { span: *span, exprs: exprs.iter().map(|e| lower_expr(e)).collect() },
        Expr::Struct { span, path, fields, base } => IrExpr::Struct {
            span: *span, path: ir_typ_path(path),
            fields: fields.iter().map(|f| IrStructExprField { span: f.span, name: f.name.name.clone(), expr: lower_expr(&f.expr) }).collect(),
            base: base.as_ref().map(|b| Box::new(lower_expr(b))),
        },
        Expr::Call { span, callee, args } => IrExpr::Call {
            span: *span, callee: Box::new(lower_expr(callee)),
            args: args.iter().map(|e| lower_expr(e)).collect(),
        },
        Expr::MethodCall { span, receiver, method, args } => IrExpr::MethodCall {
            span: *span, receiver: Box::new(lower_expr(receiver)),
            method: method.name.clone(), args: args.iter().map(|e| lower_expr(e)).collect(),
        },
        Expr::Field { span, base, field } => IrExpr::Field {
            span: *span, base: Box::new(lower_expr(base)), field: field.name.clone(),
        },
        Expr::Index { span, base, index } => IrExpr::Index {
            span: *span, base: Box::new(lower_expr(base)), index: Box::new(lower_expr(index)),
        },
        Expr::Unary { span, op, expr } => IrExpr::Unary {
            span: *span, expr: Box::new(lower_expr(expr)),
            op: match op { UnaryOp::Neg => IrUnaryOp::Neg, UnaryOp::Not => IrUnaryOp::Not, UnaryOp::Deref => IrUnaryOp::Deref },
        },
        Expr::Binary { span, op, left, right } => IrExpr::Binary {
            span: *span,
            op: match op { BinaryOp::Add => IrBinaryOp::Add, BinaryOp::Sub => IrBinaryOp::Sub, BinaryOp::Mul => IrBinaryOp::Mul, BinaryOp::Div => IrBinaryOp::Div, BinaryOp::Rem => IrBinaryOp::Rem, BinaryOp::Eq => IrBinaryOp::Eq, BinaryOp::Ne => IrBinaryOp::Ne, BinaryOp::Lt => IrBinaryOp::Lt, BinaryOp::Le => IrBinaryOp::Le, BinaryOp::Gt => IrBinaryOp::Gt, BinaryOp::Ge => IrBinaryOp::Ge, BinaryOp::And => IrBinaryOp::And, BinaryOp::Or => IrBinaryOp::Or, BinaryOp::Assign => IrBinaryOp::Assign, BinaryOp::AddAssign => IrBinaryOp::AddAssign, BinaryOp::SubAssign => IrBinaryOp::SubAssign, BinaryOp::MulAssign => IrBinaryOp::MulAssign, BinaryOp::DivAssign => IrBinaryOp::DivAssign, BinaryOp::RemAssign => IrBinaryOp::RemAssign },
            left: Box::new(lower_expr(left)), right: Box::new(lower_expr(right)),
        },
        Expr::If { span, cond, then_branch, else_branch } => IrExpr::If {
            span: *span, cond: Box::new(lower_expr(cond)),
            then_branch: Box::new(lower_expr(then_branch)),
            else_branch: else_branch.as_ref().map(|e| Box::new(lower_expr(e))),
        },
        Expr::Match { span, scrutinee, arms } => IrExpr::Match {
            span: *span, scrutinee: Box::new(lower_expr(scrutinee)),
            arms: arms.iter().map(|a| IrMatchArm { span: a.span, pat: lower_pat(&a.pat), guard: None, body: lower_expr(&a.body) }).collect(),
        },
        Expr::Block { span, block } => IrExpr::Block { span: *span, block: lower_block(block) },
        Expr::Let { span, pat, ty, is_mut, init } => IrExpr::Let {
            span: *span, pat: lower_pat(pat),
            ty: ty.as_ref().map(|t| ir_type_from_ast(t)).unwrap_or(IrType::Unit { span: *span }),
            is_mut: *is_mut, init: Box::new(lower_expr(init)),
        },
        Expr::Return { span, expr } => IrExpr::Return { span: *span, expr: expr.as_ref().map(|e| Box::new(lower_expr(e))) },
        Expr::Break { span, expr } => IrExpr::Break { span: *span, expr: expr.as_ref().map(|e| Box::new(lower_expr(e))) },
        Expr::Continue { span } => IrExpr::Continue { span: *span },
        Expr::Ask { span, model, goal, input, output_ty } => IrExpr::Ask {
            span: *span, model: ir_exp_path(model), goal: goal.clone(),
            input: Box::new(lower_expr(input)), output_ty: ir_type_from_ast(output_ty),
        },
        Expr::Verify { span, claim, policy } => IrExpr::Verify {
            span: *span, claim: Box::new(lower_expr(claim)), policy: ir_typ_path(policy),
        },
        Expr::Reason { span, prompt } => IrExpr::Reason { span: *span, prompt: prompt.clone() },
        Expr::CommitOnce { span, effect, args } => IrExpr::CommitOnce {
            span: *span, effect: IrEffectRef { span: effect.span, path: ir_typ_path(&effect.path) },
            args: args.iter().map(|e| lower_expr(e)).collect(),
        },
        Expr::New { span, ty, args } => IrExpr::New {
            span: *span, ty: ir_type_from_ast(ty), args: args.iter().map(|e| lower_expr(e)).collect(),
        },
    }
}

fn lower_lit(l: &aethel_syntax::ast::Literal) -> IrLiteral {
    use aethel_syntax::ast::Literal;
    match l {
        Literal::Unit { span } => IrLiteral::Unit { span: *span },
        Literal::Bool { span, value } => IrLiteral::Bool { span: *span, value: *value },
        Literal::Int { span, value } => IrLiteral::Int { span: *span, value: *value },
        Literal::Float { span, value } => IrLiteral::Float { span: *span, value: *value },
        Literal::String { span, value } => IrLiteral::String { span: *span, value: value.clone() },
    }
}

fn lower_pat(p: &aethel_syntax::ast::Pat) -> IrPat {
    use aethel_syntax::ast::Pat;
    match p {
        Pat::Wild { span } => IrPat::Wild { span: *span },
        Pat::Ident { span, name, is_mut } => IrPat::Ident { span: *span, name: name.name.clone(), is_mut: *is_mut },
        Pat::Literal { span, lit } => IrPat::Literal { span: *span, lit: lower_lit(lit) },
        Pat::Tuple { span, pats } => IrPat::Tuple { span: *span, pats: pats.iter().map(|p| lower_pat(p)).collect() },
        Pat::Struct { span, path, fields } => IrPat::Struct {
            span: *span, path: ir_typ_path(path),
            fields: fields.iter().map(|f| IrPatField { span: f.span, name: f.name.name.clone(), pat: f.pat.as_ref().map(|p| lower_pat(p)) }).collect(),
        },
        Pat::Enum { span, path, fields } => IrPat::Enum { span: *span, path: ir_typ_path(path), fields: fields.iter().map(|p| lower_pat(p)).collect() },
        Pat::Or { span, pats } => IrPat::Or { span: *span, pats: pats.iter().map(|p| lower_pat(p)).collect() },
        Pat::Ref { span, is_mut, pat } => IrPat::Ref { span: *span, is_mut: *is_mut, pat: Box::new(lower_pat(pat)) },
    }
}

// ── Type/Path helpers ────────────────────────────────

fn ir_type_from_ast(t: &aethel_syntax::ast::Type) -> IrType {
    use aethel_syntax::ast::Type;
    use aethel_syntax::span::Spanned;
    let span = t.span();
    match t {
        Type::Unit { .. } => IrType::Unit { span },
        Type::Never { .. } => IrType::Never { span },
        Type::Bool { .. } => IrType::Bool { span },
        Type::Int { .. } => IrType::Int { span },
        Type::Float { .. } => IrType::Float { span },
        Type::String { .. } => IrType::String { span },
        Type::Path { path, .. } => IrType::Path { span, path: ir_typ_path(path) },
        Type::Owned { ty, .. } => IrType::Owned { span, ty: Box::new(ir_type_from_ast(ty)) },
        Type::Ref { ty, is_mut, .. } => IrType::Ref { span, is_mut: *is_mut, ty: Box::new(ir_type_from_ast(ty)) },
        Type::Claim { ty, .. } => IrType::Claim { span, ty: Box::new(ir_type_from_ast(ty)) },
        Type::Verified { ty, policy, .. } => IrType::Verified { span, ty: Box::new(ir_type_from_ast(ty)), policy: Box::new(ir_type_from_ast(policy)) },
        Type::Tuple { types, .. } => IrType::Tuple { span, types: types.iter().map(ir_type_from_ast).collect() },
        Type::Array { ty, .. } => IrType::Array { span, ty: Box::new(ir_type_from_ast(ty)), size: None },
        Type::Fn { params, ret, .. } => IrType::Fn { span, params: params.iter().map(ir_type_from_ast).collect(), ret: Box::new(ir_type_from_ast(ret)), effects: IrEffectSet { span, effects: vec![] } },
    }
}

fn ir_typ_path(p: &aethel_syntax::ast::TypePath) -> IrTypePath {
    IrTypePath { span: p.span, segments: p.segments.iter().map(|s| IrPathSegment { span: s.span, name: s.name.name.clone(), args: None }).collect() }
}

fn ir_exp_path(p: &aethel_syntax::ast::ExprPath) -> IrExprPath {
    IrExprPath { span: p.span, segments: p.segments.iter().map(|s| IrPathSegment { span: s.span, name: s.name.name.clone(), args: None }).collect() }
}