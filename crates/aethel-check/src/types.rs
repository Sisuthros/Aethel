//! Type system utilities and HIR↔IR type conversion.

use aethel_hir::lower::*;
use aethel_ir::lower::*;
use aethel_syntax::span::{ByteOffset, FileId, Span};
use indexmap::IndexMap;

/// An IR expression together with the type that was inferred for it
/// during type checking. Every checked expression carries its declared
/// type so downstream passes (effects, codegen, interpreter) can
/// inspect the result without re-deriving it.
#[derive(Debug, Clone)]
pub struct TypedExpr {
    pub expr: IrExpr,
    pub ty: IrType,
}

impl TypedExpr {
    pub fn new(expr: IrExpr, ty: IrType) -> Self {
        Self { expr, ty }
    }
}

/// Check if a value type can be assigned to a target type.
pub fn check_assignable(value_ty: &IrType, target_ty: &IrType) -> Result<(), String> {
    match (value_ty, target_ty) {
        (IrType::Unit { .. }, IrType::Unit { .. })
        | (IrType::Bool { .. }, IrType::Bool { .. })
        | (IrType::Int { .. }, IrType::Int { .. })
        | (IrType::Float { .. }, IrType::Float { .. })
        | (IrType::String { .. }, IrType::String { .. }) => Ok(()),
        (IrType::Never { .. }, _) => Ok(()),
        (IrType::Path { path: p1, .. }, IrType::Path { path: p2, .. }) => {
            let n1 = p1.segments.last().map(|s| s.name.as_str()).unwrap_or("");
            let n2 = p2.segments.last().map(|s| s.name.as_str()).unwrap_or("");
            if n1 == n2 {
                Ok(())
            } else {
                Err(format!("expected `{n2}`, got `{n1}`"))
            }
        }
        (IrType::Claim { ty: v, .. }, IrType::Claim { ty: t, .. }) => check_assignable(v, t),
        (IrType::Claim { .. }, IrType::Verified { .. }) => {
            Err("unverified claim cannot cross effect boundary".into())
        }
        (
            IrType::Verified {
                ty: v, policy: vp, ..
            },
            IrType::Verified {
                ty: t, policy: tp, ..
            },
        ) => {
            check_assignable(v, t)?;
            let vpn = match vp.as_ref() {
                IrType::Path { path, .. } => {
                    path.segments.last().map(|s| s.name.as_str()).unwrap_or("")
                }
                _ => "",
            };
            let tpn = match tp.as_ref() {
                IrType::Path { path, .. } => {
                    path.segments.last().map(|s| s.name.as_str()).unwrap_or("")
                }
                _ => "",
            };
            if vpn == tpn {
                Ok(())
            } else {
                Err(format!("policy mismatch: `{vpn}` vs `{tpn}`"))
            }
        }
        _ => Err(format!("cannot assign: {}", type_to_string(value_ty))),
    }
}

/// Check if an argument type matches an effect operation parameter type.
pub fn check_effect_arg(arg_ty: &IrType, param_ty: &IrType) -> Result<(), String> {
    match (arg_ty, param_ty) {
        (IrType::Claim { .. }, IrType::Verified { .. }) => {
            Err("unverified claim cannot authorize effect".into())
        }
        _ => check_assignable(arg_ty, param_ty),
    }
}

/// Human-readable type name for error messages.
pub fn type_to_string(ty: &IrType) -> String {
    match ty {
        IrType::Unit { .. } => "()".into(),
        IrType::Never { .. } => "!".into(),
        IrType::Bool { .. } => "bool".into(),
        IrType::Int { .. } => "int".into(),
        IrType::Float { .. } => "float".into(),
        IrType::String { .. } => "string".into(),
        IrType::Path { path, .. } => path
            .segments
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join("::"),
        IrType::Claim { ty: inner, .. } => format!("Claim<{}>", type_to_string(inner)),
        IrType::Verified {
            ty: inner, policy, ..
        } => format!(
            "Verified<{}, {}>",
            type_to_string(inner),
            type_to_string(policy)
        ),
        _ => "?".into(),
    }
}

