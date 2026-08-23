//! Abstract Syntax Tree (AST) for Aethel.

use crate::span::{FileId, Span, Spanned};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt;

/// A module is the top-level compilation unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub span: Span,
    pub items: Vec<Item>,
}

impl Spanned for Module {
    fn span(&self) -> Span {
        self.span
    }
}

/// Top-level items in a module.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Fn(FnDef),
    Struct(StructDef),
    Enum(EnumDef),
    TypeAlias(TypeAlias),
    Use(UseDecl),
    Mod(ModDecl),
    Policy(PolicyDef),
    Effect(EffectDef),
}

impl Spanned for Item {
    fn span(&self) -> Span {
        match self {
            Item::Fn(f) => f.span,
            Item::Struct(s) => s.span,
            Item::Enum(e) => e.span,
            Item::TypeAlias(t) => t.span,
            Item::Use(u) => u.span,
            Item::Mod(m) => m.span,
            Item::Policy(p) => p.span,
            Item::Effect(e) => e.span,
        }
    }
}

/// Function definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnDef {
    pub span: Span,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub ret_type: Option<Type>,
    pub effects: EffectSet,
    pub body: Option<Block>,
    pub is_pub: bool,
}

impl Spanned for FnDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// Generic type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericParam {
    pub span: Span,
    pub name: Ident,
    pub bounds: Vec<TypeBound>,
}

impl Spanned for GenericParam {
    fn span(&self) -> Span {
        self.span
    }
}

/// A trait/type bound on a generic parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeBound {
    pub span: Span,
    pub path: TypePath,
}

impl Spanned for TypeBound {
    fn span(&self) -> Span {
        self.span
    }
}

/// Function parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub span: Span,
    pub name: Ident,
    pub ty: Type,
    pub is_mut: bool,
}

impl Spanned for Param {
    fn span(&self) -> Span {
        self.span
    }
}

/// Effect set (uses clause).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectSet {
    pub span: Span,
    pub effects: Vec<EffectRef>,
}

impl Spanned for EffectSet {
    fn span(&self) -> Span {
        self.span
    }
}

impl Default for EffectSet {
    fn default() -> Self {
        Self {
            span: Span::single(FileId::new(0), crate::span::ByteOffset(0)),
            effects: Vec::new(),
        }
    }
}

/// An effect reference (e.g., `PaymentGateway`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectRef {
    pub span: Span,
    pub path: TypePath,
}

impl Spanned for EffectRef {
    fn span(&self) -> Span {
        self.span
    }
}

/// Struct definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDef {
    pub span: Span,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<StructField>,
    pub is_pub: bool,
}

impl Spanned for StructDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// Struct field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructField {
    pub span: Span,
    pub name: Ident,
    pub ty: Type,
    pub is_pub: bool,
}

impl Spanned for StructField {
    fn span(&self) -> Span {
        self.span
    }
}

/// Enum definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub span: Span,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<EnumVariant>,
    pub is_pub: bool,
}

impl Spanned for EnumDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// Enum variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumVariant {
    pub span: Span,
    pub name: Ident,
    pub fields: Vec<EnumField>,
}

impl Spanned for EnumVariant {
    fn span(&self) -> Span {
        self.span
    }
}

/// Enum variant field (tuple-style or struct-style).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnumField {
    Tuple { span: Span, ty: Type },
    Named { span: Span, name: Ident, ty: Type },
}

impl Spanned for EnumField {
    fn span(&self) -> Span {
        match self {
            EnumField::Tuple { span, .. } => *span,
            EnumField::Named { span, .. } => *span,
        }
    }
}

/// Type alias.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeAlias {
    pub span: Span,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub ty: Type,
    pub is_pub: bool,
}

impl Spanned for TypeAlias {
    fn span(&self) -> Span {
        self.span
    }
}

/// Use declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseDecl {
    pub span: Span,
    pub path: UsePath,
    pub is_pub: bool,
}

impl Spanned for UseDecl {
    fn span(&self) -> Span {
        self.span
    }
}

/// Use path (can be a tree).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UsePath {
    Simple {
        span: Span,
        path: TypePath,
    },
    Glob {
        span: Span,
        prefix: TypePath,
    },
    Group {
        span: Span,
        prefix: TypePath,
        items: Vec<UsePath>,
    },
}

impl Spanned for UsePath {
    fn span(&self) -> Span {
        match self {
            UsePath::Simple { span, .. } => *span,
            UsePath::Glob { span, .. } => *span,
            UsePath::Group { span, .. } => *span,
        }
    }
}

