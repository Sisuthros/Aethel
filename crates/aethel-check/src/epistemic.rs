//! Epistemic type rules — the core guarantee of Aethel.
//!
//! These rules enforce the Claim → Verified → Permit → Effect chain.
//! Each rule corresponds to a diagnostic code AE-EPISTEMIC-XXX.

use aethel_hir::lower::*;
use aethel_ir::lower::*;
use aethel_syntax::diagnostic::{DiagnosticCode, DiagnosticSeverity};
use aethel_syntax::span::Span;
use crate::types::{lower_hir_type, default_span};

/// Check that a Claim is properly verified before use.
///
/// **AE-EPISTEMIC-001**: A `Claim<T>` value was used where `Verified<T, Policy>` is required.
/// This is the core epistemic guarantee — you cannot use an unverified claim.
pub fn check_claim_not_verified(ctx: &mut CheckContext, span: Span, expected: &IrType, found: &IrType) {
    use IrType::*;
    match (expected, found) {
        (Verified { .. }, Claim { .. }) => {
            ctx.error(
                DiagnosticCode::new("AE-EPISTEMIC-001"),
                "unverified claim used where verified value is required — use `verify` with a policy",
                span,
            );
        }
        (Path { path, .. }, Claim { .. }) if is_verified_type(path) => {
            ctx.error(
                DiagnosticCode::new("AE-EPISTEMIC-001"),
                "unverified claim used where verified value is required",
                span,
            );
        }
        _ => {}
    }
}

/// Check that a Verified type has the correct policy.
///
/// **AE-EPISTEMIC-003**: Policy mismatch — the verified value has a different policy than required.
pub fn check_policy_match(ctx: &mut CheckContext, span: Span, expected_policy: &IrType, actual_policy: &IrType) {
    if !types_equal(expected_policy, actual_policy) {
        ctx.error(
            DiagnosticCode::new("AE-EPISTEMIC-003"),
            &format!("policy mismatch: expected {}, found {}", format_type(expected_policy), format_type(actual_policy)),
            span,
        );
    }
}

/// Check that a Claim doesn't escape its verification scope.
///
/// **AE-EPISTEMIC-004**: A Claim value escapes without being verified.
/// This prevents leaking unverified claims into storage or external boundaries.
pub fn check_claim_escape(ctx: &mut CheckContext, span: Span, ty: &IrType) {
    use IrType::*;
    if let Claim { .. } = ty {
        ctx.error(
            DiagnosticCode::new("AE-EPISTEMIC-004"),
            "claim value escapes without verification — all claims must be verified before use",
            span,
        );
    }
}

/// Check that verify operation succeeds.
///
/// **AE-EPISTEMIC-005**: Verification failed — the evidence doesn't satisfy the policy.
pub fn check_verify_failed(ctx: &mut CheckContext, span: Span, claim_ty: &IrType, policy: &IrType) {
    ctx.error(
        DiagnosticCode::new("AE-EPISTEMIC-005"),
        &format!("verification failed: claim of type {} does not satisfy policy {}", format_type(claim_ty), format_type(policy)),
        span,
    );
}

/// Check epistemic budget limits.
///
/// **AE-EPISTEMIC-006**: Too many pending claims — budget exceeded.
pub fn check_budget_exceeded(ctx: &mut CheckContext, span: Span, current: usize, limit: usize) {
    if current >= limit {
        ctx.error(
            DiagnosticCode::new("AE-EPISTEMIC-006"),
            &format!("epistemic budget exceeded: {} pending claims (limit {})", current, limit),
            span,
        );
    }
}

/// Check that a Verified type is used where required.
///
/// **AE-EPISTEMIC-002**: Verified value required but not provided.
pub fn check_verified_required(ctx: &mut CheckContext, span: Span, expected: &IrType) {
    if is_verified_type_path(expected) {
        ctx.error(
            DiagnosticCode::new("AE-EPISTEMIC-002"),
            &format!("verified value required (expected {})", format_type(expected)),
            span,
        );
    }
}