/// Convert HIR types to IR types during checking.
pub fn lower_hir_type(ty: &HirType) -> IrType {
    use HirType::*;
    match ty {
        Unit { span } => IrType::Unit { span: *span },
        Never { span } => IrType::Never { span: *span },
        Bool { span } => IrType::Bool { span: *span },
        Int { span } => IrType::Int { span: *span },
        Float { span } => IrType::Float { span: *span },
        String { span } => IrType::String { span: *span },
        Path { span, path } => IrType::Path {
            span: *span,
            path: lower_type_path(path),
        },
        Ref { span, is_mut, ty } => IrType::Ref {
            span: *span,
            is_mut: *is_mut,
            ty: Box::new(lower_hir_type(ty)),
        },
        Owned { span, ty } => IrType::Owned {
            span: *span,
            ty: Box::new(lower_hir_type(ty)),
        },
        Claim { span, ty } => IrType::Claim {
            span: *span,
            ty: Box::new(lower_hir_type(ty)),
        },
        Verified { span, ty, policy } => IrType::Verified {
            span: *span,
            ty: Box::new(lower_hir_type(ty)),
            policy: Box::new(lower_hir_type(policy)),
        },
        Budget { span } => IrType::Budget { span: *span },
        Array { span, ty, size } => IrType::Array {
            span: *span,
            ty: Box::new(lower_hir_type(ty)),
            size: size.as_ref().map(|e| Box::new(lower_hir_expr(e))),
        },
        Tuple { span, types } => IrType::Tuple {
            span: *span,
            types: types.iter().map(lower_hir_type).collect(),
        },
        Fn {
            span,
            params,
            ret,
            effects,
        } => IrType::Fn {
            span: *span,
            params: params.iter().map(lower_hir_type).collect(),
            ret: Box::new(lower_hir_type(ret)),
            effects: lower_effect_set(effects),
        },
    }
}