/// Module declaration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModDecl {
    pub span: Span,
    pub name: Ident,
    pub body: Option<Module>,
    pub is_pub: bool,
}

impl Spanned for ModDecl {
    fn span(&self) -> Span {
        self.span
    }
}

/// Policy definition (epistemic type).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyDef {
    pub span: Span,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub claims: Vec<PolicyClaim>,
    pub is_pub: bool,
}

impl Spanned for PolicyDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// Effect definition (external side-effect boundary with operations).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectDef {
    pub span: Span,
    pub name: Ident,
    pub operations: Vec<EffectOperation>,
    pub is_pub: bool,
}

impl Spanned for EffectDef {
    fn span(&self) -> Span {
        self.span
    }
}

/// An operation (function signature) exposed by an effect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectOperation {
    pub span: Span,
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret_type: Option<Type>,
}

impl Spanned for EffectOperation {
    fn span(&self) -> Span {
        self.span
    }
}

/// A claim in a policy (evidence requirement).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyClaim {
    pub span: Span,
    pub name: Ident,
    pub ty: Type,
    pub evidence: Vec<EvidenceReq>,
}

impl Spanned for PolicyClaim {
    fn span(&self) -> Span {
        self.span
    }
}

/// Evidence requirement for a claim.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceReq {
    pub span: Span,
    pub kind: EvidenceKind,
    pub description: String,
}

impl Spanned for EvidenceReq {
    fn span(&self) -> Span {
        self.span
    }
}

/// Kind of evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EvidenceKind {
    SignedAttestation,
    CryptographicProof,
    AuditLog,
    HumanReview,
    Custom(String),
}

/// Types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    /// Unit type `()`
    Unit { span: Span },
    /// Never type `!`
    Never { span: Span },
    /// Boolean `bool`
    Bool { span: Span },
    /// Integer `int`
    Int { span: Span },
    /// Float `float`
    Float { span: Span },
    /// String `string`
    String { span: Span },
    /// Identifier/type path
    Path { span: Span, path: TypePath },
    /// Reference `&T` or `&mut T`
    Ref {
        span: Span,
        is_mut: bool,
        ty: Box<Type>,
    },
    /// Linear/owned `T`
    Owned { span: Span, ty: Box<Type> },
    /// Claim `Claim<T>`
    Claim { span: Span, ty: Box<Type> },
    /// Verified `Verified<T, Policy>`
    Verified {
        span: Span,
        ty: Box<Type>,
        policy: Box<Type>,
    },
    /// Budget capability token — linear, consumed by `ask`
    Budget { span: Span },
    /// Array `[T; n]`
    Array {
        span: Span,
        ty: Box<Type>,
        size: Option<Box<Expr>>,
    },
    /// Tuple `(T1, T2, ...)`
    Tuple { span: Span, types: Vec<Type> },
    /// Function type `fn(T1, T2) -> R effects E`
    Fn {
        span: Span,
        params: Vec<Type>,
        ret: Box<Type>,
        effects: EffectSet,
    },
}

impl Spanned for Type {
    fn span(&self) -> Span {
        match self {
            Type::Unit { span } => *span,
            Type::Never { span } => *span,
            Type::Bool { span } => *span,
            Type::Int { span } => *span,
            Type::Float { span } => *span,
            Type::String { span } => *span,
            Type::Path { span, .. } => *span,
            Type::Ref { span, .. } => *span,
            Type::Owned { span, .. } => *span,
            Type::Claim { span, .. } => *span,
            Type::Verified { span, .. } => *span,
            Type::Budget { span } => *span,
            Type::Array { span, .. } => *span,
            Type::Tuple { span, .. } => *span,
            Type::Fn { span, .. } => *span,
        }
    }
}

/// Type path (qualified identifier).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypePath {
    pub span: Span,
    pub segments: Vec<PathSegment>,
}

impl Spanned for TypePath {
    fn span(&self) -> Span {
        self.span
    }
}

impl TypePath {
    pub fn single(span: Span, name: Ident) -> Self {
        Self {
            span,
            segments: vec![PathSegment {
                span: name.span,
                name,
                args: None,
            }],
        }
    }

    pub fn as_ident(&self) -> Option<Ident> {
        if self.segments.len() == 1 && self.segments[0].args.is_none() {
            Some(self.segments[0].name.clone())
        } else {
            None
        }
    }
}

/// Path segment with optional generic args.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathSegment {
    pub span: Span,
    pub name: Ident,
    pub args: Option<GenericArgs>,
}

impl Spanned for PathSegment {
    fn span(&self) -> Span {
        self.span
    }
}

