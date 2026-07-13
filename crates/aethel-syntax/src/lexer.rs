//! Lexer for Aethel source code.

use logos::Logos;
use crate::span::{ByteOffset, FileId, Span};

/// Token kinds for the Aethel lexer.
#[derive(Logos, Debug, Clone, PartialEq)]
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
    #[token("owned")]
    KwOwned,
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
    #[token("effect")]
    KwEffect,
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
        #[token("reason")]
        KwReason,
    #[token("SignedAttestation")]
    KwSignedAttestation,
    #[token("CryptographicProof")]
    KwCryptographicProof,
    #[token("AuditLog")]
    KwAuditLog,
    #[token("HumanReview")]
    KwHumanReview,
    #[token("break")]
    KwBreak,
    #[token("continue")]
    KwContinue,
    #[token("new")]
    KwNew,

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
    #[token("%=")]
    PercentEq,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("!")]
    Bang,
    #[token("&")]
        And,
        #[token("&&")]
        AndAnd,
        #[token("&=")]
        AndEq,
        #[token("|=")]
        PipeEq,
        #[token("||")]
        OrOr,
        #[token("||=")]
        OrEq,
        #[token("^")]
        Xor,
        #[token("^=")]
        XorEq,
        #[token("<<")]
        Shl,
        #[token("<<=")]
        ShlEq,
    #[token(">>")]
    Shr,
    #[token(">>=")]
    ShrEq,
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
            TokenKind::KwOwned => "owned",
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
            TokenKind::KwEffect => "effect",
            TokenKind::KwReceipt => "Receipt",
                        TokenKind::KwBudget => "Budget",
                        TokenKind::KwContext => "Context",
                        TokenKind::KwTrustedRegion => "TrustedRegion",
                        TokenKind::KwUntrustedRegion => "UntrustedRegion",
                        TokenKind::KwAssert => "assert",
                        TokenKind::KwReason => "reason",
                        TokenKind::KwSignedAttestation => "SignedAttestation",
            TokenKind::KwCryptographicProof => "CryptographicProof",
            TokenKind::KwAuditLog => "AuditLog",
            TokenKind::KwHumanReview => "HumanReview",
            TokenKind::KwBreak => "break",
            TokenKind::KwContinue => "continue",
            TokenKind::KwNew => "new",
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
            TokenKind::PercentEq => "%=",
            TokenKind::PlusEq => "+=",
            TokenKind::MinusEq => "-=",
            TokenKind::StarEq => "*=",
            TokenKind::SlashEq => "/=",
            TokenKind::Bang => "!",
            TokenKind::And => "&",
            TokenKind::AndAnd => "&&",
            TokenKind::AndEq => "&=",
            TokenKind::PipeEq => "|=",
            TokenKind::OrOr => "||",
            TokenKind::OrEq => "||=",
            TokenKind::Xor => "^",
            TokenKind::XorEq => "^=",
            TokenKind::Shl => "<<",
            TokenKind::ShlEq => "<<=",
            TokenKind::Shr => ">>",
            TokenKind::ShrEq => ">>=",
            TokenKind::DotDot => "..",
            TokenKind::DotDotDot => "...",
            TokenKind::At => "@",
            TokenKind::Question => "?",
        }
    }
    
    /// Check if this token kind is an identifier
    pub fn is_ident(&self) -> bool {
        matches!(self, TokenKind::Ident(_))
    }
}

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

/// A static empty token for fallback when parser is at EOF.
pub static EMPTY_TOKEN: std::sync::OnceLock<Token> = std::sync::OnceLock::new();

pub fn lex(source: &str, file_id: FileId) -> Vec<Token> {
    let mut lexer = TokenKind::lexer(source);
    let mut tokens = Vec::new();

    while let Some(result) = lexer.next() {
        match result {
            Ok(kind) => {
                let span = Span::new(
                    file_id,
                    ByteOffset(lexer.span().start as u32),
                    ByteOffset(lexer.span().end as u32),
                );
                tokens.push(Token::new(kind, span));
            }
            Err(_) => {
                // Skip invalid tokens
                let span = Span::new(
                    file_id,
                    ByteOffset(lexer.span().start as u32),
                    ByteOffset(lexer.span().end as u32),
                );
            }
        }
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::FileId;

    #[test]
    fn test_lex_keywords() {
        let source = "fn let mut return owned if else while for in match type struct enum use mod pub uses ask verify commit once Claim Verified Policy effect Receipt Budget Context TrustedRegion UntrustedRegion assert SignedAttestation CryptographicProof AuditLog HumanReview break continue new";
        let tokens = lex(source, FileId::new(0));
        assert_eq!(tokens.len(), 39); // 39 keywords
    }

    #[test]
    fn test_lex_literals() {
        let source = "\"hello\" 42 3.14 true false";
        let tokens = lex(source, FileId::new(0));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::String(ref s) if s == "hello")));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Int(42))));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Float(f) if (f - 3.14).abs() < 0.01)));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Bool(true))));
        assert!(tokens
            .iter()
            .any(|t| matches!(t.kind, TokenKind::Bool(false))));
    }

    #[test]
        fn test_lex_operators() {
            let source =
                "= : :: ; , . -> => | ( ) { } [ ] < > <= >= == != + - * / % ! && || .. ... @ ? & %= += -= *= /= |= ^= <<= >>= &&= ||=";
            let tokens = lex(source, FileId::new(0));
            assert_eq!(tokens.len(), 46);
        }
    }