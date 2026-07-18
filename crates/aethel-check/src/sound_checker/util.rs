//! Shared checker and lowering helpers.

use aethel_hir::lower as hir;
use aethel_ir::lower as ir;
use aethel_syntax::span::Span;

pub(super) fn lower_type(ty: &hir::HirType) -> ir::IrType {
    crate::types::lower_hir_type(ty)
}

pub(super) fn lower_generics(params: &[hir::HirGenericParam]) -> Vec<ir::IrGenericParam> {
    params
        .iter()
        .map(|param| ir::IrGenericParam {
            span: param.span,
            name: param.name.clone(),
            bounds: param
                .bounds
                .iter()
                .map(|bound| ir::IrTypeBound {
                    span: bound.span,
                    path: lower_type_path(&bound.path),
                })
                .collect(),
        })
        .collect()
}

pub(super) fn lower_effect_set(effects: &hir::HirEffectSet) -> ir::IrEffectSet {
    ir::IrEffectSet {
        span: effects.span,
        effects: effects
            .effects
            .iter()
            .map(|effect| ir::IrEffectRef {
                span: effect.span,
                path: lower_type_path(&effect.path),
            })
            .collect(),
    }
}

pub(super) fn lower_type_path(path: &hir::HirTypePath) -> ir::IrTypePath {
    ir::IrTypePath {
        span: path.span,
        segments: path
            .segments
            .iter()
            .map(|segment| ir::IrPathSegment {
                span: segment.span,
                name: segment.name.clone(),
                args: None,
            })
            .collect(),
    }
}

pub(super) fn lower_use_path(path: &hir::HirUsePath) -> ir::IrUsePath {
    match path {
        hir::HirUsePath::Simple { span, path } => ir::IrUsePath::Simple {
            span: *span,
            path: lower_type_path(path),
        },
        hir::HirUsePath::Glob { span, prefix } => ir::IrUsePath::Glob {
            span: *span,
            prefix: lower_type_path(prefix),
        },
        hir::HirUsePath::Group {
            span,
            prefix,
            items,
        } => ir::IrUsePath::Group {
            span: *span,
            prefix: lower_type_path(prefix),
            items: items.iter().map(lower_use_path).collect(),
        },
    }
}

pub(super) fn type_path_name(path: &hir::HirTypePath) -> String {
    path.segments
        .last()
        .map_or_else(String::new, |segment| segment.name.clone())
}

pub(super) fn expr_path_name(path: &hir::HirExprPath) -> String {
    path.segments
        .last()
        .map_or_else(String::new, |segment| segment.name.clone())
}

pub(super) fn ir_path_name(path: &ir::IrTypePath) -> String {
    path.segments
        .last()
        .map_or_else(String::new, |segment| segment.name.clone())
}

pub(super) fn hir_type_name(ty: &hir::HirType) -> Option<String> {
    match ty {
        hir::HirType::Path { path, .. } => Some(type_path_name(path)),
        _ => None,
    }
}

/// Convert a PascalCase or camelCase name to snake_case.
/// Used to generate the canonical alias for effect resolution.
/// e.g. "AuditService" → "audit_service", "PaymentGateway" → "payment_gateway"
pub(super) fn to_snake_case(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
        } else {
            result.push(c);
        }
    }
    result
}

/// Check if a receiver name matches an effect declaration.
/// Matches either the exact declared name or the exact snake_case alias.
/// e.g. "audit_service" matches "AuditService" (as its snake_case alias)
///      "AuditService" matches "AuditService" (exact)
///      "auditservice" does NOT match either
pub(super) fn effect_name_matches(receiver: &str, declared: &str) -> bool {
    receiver == declared || receiver == to_snake_case(declared)
}

pub(super) fn expr_span(expr: &hir::HirExpr) -> Span {
    match expr {
        hir::HirExpr::Literal { span, .. }
        | hir::HirExpr::Path { span, .. }
        | hir::HirExpr::Tuple { span, .. }
        | hir::HirExpr::Array { span, .. }
        | hir::HirExpr::Struct { span, .. }
        | hir::HirExpr::Call { span, .. }
        | hir::HirExpr::MethodCall { span, .. }
        | hir::HirExpr::Field { span, .. }
        | hir::HirExpr::Index { span, .. }
        | hir::HirExpr::Unary { span, .. }
        | hir::HirExpr::Binary { span, .. }
        | hir::HirExpr::If { span, .. }
        | hir::HirExpr::Match { span, .. }
        | hir::HirExpr::Block { span, .. }
        | hir::HirExpr::Let { span, .. }
        | hir::HirExpr::Return { span, .. }
        | hir::HirExpr::Break { span, .. }
        | hir::HirExpr::Continue { span }
        | hir::HirExpr::Ask { span, .. }
        | hir::HirExpr::Verify { span, .. }
        | hir::HirExpr::Reason { span, .. }
        | hir::HirExpr::CommitOnce { span, .. }
        | hir::HirExpr::New { span, .. } => *span,
    }
}
