use aethel_syntax::{lex, parse, FileId};

fn main() {
    let source = r#"
fn refund(claim: Claim<RefundDecision>) -> Receipt
uses PaymentGateway:
    { return payments.refund(claim); }
"#;
    let tokens = lex(source, FileId::new(0));
    println!("Tokens ({}):", tokens.len());
    for t in &tokens {
        println!("  {:?}", t.kind);
    }
    
    let (module, diagnostics) = parse(&tokens, FileId::new(0));
    println!("\nDiagnostics has_errors: {}", diagnostics.has_errors());
    for d in diagnostics.errors() {
        println!("  {:?}", d);
    }
    
    println!("\nModule items: {:?}", module.items);
}
