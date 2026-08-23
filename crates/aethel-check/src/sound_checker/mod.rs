//! Single sound HIR-based checker used by every Aethel CLI command.

mod lower;
mod semantic;
mod util;

use aethel_hir::lower as hir;
use aethel_ir::lower as ir;
use aethel_syntax::ast;
use aethel_syntax::diagnostic::{codes, DiagnosticBuilder, Diagnostics};
use aethel_syntax::span::{FileId, Span};

/// Check a parsed module through AST → HIR → resolve → semantic check → IR.
pub fn check_module(module: &ast::Module, file_id: FileId) -> (ir::IrModule, Diagnostics) {
    let mut hir_module = aethel_hir::lower::lower_module(module, file_id);
    let resolver_errors = aethel_hir::resolve::resolve_module(&mut hir_module);
    let (ir_module, mut diagnostics) = check_hir_module(&hir_module, file_id);
    for error in resolver_errors {
        diagnostics.push(
            DiagnosticBuilder::error(codes::UNDEFINED_VAR(), error)
                .primary_label(Span::zero(), "name resolution failed")
                .build(),
        );
    }
    (ir_module, diagnostics)
}

/// Check HIR and emit IR only after the semantic pass has completed.
pub fn check_hir_module(module: &hir::HirModule, file_id: FileId) -> (ir::IrModule, Diagnostics) {
    let diagnostics = semantic::SemanticChecker::new(module).check(module);
    (lower::lower_module(module, file_id), diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> Diagnostics {
        let file_id = FileId::new(0);
        let tokens = aethel_syntax::lexer::lex(source, file_id);
        let (module, parse_diagnostics) = aethel_syntax::parser::parse(&tokens, file_id);
        assert!(!parse_diagnostics.has_errors(), "fixture must parse");
        check_module(&module, file_id).1
    }

    #[test]
    fn rejects_type_forgery() {
        let diagnostics = check(
            r#"
            struct Order { id: string }
            policy P { Order: Order { evidence SignedAttestation "ok" } }
            fn f(c: Claim<Order>) -> Verified<Order, P> {
                let forged: Verified<Order, P> = c;
                return forged;
            }
            "#,
        );
        assert!(diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::EPISTEMIC_CLAIM_NOT_VERIFIED()));
    }

    #[test]
    fn rejects_policy_mismatch() {
        let diagnostics = check(
            r#"
            struct Order { id: string }
            policy P1 { Order: Order { evidence SignedAttestation "ok" } }
            policy P2 { Order: Order { evidence SignedAttestation "ok" } }
            effect Pay { fn send(order: Verified<Order, P1>) -> int }
            fn f(c: Claim<Order>) -> int
            uses Pay:
                {
                    let checked = verify(c, P2);
                    return pay.send(checked);
                }
            "#,
        );
        assert!(diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::EPISTEMIC_POLICY_MISMATCH()));
    }

    #[test]
    fn rejects_unconsumed_claim_parameter() {
        // BREAKER 016 — a linear Claim parameter must be consumed via verify.
        let diagnostics = check(
            r#"
            fn unused_claim(c: Claim<int>) -> int {
                let x = 42;
                return x;
            }
            "#,
        );
        assert!(diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::LINEAR_NOT_CONSUMED()));
    }

    #[test]
    fn accepts_consumed_claim_parameter() {
        // Control for breaker-016: verifying the claim consumes it, so the
        // function must type check without AE-TYPE-013.
        let diagnostics = check(
            r#"
            policy P { int: int { evidence SignedAttestation "ok" } }
            fn used_claim(c: Claim<int>) -> Verified<int, P> {
                return verify(c, P);
            }
            "#,
        );
        assert!(!diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::LINEAR_NOT_CONSUMED()));
    }

    #[test]
    fn rejects_uninitialised_verified_binding() {
        // BREAKER 021 — origin enforcement: a bare declaration is not a
        // constructor. Only `verify(claim, policy)` may produce Verified.
        let diagnostics = check(
            r#"
            struct D { v: int }
            fn origin_hole() -> int
            uses Single:
                {
                    let v: Verified<D, DPolicy>;
                    return single.do_it(v);
                }
            effect Single {
                fn do_it(d: Verified<D, DPolicy>) -> int
            }
            policy DPolicy {
                D: D { evidence SignedAttestation "test" }
            }
            "#,
        );
        assert!(diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::EPISTEMIC_VERIFIED_REQUIRED()));
    }

    #[test]
    fn accepts_matching_evidence_kind() {
        // Surface syntax `verify(c, P, evidence Kind)` with the kind the
        // policy requires must verify cleanly.
        let diagnostics = check(
            r#"
            struct Order { id: string }
            policy P {
                Order: Order { evidence SignedAttestation "ok" }
            }
            fn f(c: Claim<Order>) -> Verified<Order, P> {
                return verify(c, P, evidence SignedAttestation);
            }
            "#,
        );
        assert!(
            !diagnostics.has_errors(),
            "diagnostics: {:?}",
            diagnostics.errors()
        );
    }

    #[test]
    fn rejects_wrong_evidence_kind() {
        // BREAKER 009 — policy requires HumanReview, verify provides
        // SignedAttestation → AE-EPISTEMIC-003.
        let diagnostics = check(
            r#"
            struct Order { id: string }
            policy P {
                Order: Order { evidence HumanReview "sign-off" }
            }
            fn f(c: Claim<Order>) -> Verified<Order, P> {
                return verify(c, P, evidence SignedAttestation);
            }
            "#,
        );
        assert!(diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::EPISTEMIC_POLICY_MISMATCH()));
    }

    #[test]
    fn rejects_missing_evidence_when_required() {
        // No third argument while the policy requires evidence → AE-EPISTEMIC-005.
        let diagnostics = check(
            r#"
            struct Order { id: string }
            policy P {
                Order: Order { evidence HumanReview "sign-off" }
            }
            fn f(c: Claim<Order>) -> Verified<Order, P> {
                return verify(c, P);
            }
            "#,
        );
        assert!(diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::EPISTEMIC_VERIFY_FAILED()));
    }

    #[test]
    fn rejects_double_verification_of_same_claim() {
        // BREAKER 016 follow-up — duplication side of linearity: verifying
        // the same linear Claim twice (the double-charge shape) must be
        // rejected with AE-TYPE-012.
        let diagnostics = check(
            r#"
            policy P1 { int: int { evidence SignedAttestation "a" } }
            policy P2 { int: int { evidence SignedAttestation "b" } }
            effect A { fn run(d: Verified<int, P1>) -> int }
            effect B { fn run(d: Verified<int, P2>) -> int }
            fn f(c: Claim<int>) -> int
            uses A, B:
                {
                    let va = verify(c, P1);
                    let ra = a.run(va);
                    let vb = verify(c, P2);
                    let rb = b.run(vb);
                    return ra + rb;
                }
            "#,
        );
        assert!(diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::LINEAR_USE_AFTER_MOVE()));
    }

    #[test]
    fn consumption_through_alias_counts() {
        // `let x = c; verify(x, P)` consumes c through the alias — no
        // unconsumed-parameter error may fire.
        let diagnostics = check(
            r#"
            policy P { int: int { evidence SignedAttestation "ok" } }
            fn f(c: Claim<int>) -> Verified<int, P> {
                let x = c;
                return verify(x, P);
            }
            "#,
        );
        assert!(!diagnostics
            .errors()
            .iter()
            .any(|diag| diag.code == codes::LINEAR_NOT_CONSUMED()));
    }
}
