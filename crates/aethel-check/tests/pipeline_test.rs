#[test]
fn test_checker_produces_ir_items() {
    // Test that check_module actually returns non-empty IR
    let source = "fn main() { let x = 42; }";
    let file_id = aethel_syntax::span::FileId::new(0);
    let tokens = aethel_syntax::lexer::lex(source, file_id);
    let (module, _) = aethel_syntax::parser::parse(&tokens, file_id);
    let (ir, _) = aethel_check::checker::check_module(&module, file_id);
    assert!(!ir.items.is_empty(), "IR items should not be empty!");
    assert!(ir.items.iter().any(|item| matches!(item, aethel_ir::lower::IrItem::Fn(_))),
        "Should contain at least one function");
}

#[test]
fn test_ir_contains_fn_body() {
    // Test that function bodies are properly lowered
    let source = "fn add(a: int, b: int) -> int { a + b }";
    let file_id = aethel_syntax::span::FileId::new(0);
    let tokens = aethel_syntax::lexer::lex(source, file_id);
    let (module, _) = aethel_syntax::parser::parse(&tokens, file_id);
    let (ir, _) = aethel_check::checker::check_module(&module, file_id);
    
    let fn_count = ir.items.iter().filter(|i| matches!(i, aethel_ir::lower::IrItem::Fn(_))).count();
    assert_eq!(fn_count, 1, "Should have exactly one function");
    
    if let Some(aethel_ir::lower::IrItem::Fn(f)) = ir.items.first() {
        assert_eq!(f.name, "add");
        assert!(!f.params.is_empty());
        assert!(f.body.is_some(), "Function body should be lowered");
    }
}
