use aethel_syntax::{lex, FileId};

fn main() {
    let source =
        std::fs::read_to_string(r"<repo>\examples\refund\invalid_unverified.aet")
            .unwrap();
    println!("Source length: {}", source.len());
    let tokens = lex(&source, FileId::new(0));
    println!("Tokens ({}):", tokens.len());
    for (i, t) in tokens.iter().enumerate() {
        println!("{:3}: {:?}", i, t.kind);
    }
}
