#[test]
fn breaker_009_evidence_kind_mismatch() {
    // BREAKER 009 — Evidence mismatch: wrong evidence kind for policy
    // Policy requires HumanReview but verify() provides none
    let source = r#"
struct Order { id: string, total: int }

fn bad_evidence(c: Claim<Order>) -> int
uses Payment:
    {
        let v = verify(c, OrderPolicy);
        return payment.charge(v);
    }

effect Payment {
    fn charge(order: Verified<Order, OrderPolicy>) -> int
}

policy OrderPolicy {
    Order: Order {
        evidence HumanReview "requires human sign-off"
    }
}
"#;

    let file_id = aethel_syntax::span::FileId::new(0);
    let tokens = aethel_syntax::lexer::lex(source, file_id);
    let (module, diagnostics) = aethel_syntax::parser::parse(&tokens, file_id);
    let (_ir, check_diagnostics) = aethel_check::checker::check_module(&module, file_id);

    // Should fail because policy requires HumanReview evidence but verify provides none
    let has_epistemical_error = check_diagnostics
        .errors()
        .iter()
        .any(|diag| diag.code == aethel_syntax::diagnostic::codes::EPISTEMIC_VERIFY_FAILED());

    assert!(
        has_epistemical_error,
        "Breaker 009 should fail: policy requires HumanReview but verify provides none. \
         Diagnostics: {:?}",
        check_diagnostics
    );
}