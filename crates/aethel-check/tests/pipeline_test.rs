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
    let (module, _diagnostics) = aethel_syntax::parser::parse(&tokens, file_id);
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

/// The parser's nesting limit exists to keep every later pass inside the stack.
/// Push the deepest shapes it still accepts through all of them (on this test
/// thread's small stack) and drop the results; an overflow would abort the run.
#[test]
fn nesting_just_under_the_limit_survives_every_pass() {
    use aethel_syntax::parser::MAX_NESTING_DEPTH;

    let depth = MAX_NESTING_DEPTH - 8;
    let sources = [
        format!(
            "fn main() -> int {{ return {}1{}; }}",
            "(".repeat(depth),
            ")".repeat(depth)
        ),
        format!(
            "fn main() -> int {{ {}{} return 1; }}",
            "{".repeat(depth),
            "}".repeat(depth)
        ),
        format!("fn main() -> int {{ return 1{}; }}", "+1".repeat(depth)),
        format!(
            "fn f(x: int) -> int {{ return x; }} fn main() -> int {{ return {}1{}; }}",
            "f(".repeat(depth),
            ")".repeat(depth)
        ),
    ];

    for source in &sources {
        let file_id = aethel_syntax::span::FileId::new(0);
        let tokens = aethel_syntax::lexer::lex(source, file_id);
        let (module, parse_diagnostics) = aethel_syntax::parser::parse(&tokens, file_id);
        assert!(!parse_diagnostics.has_errors(), "{parse_diagnostics:?}");

        let mut hir = aethel_hir::lower::lower_module(&module, file_id);
        let resolve_errors = aethel_hir::resolve::resolve_module(&mut hir);
        assert!(resolve_errors.is_empty(), "{resolve_errors:?}");

        let (_ir, diagnostics) = aethel_check::checker::check_hir_module(&hir, file_id);
        assert!(!diagnostics.has_errors(), "{diagnostics:?}");
        let (_ir, diagnostics) = aethel_check::checker::check_module(&module, file_id);
        assert!(!diagnostics.has_errors(), "{diagnostics:?}");
    }
}