/// Type environment extension for epistemic checking.
pub struct CheckContext {
    pub file_id: aethel_syntax::span::FileId,
    pub diagnostics: aethel_syntax::diagnostic::Diagnostics,
    pub effect_registry: aethel_effects::EffectRegistry,
    pub type_env: crate::checker::TypeEnvironment,
    pub policy_registry: crate::checker::PolicyRegistry,
    /// Pending claims counter for budget checking.
    pub pending_claims: usize,
    /// Maximum allowed pending claims (budget).
    pub claim_budget: usize,
}

impl CheckContext {
    pub fn new(file_id: aethel_syntax::span::FileId) -> Self {
        Self {
            file_id,
            diagnostics: aethel_syntax::diagnostic::Diagnostics::new(),
            effect_registry: aethel_effects::EffectRegistry::default(),
            type_env: crate::checker::TypeEnvironment::default(),
            policy_registry: crate::checker::PolicyRegistry::default(),
            pending_claims: 0,
            claim_budget: 100, // default budget
        }
    }

    pub fn error(&mut self, code: DiagnosticCode, message: &str, span: Span) {
        use aethel_syntax::diagnostic::DiagnosticBuilder;
        let diag = DiagnosticBuilder::error(code, message)
            .primary_label(span, "here")
            .build();
        self.diagnostics.push(diag);
    }

    pub fn note(&mut self, code: DiagnosticCode, message: &str, span: Span) {
        use aethel_syntax::diagnostic::DiagnosticBuilder;
        let diag = DiagnosticBuilder::note_severity(code, message)
            .primary_label(span, "here")
            .build();
        self.diagnostics.push(diag);
    }

    /// Increment pending claims counter.
    pub fn claim_created(&mut self, span: Span) {
        self.pending_claims += 1;
        check_budget_exceeded(self, span, self.pending_claims, self.claim_budget);
    }

    /// Decrement when claim is verified.
    pub fn claim_verified(&mut self) {
        if self.pending_claims > 0 {
            self.pending_claims -= 1;
        }
    }
}

fn is_verified_type(path: &IrTypePath) -> bool {
    path.segments.last().map(|s| s.name == "Verified").unwrap_or(false)
}

fn is_verified_type_path(ty: &IrType) -> bool {
    use IrType::*;
    match ty {
        Verified { .. } => true,
        Path { path, .. } => is_verified_type(path),
        _ => false,
    }
}

fn types_equal(a: &IrType, b: &IrType) -> bool {
    use IrType::*;
    match (a, b) {
        (Unit { .. }, Unit { .. }) => true,
        (Never { .. }, Never { .. }) => true,
        (Bool { .. }, Bool { .. }) => true,
        (Int { .. }, Int { .. }) => true,
        (Float { .. }, Float { .. }) => true,
        (String { .. }, String { .. }) => true,
        (Path { path: pa, .. }, Path { path: pb, .. }) => type_paths_equal(pa, pb),
        (Ref { is_mut: ma, ty: ta, .. }, Ref { is_mut: mb, ty: tb, .. }) => ma == mb && types_equal(ta, tb),
        (Owned { ty: ta, .. }, Owned { ty: tb, .. }) => types_equal(ta, tb),
        (Claim { ty: ta, .. }, Claim { ty: tb, .. }) => types_equal(ta, tb),
        (Verified { ty: ta, policy: pa, .. }, Verified { ty: tb, policy: pb, .. }) => types_equal(ta, tb) && types_equal(pa, pb),
        (Array { ty: ta, size: sa, .. }, Array { ty: tb, size: sb, .. }) => types_equal(ta, tb) && sa.as_ref().map(|e| expr_equal(e, sb.as_ref().unwrap())).unwrap_or(sb.is_none()),
        (Tuple { types: ta, .. }, Tuple { types: tb, .. }) => ta.len() == tb.len() && ta.iter().zip(tb).all(|(a, b)| types_equal(a, b)),
        (Fn { params: pa, ret: ra, effects: ea, .. }, Fn { params: pb, ret: rb, effects: eb, .. }) => {
            pa.len() == pb.len() && pa.iter().zip(pb).all(|(a, b)| types_equal(a, b)) && types_equal(ra, rb) && effect_sets_equal(ea, eb)
        },
        _ => false,
    }
}

fn type_paths_equal(a: &IrTypePath, b: &IrTypePath) -> bool {
    a.segments.len() == b.segments.len() && a.segments.iter().zip(&b.segments).all(|(sa, sb)| {
        sa.name == sb.name && sa.args.is_some() && sb.args.is_some() && generic_args_equal(sa.args.as_ref().unwrap(), sb.args.as_ref().unwrap())
    })
}