/// Convert HIR expression to IR expression (for type annotations in types)
pub fn lower_hir_expr(expr: &HirExpr) -> IrExpr {
    use HirExpr::*;
    let span = expr.span();
    match expr {
        Literal { lit, .. } => IrExpr::Literal {
            span,
            lit: lower_literal(lit),
        },
        Path { path, .. } => IrExpr::Path {
            span,
            path: lower_expr_path(path),
        },
        Tuple { exprs, .. } => IrExpr::Tuple {
            span,
            exprs: exprs.iter().map(lower_hir_expr).collect(),
        },
        Array { exprs, .. } => IrExpr::Array {
            span,
            exprs: exprs.iter().map(lower_hir_expr).collect(),
        },
        Struct {
            path, fields, base, ..
        } => IrExpr::Struct {
            span,
            path: lower_type_path(path),
            fields: fields
                .iter()
                .map(|f| IrStructExprField {
                    span: f.span,
                    name: f.name.clone(),
                    expr: lower_hir_expr(&f.expr),
                })
                .collect(),
            base: base.as_ref().map(|b| Box::new(lower_hir_expr(b))),
        },
        Call { callee, args, .. } => IrExpr::Call {
            span,
            callee: Box::new(lower_hir_expr(callee)),
            args: args.iter().map(lower_hir_expr).collect(),
        },
        MethodCall {
            receiver,
            method,
            args,
            ..
        } => IrExpr::MethodCall {
            span,
            receiver: Box::new(lower_hir_expr(receiver)),
            method: method.clone(),
            args: args.iter().map(lower_hir_expr).collect(),
        },
        Field { base, field, .. } => IrExpr::Field {
            span,
            base: Box::new(lower_hir_expr(base)),
            field: field.clone(),
        },
        Index { base, index, .. } => IrExpr::Index {
            span,
            base: Box::new(lower_hir_expr(base)),
            index: Box::new(lower_hir_expr(index)),
        },
        Unary { op, expr, .. } => IrExpr::Unary {
            span,
            op: lower_unary_op(op),
            expr: Box::new(lower_hir_expr(expr)),
        },
        Binary {
            op, left, right, ..
        } => IrExpr::Binary {
            span,
            op: lower_binary_op(op),
            left: Box::new(lower_hir_expr(left)),
            right: Box::new(lower_hir_expr(right)),
        },
        If {
            cond,
            then_branch,
            else_branch,
            ..
        } => IrExpr::If {
            span,
            cond: Box::new(lower_hir_expr(cond)),
            then_branch: Box::new(lower_hir_expr(then_branch)),
            else_branch: else_branch.as_ref().map(|e| Box::new(lower_hir_expr(e))),
        },
        Match {
            scrutinee, arms, ..
        } => IrExpr::Match {
            span,
            scrutinee: Box::new(lower_hir_expr(scrutinee)),
            arms: arms.iter().map(lower_match_arm).collect(),
        },
        Block { block, .. } => IrExpr::Block {
            span,
            block: lower_hir_block(block),
        },
        Let {
            pat,
            ty,
            is_mut,
            init,
            ..
        } => IrExpr::Let {
            span,
            pat: lower_pat(pat),
            ty: ty
                .as_ref()
                .map(lower_hir_type)
                .unwrap_or(IrType::Unit { span }),
            is_mut: *is_mut,
            init: Box::new(lower_hir_expr(init)),
        },
        Return { expr, .. } => IrExpr::Return {
            span,
            expr: expr.as_ref().map(|e| Box::new(lower_hir_expr(e))),
        },
        Break { expr, .. } => IrExpr::Break {
            span,
            expr: expr.as_ref().map(|e| Box::new(lower_hir_expr(e))),
        },
        Continue { span } => IrExpr::Continue { span: *span },
        Ask {
            model,
            goal,
            input,
            output_ty,
            ..
        } => IrExpr::Ask {
            span,
            model: lower_expr_path(model),
            goal: goal.clone(),
            input: Box::new(lower_hir_expr(input)),
            output_ty: lower_hir_type(output_ty),
        },
        Verify { claim, policy, .. } => IrExpr::Verify {
            span,
            claim: Box::new(lower_hir_expr(claim)),
            policy: lower_type_path(policy),
        },
        Reason { span, prompt } => IrExpr::Reason {
            span: *span,
            prompt: prompt.clone(),
        },
        CommitOnce { effect, args, .. } => IrExpr::CommitOnce {
            span,
            effect: lower_effect_ref(effect),
            args: args.iter().map(lower_hir_expr).collect(),
        },
        New { ty, args, .. } => IrExpr::New {
            span,
            ty: lower_hir_type(ty),
            args: args.iter().map(lower_hir_expr).collect(),
        },
    }
}

fn lower_literal(l: &HirLiteral) -> IrLiteral {
    use HirLiteral::*;
    match l {
        Unit { span } => IrLiteral::Unit { span: *span },
        Bool { span, value } => IrLiteral::Bool {
            span: *span,
            value: *value,
        },
        Int { span, value } => IrLiteral::Int {
            span: *span,
            value: *value,
        },
        Float { span, value } => IrLiteral::Float {
            span: *span,
            value: *value,
        },
        String { span, value } => IrLiteral::String {
            span: *span,
            value: value.clone(),
        },
    }
}

fn lower_type_path(p: &HirTypePath) -> IrTypePath {
    IrTypePath {
        span: p.span,
        segments: p
            .segments
            .iter()
            .map(|s| IrPathSegment {
                span: s.span,
                name: s.name.clone(),
                args: s.args.as_ref().map(lower_generic_args),
            })
            .collect(),
    }
}

fn lower_expr_path(p: &HirExprPath) -> IrExprPath {
    IrExprPath {
        span: p.span,
        segments: p
            .segments
            .iter()
            .map(|s| IrPathSegment {
                span: s.span,
                name: s.name.clone(),
                args: s.args.as_ref().map(lower_generic_args),
            })
            .collect(),
    }
}

