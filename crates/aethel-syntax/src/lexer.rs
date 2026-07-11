//! Lexer for Aethel source code.

use logos::Logos;
use crate::span::{ByteOffset, FileId, Span, Spanned};

/// Token kinds for the Aethel lexer.
#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r\n\f]+", skip r"//[^\n]*", skip r"/\*([^*]|\*[^/])*\*/")]
pub enum TokenKind {
    // Keywords
    #[token("fn")]
    KwFn,
    #[token("let")]
    KwLet,
    #[token("mut")]
    KwMut,
    #[token("return")]
    KwReturn,
    #[token("if")]
    KwIf,
    #[token("else")]
    KwElse,
    #[token("while")]
    KwWhile,
    #[token("for")]
    KwFor,
    #[token("in")]
    KwIn,
    #[token("match")]
    KwMatch,
    #[token("type")]
    KwType,
    #[token("struct")]
    KwStruct,
    #[token("enum")]
    KwEnum,
    #[token("use")]
    KwUse,
    #[token("mod")]
    KwMod,
    #[token("pub")]
    KwPub,
    #[token("uses")]
    KwUses,
    #[token("ask")]
    KwAsk,
    #[token("verify")]
    KwVerify,
    #[token("commit")]
    KwCommit,
    #[token("once")]
    KwOnce,
    #[token("Claim")]
    KwClaim,
    #[token("Verified")]
    KwVerified,
    #[token("Policy")]
    KwPolicy,
    #[token("Receipt")]
    KwReceipt,
    #[token("Budget")]
    KwBudget,
    #[token("Context")]
    KwContext,
    #[token("TrustedRegion")]
    KwTrustedRegion,
    #[token("UntrustedRegion")]
    KwUntrustedRegion,
    #[token("assert")]
    KwAssert,

    // Literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    String(String),

    #[regex(r"-?\d+\.\d+([eE][+-]?\d+)?", |lex| lex.slice().parse().ok())]
    Float(f64),

    #[regex(r"-?\d+", |lex| lex.slice().parse().ok())]
    Int(i64),

    #[regex(r"true|false", |lex| lex.slice().parse().ok())]
    Bool(bool),

    // Identifiers
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    // Operators and punctuation
    #[token("=")]
    Eq,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,
    #[token(";")]
    Semi,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("|")]
    Pipe,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Ne,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("!")]
    Bang,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("..")]
    DotDot,
    #[token("...")]
    DotDotDot,
    #[token("@")]
    At,
    #[token("?")]
    Question,
}

impl TokenKind {
    /// Get the token kind as a static string for diagnostics.
    pub fn name(&self) -> &'static str {
        match self {
            TokenKind::KwFn => "fn",
            TokenKind::KwLet => "let",
            TokenKind::KwMut => "mut",
            TokenKind::KwReturn => "return",
            TokenKind::KwIf => "if",
            TokenKind::KwElse => "else",
            TokenKind::KwWhile => "while",
            TokenKind::KwFor => "for",
            TokenKind::KwIn => "in",
            TokenKind::KwMatch => "match",
            TokenKind::KwType => "type",
            TokenKind::KwStruct => "struct",
            TokenKind::KwEnum => "enum",
            TokenKind::KwUse => "use",
            TokenKind::KwMod => "mod",
            TokenKind::KwPub => "pub",
            TokenKind::KwUses => "uses",
            TokenKind::KwAsk => "ask",
            TokenKind::KwVerify => "verify",
            TokenKind::KwCommit => "commit",
            TokenKind::KwOnce => "once",
            TokenKind::KwClaim => "Claim",
            TokenKind::KwVerified => "Verified",
            TokenKind::KwPolicy => "Policy",
            TokenKind::KwReceipt => "Receipt",
            TokenKind::KwBudget => "Budget",
            TokenKind::KwContext => "Context",
            TokenKind::KwTrustedRegion => "TrustedRegion",
            TokenKind::KwUntrustedRegion => "UntrustedRegion",
            TokenKind::KwAssert => "assert",
            TokenKind::String(_) => "string",
            TokenKind::Float(_) => "float",
            TokenKind::Int(_) => "int",
            TokenKind::Bool(_) => "bool",
            TokenKind::Ident(_) => "identifier",
            TokenKind::Eq => "=",
            TokenKind::Colon => ":",
            TokenKind::ColonColon => "::",
            TokenKind::Semi => ";",
            TokenKind::Comma => ",",
            TokenKind::Dot => ".",
            TokenKind::Arrow => "->",
            TokenKind::FatArrow => "=>",
            TokenKind::Pipe => "|",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::Lt => "<",
            TokenKind::Gt => ">",
            TokenKind::Le => "<=",
            TokenKind::Ge => ">=",
            TokenKind::EqEq => "==",
            TokenKind::Ne => "!=",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            TokenKind::Bang => "!",
            TokenKind::AndAnd => "&&",
            TokenKind::OrOr => "||",
            TokenKind::DotDot => "..",
            TokenKind::DotDotDot => "...",
            TokenKind::At => "@",
            TokenKind::Question => "?",
        }
    }
}

/// A token with its span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}

/// Lexer that produces tokens with spans.
pub struct Lexer<'a> {
    logos: logos::Lexer<'a, TokenKind>,
    file_id: FileId,
    source: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, file_id: FileId) -> Self {
        Self {
            logos: TokenKind::lexer(source),
            file_id,
            source,
        }
    }

    pub fn next(&mut self) -> Option<Token> {
        let kind = self.logos.next()?;
        let start = ByteOffset(self.logos.span().start as u32);
        let end = ByteOffset(self.logos.span().end as u32);
        let span = Span::new(self.file_id, start, end);
        Some(Token::new(kind, span))
    }

    pub fn collect(self) -> Vec<Token> {
        self.into_iter().collect()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

/// Lex a source string into tokens.
pub fn lex(source: &str, file_id: FileId) -> Vec<Token> {
    Lexer::new(source, file_id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::FileId;

    #[test]
    fn test_lex_keywords() {
        let source = r#"
fn let mut return if else while for in match
type struct enum use mod pub
uses ask verify commit once
Claim Verified Policy Receipt Budget Context
TrustedRegion UntrustedRegion assert
"#;
        let tokens = lex(source, FileId::new(0));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwFn)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwLet)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwUses)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwAsk)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::KwClaim)));
    }

    #[test]
    fn test_lex_literals() {
        let source = r#""hello" 42 3.14 true false"#;
        let tokens = lex(source, FileId::new(0));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::String(ref s) if s == "hello")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Int(42))));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Float(f) if (f - 3.14).abs() < 0.01)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Bool(true))));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Bool(false))));
    }

    #[test]
    fn test_lex_operators() {
        let source = "= : :: ; , . -> => | ( ) { } [ ] < > <= >= == != + - * / % ! && || .. ... @ ?";
        let tokens = lex(source, FileId::new(0));
        assert_eq!(tokens.len(), 33);
    }
}