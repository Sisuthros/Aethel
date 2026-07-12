//! Main type checker orchestration.

use aethel_hir::lower::HirModule;
use aethel_ir::lower::{IrModule, IrType};
use aethel_syntax::diagnostic::{Diagnostics, DiagnosticCode, DiagnosticSeverity};
use aethel_syntax::span::{FileId, Span};
use aethel_effects::EffectRegistry;
use crate::types::HirExprSpan;
use indexmap::IndexMap;
use std::collections::HashMap;

/// Type checking context.
pub struct CheckContext {
    pub file_id: FileId,
    pub diagnostics: Diagnostics,
    pub effect_registry: EffectRegistry,
    pub type_env: TypeEnvironment,
    pub policy_registry: PolicyRegistry,
}

/// Type environment for checking.
#[derive(Debug, Default)]
pub struct TypeEnvironment {
    pub variables: IndexMap<String, VariableInfo>,
    pub type_defs: IndexMap<String, TypeDefinition>,
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
        }
    }

    pub fn error(&mut self, code: DiagnosticCode, message: &str, span: Span) {
        use aethel_syntax::diagnostic::DiagnosticBuilder;
        let diag = DiagnosticBuilder::error(code, message).primary_label(span, "here").build();
        self.diagnostics.push(diag);
    }

    pub fn note(&mut self, code: DiagnosticCode, message: &str, span: Span) {
        use aethel_syntax::diagnostic::DiagnosticBuilder;
        let diag = DiagnosticBuilder::note(code, message).primary_label(span, "here").build();
        self.diagnostics.push(diag);
    }
}

/// Check a HIR module and produce IR.
pub fn check_module(hir: &HirModule, file_id: FileId) -> (IrModule, Diagnostics) {
    let mut ctx = CheckContext::new(file_id);

    // Register built-in effects
    ctx.effect_registry.register_builtin("Model", &[]);
    ctx.effect_registry.register_builtin("PaymentGateway", &[]);

    // Collect type definitions
    for item in &hir.items {
        collect_type_defs(&mut ctx, item);
    }

    // Collect policies
    for item in &hir.items {
        collect_policies(&mut ctx, item);
    }

    // Check items
    let mut ir_items = Vec::new();
    for item in &hir.items {
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

fn collect_type_defs(ctx: &mut CheckContext, item: &aethel_hir::lower::HirItem) {
    use aethel_hir::lower::HirItem;
    match item {
        HirItem::Struct(s) => {
            let mut fields = IndexMap::new();
            for field in &s.fields {
                fields.insert(field.name.clone(), field.ty.clone());
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
            Some(IrExpr::Path { span, path: lower_expr_path(path) })
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
        HirExpr::Verify { claim, policy, .. } => {
            // EPISTEMIC TYPE RULE: Claim<T> -> Verified<T, Policy>
            // This is where AE-EPISTEMIC-001 is enforced
            let claim_expr = check_expr(ctx, claim)?;
            let claim_ty = claim_expr.ty();
            
            // Check if claim is Claim<T>
            if let aethel_ir::lower::IrType::Claim { ty: inner, .. } = &claim_ty {
                // This is a Claim<T> - need to verify it produces Verified<T, Policy>
                // The verify expression itself should produce Verified<T, Policy>
                Some(IrExpr::Verify {
                    span,
                    claim: Box::new(claim_expr),
                    policy: lower_type_path(policy),
                })
            } else {
                // Not a Claim - error
                ctx.error(
                    aethel_syntax::diagnostic::codes::EPISTEMIC_CLAIM_NOT_VERIFIED,
                    "expected `Claim<T>` as argument to `verify`",
                    claim_expr.span(),
                );
                Some(IrExpr::Verify {
                    span,
                    claim: Box::new(claim_expr),
                    policy: lower_type_path(policy),
                })
            }
        }
        HirExpr::CommitOnce { effect, args, .. } => {
            Some(IrExpr::CommitOnce {
                span,
                effect: lower_effect_ref(effect),
                args: args.iter().map(|a| check_expr(ctx, a)).collect::<Option<Vec<_>>>()?,
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
                pat: f.pat.as_ref().and_then(|p| check_pat(ctx, p)).map(Box::new),
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
        ty: t.ty.clone(),
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
        body: m.body.as_ref().map(|b| {
            let (items, _) = crate::check_module(b, m.span.file);
            items
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
            ty: c.ty.clone(),
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
                    aethel_hir::lower::HirGenericArg::Const { span, expr } => aethel_ir::lower::IrGenericArg::Const { span: *span, expr: expr.clone() },
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
                    aethel_hir::lower::HirGenericArg::Type { span, ty } => aethel_ir::lower::IrGenericArg::Type { span: *span, ty: ty.clone() },
                    aethel_hir::lower::HirGenericArg::Const { span, expr } => aethel_ir::lower::IrGenericArg::Const { span: *span, expr: expr.clone() },
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

// Extend TypeEnvironment with scope management
impl TypeEnvironment {
    fn enter_scope(&mut self) {
        self.variables = self.variables.clone();
    }

    fn exit_scope(&mut self) {
        // In a real impl, we'd track scope depth
    }
}

// Add ty() method to IrExpr for type inference
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
            IrExpr::CommitOnce { span, .. } => *span,
            IrExpr::New { span, .. } => *span,
        }
    }
}