/// Generic arguments for a path segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericArgs {
    pub span: Span,
    pub args: Vec<GenericArg>,
}

impl Spanned for GenericArgs {
    fn span(&self) -> Span {
        self.span
    }
}

/// A generic argument (type or const).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GenericArg {
    Type { span: Span, ty: Type },
    Const { span: Span, expr: Expr },
}

impl Spanned for GenericArg {
    fn span(&self) -> Span {
        match self {
            GenericArg::Type { span, .. } => *span,
            GenericArg::Const { span, .. } => *span,
        }
    }
}

/// Identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ident {
    pub span: Span,
    pub name: String,
}

impl Spanned for Ident {
    fn span(&self) -> Span {
        self.span
    }
}

impl Ident {
    pub fn new(span: Span, name: impl Into<String>) -> Self {
        Self {
            span,
            name: name.into(),
        }
    }

    pub fn dummy(name: impl Into<String>) -> Self {
        Self {
            span: Span::single(FileId::new(0), crate::span::ByteOffset(0)),
            name: name.into(),
        }
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Statements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Let {
        span: Span,
        name: Ident,
        ty: Option<Type>,
        is_mut: bool,
        init: Option<Expr>,
    },
    Expr {
        span: Span,
        expr: Expr,
    },
    Return {
        span: Span,
        expr: Option<Expr>,
    },
    If {
        span: Span,
        cond: Expr,
        then_branch: Block,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        span: Span,
        cond: Expr,
        body: Block,
    },
    For {
        span: Span,
        pat: Pat,
        iter: Expr,
        body: Block,
    },
    Match {
        span: Span,
        scrutinee: Expr,
        arms: Vec<MatchArm>,
    },
    Block {
        span: Span,
        block: Block,
    },
    Use {
        span: Span,
        decl: UseDecl,
    },
}

impl Spanned for Stmt {
    fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. } => *span,
            Stmt::Expr { span, .. } => *span,
            Stmt::Return { span, .. } => *span,
            Stmt::If { span, .. } => *span,
            Stmt::While { span, .. } => *span,
            Stmt::For { span, .. } => *span,
            Stmt::Match { span, .. } => *span,
            Stmt::Block { span, .. } => *span,
            Stmt::Use { span, .. } => *span,
        }
    }
}

/// Block of statements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub span: Span,
    pub stmts: Vec<Stmt>,
    pub tail: Option<Box<Expr>>,
}

impl Spanned for Block {
    fn span(&self) -> Span {
        self.span
    }
}

impl Default for Block {
    fn default() -> Self {
        Self {
            span: Span::single(FileId::new(0), crate::span::ByteOffset(0)),
            stmts: Vec::new(),
            tail: None,
        }
    }
}

/// Patterns for matching.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pat {
    Wild {
        span: Span,
    },
    Ident {
        span: Span,
        name: Ident,
        is_mut: bool,
    },
    Literal {
        span: Span,
        lit: Literal,
    },
    Tuple {
        span: Span,
        pats: Vec<Pat>,
    },
    Struct {
        span: Span,
        path: TypePath,
        fields: Vec<PatField>,
    },
    Enum {
        span: Span,
        path: TypePath,
        fields: Vec<Pat>,
    },
    Or {
        span: Span,
        pats: Vec<Pat>,
    },
    Ref {
        span: Span,
        is_mut: bool,
        pat: Box<Pat>,
    },
}

impl Spanned for Pat {
    fn span(&self) -> Span {
        match self {
            Pat::Wild { span } => *span,
            Pat::Ident { span, .. } => *span,
            Pat::Literal { span, .. } => *span,
            Pat::Tuple { span, .. } => *span,
            Pat::Struct { span, .. } => *span,
            Pat::Enum { span, .. } => *span,
            Pat::Or { span, .. } => *span,
            Pat::Ref { span, .. } => *span,
        }
    }
}

/// Struct pattern field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatField {
    pub span: Span,
    pub name: Ident,
    pub pat: Option<Pat>,
}

impl Spanned for PatField {
    fn span(&self) -> Span {
        self.span
    }
}

/// Match arm.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub span: Span,
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Expr,
}

impl Spanned for MatchArm {
    fn span(&self) -> Span {
        self.span
    }
}

