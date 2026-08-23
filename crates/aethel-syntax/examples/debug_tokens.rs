use aethel_syntax::{lex, FileId};

fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/refund/invalid_unverified.aet");
    let source = std::fs::read_to_string(&path).unwrap();
    println!("Source length: {}", source.len());
    let tokens = lex(&source, FileId::new(0));
    println!("Tokens ({}):", tokens.len());
    for (i, t) in tokens.iter().enumerate() {
        println!("{:3}: {:?}", i, t.kind);
    }
}
