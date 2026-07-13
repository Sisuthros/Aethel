use aethel_syntax::{lex, FileId};

fn main() {
    let source = r#"{ return; }"#;
    let tokens = lex(source, FileId::new(0));
    println!("Tokens ({}):", tokens.len());
    for t in &tokens {
        println!("  {:?}", t.kind);
    }
}