/// Expressions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Literal {
        span: Span,
        lit: Literal,
    },
    Path {
        span: Span,
        path: ExprPath,
    },
    Tuple {
        span: Span,
        exprs: Vec<Expr>,
    },
    Array {
        span: Span,
        exprs: Vec<Expr>,
    },
    Struct {
        span: Span,
        path: TypePath,
        fields: Vec<StructExprField>,
        base: Option<Box<Expr>>,
    },
    Call {
        span: Span,
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    MethodCall {
        span: Span,
        receiver: Box<Expr>,
        method: Ident,
        args: Vec<Expr>,
    },
    Field {
        span: Span,
        base: Box<Expr>,
        field: Ident,
    },
    Index {
        span: Span,
        base: Box<Expr>,
        index: Box<Expr>,
    },
    Unary {
        span: Span,
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        span: Span,
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    If {
        span: Span,
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    Match {
        span: Span,
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Block {
        span: Span,
        block: Block,
    },
    Let {
        span: Span,
        pat: Pat,
        ty: Option<Type>,
        is_mut: bool,
        init: Box<Expr>,
    },
    Return {
        span: Span,
        expr: Option<Box<Expr>>,
    },
    Break {
        span: Span,
        expr: Option<Box<Expr>>,
    },
    Continue {
        span: Span,
    },
    Ask {
        span: Span,
        model: ExprPath,
        goal: String,
        input: Box<Expr>,
        output_ty: Type,
    },
    Verify {
        span: Span,
        claim: Box<Expr>,
        policy: TypePath,
        /// Optional `evidence Kind` third argument. `None` when the caller
        /// provides no evidence; `Some` only via the `evidence` keyword form
        /// `verify(c, Policy, evidence SignedAttestation)`.
        evidence: Option<EvidenceKind>,
    },
    Reason {
        span: Span,
        prompt: String,
    },
    CommitOnce {
        span: Span,
        effect: EffectRef,
        args: Vec<Expr>,
    },
    New {
        span: Span,
        ty: Type,
        args: Vec<Expr>,
    },
}

impl Spanned for Expr {
    fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. } => *span,
            Expr::Path { span, .. } => *span,
            Expr::Tuple { span, .. } => *span,
            Expr::Array { span, .. } => *span,
            Expr::Struct { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::MethodCall { span, .. } => *span,
            Expr::Field { span, .. } => *span,
            Expr::Index { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::If { span, .. } => *span,
            Expr::Match { span, .. } => *span,
            Expr::Block { span, .. } => *span,
            Expr::Let { span, .. } => *span,
            Expr::Return { span, .. } => *span,
            Expr::Break { span, .. } => *span,
            Expr::Continue { span } => *span,
            Expr::Ask { span, .. } => *span,
            Expr::Verify { span, .. } => *span,
            Expr::Reason { span, .. } => *span,
            Expr::CommitOnce { span, .. } => *span,
            Expr::New { span, .. } => *span,
        }
    }
}

/// Expression path (for calls, method calls, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExprPath {
    pub span: Span,
    pub segments: Vec<PathSegment>,
}

impl Spanned for ExprPath {
    fn span(&self) -> Span {
        self.span
    }
}

/// Struct expression field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructExprField {
    pub span: Span,
    pub name: Ident,
    pub expr: Expr,
}

impl Spanned for StructExprField {
    fn span(&self) -> Span {
        self.span
    }
}

/// Literals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Unit { span: Span },
    Bool { span: Span, value: bool },
    Int { span: Span, value: i64 },
    Float { span: Span, value: f64 },
    String { span: Span, value: String },
}

impl Spanned for Literal {
    fn span(&self) -> Span {
        match self {
            Literal::Unit { span } => *span,
            Literal::Bool { span, .. } => *span,
            Literal::Int { span, .. } => *span,
            Literal::Float { span, .. } => *span,
            Literal::String { span, .. } => *span,
        }
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
    Deref,
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnaryOp::Neg => write!(f, "-"),
            UnaryOp::Not => write!(f, "!"),
            UnaryOp::Deref => write!(f, "*"),
        }
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Rem => write!(f, "%"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Ne => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Ge => write!(f, ">="),
            BinaryOp::And => write!(f, "&&"),
            BinaryOp::Or => write!(f, "||"),
            BinaryOp::Assign => write!(f, "="),
            BinaryOp::AddAssign => write!(f, "+="),
            BinaryOp::SubAssign => write!(f, "-="),
            BinaryOp::MulAssign => write!(f, "*="),
            BinaryOp::DivAssign => write!(f, "/="),
            BinaryOp::RemAssign => write!(f, "%="),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::{ByteOffset, FileId, Span};

    #[test]
    fn test_ast_node_spans() {
        let span = Span::new(FileId::new(0), ByteOffset(0), ByteOffset(10));
        let ident = Ident::new(span, "test");
        assert_eq!(ident.span(), span);
        assert_eq!(ident.name, "test");
    }
}