fn lower_generic_args(a: &HirGenericArgs) -> IrGenericArgs {
    IrGenericArgs {
        span: a.span,
        args: a
            .args
            .iter()
            .map(|arg| match arg {
                HirGenericArg::Type { span, ty } => IrGenericArg::Type {
                    span: *span,
                    ty: lower_hir_type(ty),
                },
                HirGenericArg::Const { span, expr } => IrGenericArg::Const {
                    span: *span,
                    expr: lower_hir_expr(expr),
                },
            })
            .collect(),
    }
}

fn lower_hir_block(b: &HirBlock) -> IrBlock {
    IrBlock {
        span: b.span,
        stmts: b.stmts.iter().map(lower_hir_stmt).collect(),
        tail: b.tail.as_ref().map(|e| Box::new(lower_hir_expr(e))),
    }
}

fn lower_hir_stmt(s: &HirStmt) -> IrStmt {
    use HirStmt::*;
    match s {
        Let {
            span,
            name,
            ty,
            is_mut,
            init,
        } => IrStmt::Let {
            span: *span,
            name: name.clone(),
            ty: ty
                .as_ref()
                .map(lower_hir_type)
                .unwrap_or(IrType::Unit { span: *span }),
            is_mut: *is_mut,
            init: init.as_ref().map(lower_hir_expr),
        },
        Expr { span, expr } => IrStmt::Expr {
            span: *span,
            expr: lower_hir_expr(expr),
        },
        Return { span, expr } => IrStmt::Return {
            span: *span,
            expr: expr.as_ref().map(lower_hir_expr),
        },
        If {
            span,
            cond,
            then_branch,
            else_branch,
        } => IrStmt::If {
            span: *span,
            cond: lower_hir_expr(cond),
            then_branch: lower_hir_block(then_branch),
            else_branch: else_branch.as_ref().map(|b| Box::new(lower_hir_stmt(b))),
        },
        While { span, cond, body } => IrStmt::While {
            span: *span,
            cond: lower_hir_expr(cond),
            body: lower_hir_block(body),
        },
        For {
            span,
            pat,
            iter,
            body,
        } => IrStmt::For {
            span: *span,
            pat: lower_pat(pat),
            iter: lower_hir_expr(iter),
            body: lower_hir_block(body),
        },
        Match {
            span,
            scrutinee,
            arms,
        } => IrStmt::Match {
            span: *span,
            scrutinee: lower_hir_expr(scrutinee),
            arms: arms.iter().map(lower_match_arm).collect(),
        },
        Block { span, block } => IrStmt::Block {
            span: *span,
            block: lower_hir_block(block),
        },
    }
}

fn lower_match_arm(a: &HirMatchArm) -> IrMatchArm {
    IrMatchArm {
        span: a.span,
        pat: lower_pat(&a.pat),
        guard: a.guard.as_ref().map(lower_hir_expr),
        body: lower_hir_expr(&a.body),
    }
}

fn lower_pat(p: &HirPat) -> IrPat {
    use HirPat::*;
    let span = p.span();
    match p {
        Wild { span } => IrPat::Wild { span: *span },
        Ident { span, name, is_mut } => IrPat::Ident {
            span: *span,
            name: name.clone(),
            is_mut: *is_mut,
        },
        Literal { span, lit } => IrPat::Literal {
            span: *span,
            lit: lower_literal(lit),
        },
        Tuple { span, pats } => IrPat::Tuple {
            span: *span,
            pats: pats.iter().map(lower_pat).collect(),
        },
        Struct { span, path, fields } => IrPat::Struct {
            span: *span,
            path: lower_type_path(path),
            fields: fields
                .iter()
                .map(|f| IrPatField {
                    span: f.span,
                    name: f.name.clone(),
                    pat: f.pat.as_ref().map(lower_pat),
                })
                .collect(),
        },
        Enum { span, path, fields } => IrPat::Enum {
            span: *span,
            path: lower_type_path(path),
            fields: fields.iter().map(lower_pat).collect(),
        },
        Or { span, pats } => IrPat::Or {
            span: *span,
            pats: pats.iter().map(lower_pat).collect(),
        },
        Ref { span, is_mut, pat } => IrPat::Ref {
            span: *span,
            is_mut: *is_mut,
            pat: Box::new(lower_pat(pat)),
        },
    }
}

