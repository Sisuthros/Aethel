use aethel_syntax::{lex, FileId};

fn main() {
    let source = "= : :: ; , . -> => | ( ) { } [ ] < > <= >= == != + - * / % ! && || .. ... @ ? & %= += -= *= /= |= ^= <<= >>= &&= ||=";
    let tokens = lex(source, FileId::new(0));
    println!("Token count: {}", tokens.len());
    for t in tokens {
        println!("  {:?}", t.kind);
    }
}
