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
            fn f(c: Claim<Order>) -> int uses Pay: {
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
}
