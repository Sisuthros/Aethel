//! Snapshot testing utilities.

use aethel_syntax::diagnostic::Diagnostics;

/// Assert that diagnostics match expected snapshot.
pub fn assert_diagnostics_snapshot(diagnostics: &Diagnostics, snapshot_name: &str) {
    let serialized = serde_json::to_string_pretty(diagnostics).unwrap();
    insta::assert_snapshot!(snapshot_name, serialized);
}

/// Assert that a program compiles without errors.
pub fn assert_compiles(source: &str) {
    let file_id = aethel_syntax::span::FileId::new(0);
    let tokens = aethel_syntax::lexer::lex(source, file_id);
    let (module, diagnostics) = aethel_syntax::parser::parse(&tokens, file_id);
    assert!(
        !diagnostics.has_errors(),
        "Parse errors: {:?}",
        diagnostics.errors()
    );

    let (_ir, diagnostics) = aethel_check::checker::check_module(&module, file_id);
    assert!(
        !diagnostics.has_errors(),
        "Check errors: {:?}",
        diagnostics.errors()
    );
}

/// Assert that a program fails to compile with specific error.
pub fn assert_compile_error(
    source: &str,
    expected_code: aethel_syntax::diagnostic::DiagnosticCode,
) {
    let file_id = aethel_syntax::span::FileId::new(0);
    let tokens = aethel_syntax::lexer::lex(source, file_id);
    let (module, diagnostics) = aethel_syntax::parser::parse(&tokens, file_id);

    if diagnostics.has_errors() {
        assert!(diagnostics.items.iter().any(|d| d.code == expected_code));
        return;
    }

    let (_, diagnostics) = aethel_check::checker::check_module(&module, file_id);
    assert!(diagnostics.has_errors());
    assert!(diagnostics.items.iter().any(|d| d.code == expected_code));
}