fn lower_unary_op(op: &HirUnaryOp) -> IrUnaryOp {
    use HirUnaryOp::*;
    match op {
        Neg => IrUnaryOp::Neg,
        Not => IrUnaryOp::Not,
        Deref => IrUnaryOp::Deref,
    }
}

fn lower_binary_op(op: &HirBinaryOp) -> IrBinaryOp {
    use HirBinaryOp::*;
    match op {
        Add => IrBinaryOp::Add,
        Sub => IrBinaryOp::Sub,
        Mul => IrBinaryOp::Mul,
        Div => IrBinaryOp::Div,
        Rem => IrBinaryOp::Rem,
        Eq => IrBinaryOp::Eq,
        Ne => IrBinaryOp::Ne,
        Lt => IrBinaryOp::Lt,
        Le => IrBinaryOp::Le,
        Gt => IrBinaryOp::Gt,
        Ge => IrBinaryOp::Ge,
        And => IrBinaryOp::And,
        Or => IrBinaryOp::Or,
        Assign => IrBinaryOp::Assign,
        AddAssign => IrBinaryOp::AddAssign,
        SubAssign => IrBinaryOp::SubAssign,
        MulAssign => IrBinaryOp::MulAssign,
        DivAssign => IrBinaryOp::DivAssign,
        RemAssign => IrBinaryOp::RemAssign,
    }
}

pub(crate) fn lower_effect_set(e: &HirEffectSet) -> IrEffectSet {
    IrEffectSet {
        span: e.span,
        effects: e.effects.iter().map(lower_effect_ref).collect(),
    }
}

fn lower_effect_ref(e: &HirEffectRef) -> IrEffectRef {
    IrEffectRef {
        span: e.span,
        path: lower_type_path(&e.path),
    }
}

/// Trait for getting span from HIR expressions.
pub trait HirExprSpan {
    fn span(&self) -> Span;
}

impl HirExprSpan for HirExpr {
    fn span(&self) -> Span {
        use HirExpr::*;
        match self {
            Literal { span, .. } => *span,
            Path { span, .. } => *span,
            Tuple { span, .. } => *span,
            Array { span, .. } => *span,
            Struct { span, .. } => *span,
            Call { span, .. } => *span,
            MethodCall { span, .. } => *span,
            Field { span, .. } => *span,
            Index { span, .. } => *span,
            Unary { span, .. } => *span,
            Binary { span, .. } => *span,
            If { span, .. } => *span,
            Match { span, .. } => *span,
            Block { span, .. } => *span,
            Let { span, .. } => *span,
            Return { span, .. } => *span,
            Break { span, .. } => *span,
            Continue { span } => *span,
            Ask { span, .. } => *span,
            Verify { span, .. } => *span,
            Reason { span, .. } => *span,
            CommitOnce { span, .. } => *span,
            New { span, .. } => *span,
        }
    }
}

impl HirExprSpan for HirPat {
    fn span(&self) -> Span {
        use HirPat::*;
        match self {
            Wild { span } => *span,
            Ident { span, .. } => *span,
            Literal { span, .. } => *span,
            Tuple { span, .. } => *span,
            Struct { span, .. } => *span,
            Enum { span, .. } => *span,
            Or { span, .. } => *span,
            Ref { span, .. } => *span,
        }
    }
}

/// Default span for synthetic nodes.
pub fn default_span() -> Span {
    Span::new(FileId::new(0), ByteOffset(0), ByteOffset(0))
}