fn generic_args_equal(a: &IrGenericArgs, b: &IrGenericArgs) -> bool {
    a.args.len() == b.args.len() && a.args.iter().zip(&b.args).all(|(aa, bb)| match (aa, bb) {
        (IrGenericArg::Type { ty: ta, .. }, IrGenericArg::Type { ty: tb, .. }) => types_equal(ta, tb),
        (IrGenericArg::Const { expr: ea, .. }, IrGenericArg::Const { expr: eb, .. }) => expr_equal(ea, eb),
        _ => false,
    })
}

fn expr_equal(a: &IrExpr, b: &IrExpr) -> bool {
    use IrExpr::*;
    match (a, b) {
        (Literal { lit: la, .. }, Literal { lit: lb, .. }) => literal_equal(la, lb),
        (Path { path: pa, .. }, Path { path: pb, .. }) => expr_paths_equal(pa, pb),
        _ => false,
    }
}

fn literal_equal(a: &IrLiteral, b: &IrLiteral) -> bool {
    use IrLiteral::*;
    match (a, b) {
        (Unit { .. }, Unit { .. }) => true,
        (Bool { value: va, .. }, Bool { value: vb, .. }) => va == vb,
        (Int { value: va, .. }, Int { value: vb, .. }) => va == vb,
        (Float { value: va, .. }, Float { value: vb, .. }) => (va - vb).abs() < f64::EPSILON,
        (String { value: va, .. }, String { value: vb, .. }) => va == vb,
        _ => false,
    }
}

fn expr_paths_equal(a: &IrExprPath, b: &IrExprPath) -> bool {
    a.segments.len() == b.segments.len() && a.segments.iter().zip(&b.segments).all(|(sa, sb)| {
        sa.name == sb.name && sa.args.is_some() && sb.args.is_some() && generic_args_equal(sa.args.as_ref().unwrap(), sb.args.as_ref().unwrap())
    })
}

fn effect_sets_equal(a: &IrEffectSet, b: &IrEffectSet) -> bool {
    a.effects.len() == b.effects.len() && a.effects.iter().zip(&b.effects).all(|(ea, eb)| effect_refs_equal(ea, eb))
}

fn effect_refs_equal(a: &IrEffectRef, b: &IrEffectRef) -> bool {
    type_paths_equal(&a.path, &b.path)
}

fn format_type(ty: &IrType) -> String {
    use IrType::*;
    match ty {
        Unit { .. } => "()".into(),
        Never { .. } => "!".into(),
        Bool { .. } => "bool".into(),
        Int { .. } => "int".into(),
        Float { .. } => "float".into(),
        String { .. } => "string".into(),
        Path { path, .. } => format_type_path(path),
        Ref { is_mut, ty, .. } => format!("&{} {}", if *is_mut { "mut " } else { "" }, format_type(ty)),
        Owned { ty, .. } => format!("owned {}", format_type(ty)),
        Claim { ty, .. } => format!("Claim<{}>", format_type(ty)),
        Verified { ty, policy, .. } => format!("Verified<{}, {}>", format_type(ty), format_type(policy)),
        Array { ty, size, .. } => format!("[{}; {}]", format_type(ty), size.as_ref().map(|e| format!("{:?}", e)).unwrap_or("?".into())),
        Tuple { types, .. } => format!("({})", types.iter().map(format_type).collect::<Vec<_>>().join(", ")),
        Fn { params, ret, effects, .. } => format!("fn({}) -> {} [{}]", params.iter().map(format_type).collect::<Vec<_>>().join(", "), format_type(ret), effects.effects.len()),
    }
}

fn format_type_path(path: &IrTypePath) -> String {
    path.segments.iter().map(|s| {
        let args = s.args.as_ref().map(|a| format!("<{}>", a.args.iter().map(|arg| match arg {
            IrGenericArg::Type { ty, .. } => format_type(ty),
            IrGenericArg::Const { expr, .. } => format!("{:?}", expr),
        }).collect::<Vec<_>>().join(", "))).unwrap_or_default();
        format!("{}{}", s.name, args)
    }).collect::<Vec<_>>().join("::")
}