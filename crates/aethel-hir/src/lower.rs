//! HIR lowering from AST.

use aethel_syntax::ast::*;
use aethel_syntax::span::{FileId, Span};

/// Result of lowering an AST to HIR.
#[derive(Debug, Clone)]
pub struct HirModule {
    pub file_id: FileId,
    pub items: Vec<HirItem>,
}

/// HIR items after name resolution.
#[derive(Debug, Clone)]
pub enum HirItem {
    Fn(HirFnDef),
    Struct(HirStructDef),
    Enum(HirEnumDef),
    TypeAlias(HirTypeAlias),
    Use(HirUseDecl),
    Mod(HirModDecl),
    Policy(HirPolicyDef),
    Effect(HirEffectDef),
}

/// Lowered function definition.
#[derive(Debug, Clone)]
pub struct HirFnDef {
    pub span: Span,
    pub name: String,
    pub generics: Vec<HirGenericParam>,
    pub params: Vec<HirParam>,
    pub ret_type: Option<HirType>,
    pub effects: HirEffectSet,
    pub body: Option<HirBlock>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct HirGenericParam {
    pub span: Span,
    pub name: String,
    pub bounds: Vec<HirTypeBound>,
}

#[derive(Debug, Clone)]
pub struct HirTypeBound {
    pub span: Span,
    pub path: HirTypePath,
}

#[derive(Debug, Clone)]
pub struct HirParam {
    pub span: Span,
    pub name: String,
    pub ty: HirType,
    pub is_mut: bool,
}

#[derive(Debug, Clone)]
pub struct HirEffectSet {
    pub span: Span,
    pub effects: Vec<HirEffectRef>,
}

#[derive(Debug, Clone)]
pub struct HirEffectRef {
    pub span: Span,
    pub path: HirTypePath,
}

#[derive(Debug, Clone)]
pub struct HirBlock {
    pub span: Span,
    pub stmts: Vec<HirStmt>,
    pub tail: Option<Box<HirExpr>>,
}

#[derive(Debug, Clone)]
pub enum HirStmt {
    Let {
        span: Span,
        name: String,
        ty: Option<HirType>,
        is_mut: bool,
        init: Option<HirExpr>,
    },
    Expr {
        span: Span,
        expr: HirExpr,
    },
    Return {
        span: Span,
        expr: Option<HirExpr>,
    },
    If {
        span: Span,
        cond: HirExpr,
        then_branch: HirBlock,
        else_branch: Option<Box<HirStmt>>,
    },
    While {
        span: Span,
        cond: HirExpr,
        body: HirBlock,
    },
    For {
        span: Span,
        pat: HirPat,
        iter: HirExpr,
        body: HirBlock,
    },
    Match {
        span: Span,
        scrutinee: HirExpr,
        arms: Vec<HirMatchArm>,
    },
    Block {
        span: Span,
        block: HirBlock,
    },
}

#[derive(Debug, Clone)]
pub enum HirExpr {
    Literal { span: Span, lit: HirLiteral },
    Path { span: Span, path: HirExprPath },
    Tuple { span: Span, exprs: Vec<HirExpr> },
    Array { span: Span, exprs: Vec<HirExpr> },
    Struct {
        span: Span,
        path: HirTypePath,
        fields: Vec<HirStructExprField>,
        base: Option<Box<HirExpr>>,
    },
    Call { span: Span, callee: Box<HirExpr>, args: Vec<HirExpr> },
    MethodCall {
        span: Span,
        receiver: Box<HirExpr>,
        method: String,
        args: Vec<HirExpr>,
    },
    Field { span: Span, base: Box<HirExpr>, field: String },
    Index { span: Span, base: Box<HirExpr>, index: Box<HirExpr> },
    Unary { span: Span, op: HirUnaryOp, expr: Box<HirExpr> },
    Binary { span: Span, op: HirBinaryOp, left: Box<HirExpr>, right: Box<HirExpr> },
    If {
        span: Span,
        cond: Box<HirExpr>,
        then_branch: Box<HirExpr>,
        else_branch: Option<Box<HirExpr>>,
    },
    Match {
        span: Span,
        scrutinee: Box<HirExpr>,
        arms: Vec<HirMatchArm>,
    },
    Block { span: Span, block: HirBlock },
    Let {
        span: Span,
        pat: HirPat,
        ty: Option<HirType>,
        is_mut: bool,
        init: Box<HirExpr>,
    },
    Return { span: Span, expr: Option<Box<HirExpr>> },
    Break { span: Span, expr: Option<Box<HirExpr>> },
    Continue { span: Span },
    Ask {
        span: Span,
        model: HirExprPath,
        goal: String,
        input: Box<HirExpr>,
        output_ty: HirType,
    },
    Verify {
        span: Span,
        claim: Box<HirExpr>,
        policy: HirTypePath,
    },
    Reason {
        span: Span,
        prompt: String,
    },
    CommitOnce {
        span: Span,
        effect: HirEffectRef,
        args: Vec<HirExpr>,
    },
    New { span: Span, ty: HirType, args: Vec<HirExpr> },
}

#[derive(Debug, Clone)]
pub struct HirMatchArm {
    pub span: Span,
    pub pat: HirPat,
    pub guard: Option<HirExpr>,
    pub body: HirExpr,
}

#[derive(Debug, Clone)]
pub enum HirPat {
    Wild { span: Span },
    Ident { span: Span, name: String, is_mut: bool },
    Literal { span: Span, lit: HirLiteral },
    Tuple { span: Span, pats: Vec<HirPat> },
    Struct { span: Span, path: HirTypePath, fields: Vec<HirPatField> },
    Enum { span: Span, path: HirTypePath, fields: Vec<HirPat> },
    Or { span: Span, pats: Vec<HirPat> },
    Ref { span: Span, is_mut: bool, pat: Box<HirPat> },
}

#[derive(Debug, Clone)]
pub struct HirPatField {
    pub span: Span,
    pub name: String,
    pub pat: Option<HirPat>,
}

#[derive(Debug, Clone)]
pub enum HirLiteral {
    Unit { span: Span },
    Bool { span: Span, value: bool },
    Int { span: Span, value: i64 },
    Float { span: Span, value: f64 },
    String { span: Span, value: String },
}

#[derive(Debug, Clone)]
pub enum HirUnaryOp {
    Neg,
    Not,
    Deref,
}

#[derive(Debug, Clone)]
pub enum HirBinaryOp {
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

#[derive(Debug, Clone)]
pub struct HirExprPath {
    pub span: Span,
    pub segments: Vec<HirPathSegment>,
}

#[derive(Debug, Clone)]
pub struct HirPathSegment {
    pub span: Span,
    pub name: String,
    pub args: Option<HirGenericArgs>,
}

#[derive(Debug, Clone)]
pub struct HirGenericArgs {
    pub span: Span,
    pub args: Vec<HirGenericArg>,
}

#[derive(Debug, Clone)]
pub enum HirGenericArg {
    Type { span: Span, ty: HirType },
    Const { span: Span, expr: HirExpr },
}

#[derive(Debug, Clone)]
pub struct HirStructExprField {
    pub span: Span,
    pub name: String,
    pub expr: HirExpr,
}

#[derive(Debug, Clone)]
pub struct HirTypePath {
    pub span: Span,
    pub segments: Vec<HirPathSegment>,
}

impl HirTypePath {
    pub fn as_ident(&self) -> Option<String> {
        if self.segments.len() == 1 && self.segments[0].args.is_none() {
            Some(self.segments[0].name.clone())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub enum HirType {
    Unit { span: Span },
    Never { span: Span },
    Bool { span: Span },
    Int { span: Span },
    Float { span: Span },
    String { span: Span },
    Path { span: Span, path: HirTypePath },
    Ref { span: Span, is_mut: bool, ty: Box<HirType> },
    Owned { span: Span, ty: Box<HirType> },
    Claim { span: Span, ty: Box<HirType> },
    Verified { span: Span, ty: Box<HirType>, policy: Box<HirType> },
    Array { span: Span, ty: Box<HirType>, size: Option<Box<HirExpr>> },
    Tuple { span: Span, types: Vec<HirType> },
    Fn {
        span: Span,
        params: Vec<HirType>,
        ret: Box<HirType>,
        effects: HirEffectSet,
    },
}

#[derive(Debug, Clone)]
pub struct HirStructDef {
    pub span: Span,
    pub name: String,
    pub generics: Vec<HirGenericParam>,
    pub fields: Vec<HirStructField>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct HirStructField {
    pub span: Span,
    pub name: String,
    pub ty: HirType,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct HirEnumDef {
    pub span: Span,
    pub name: String,
    pub generics: Vec<HirGenericParam>,
    pub variants: Vec<HirEnumVariant>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct HirEnumVariant {
    pub span: Span,
    pub name: String,
    pub fields: Vec<HirEnumField>,
}

#[derive(Debug, Clone)]
pub enum HirEnumField {
    Tuple { span: Span, ty: HirType },
    Named { span: Span, name: String, ty: HirType },
}

#[derive(Debug, Clone)]
pub struct HirTypeAlias {
    pub span: Span,
    pub name: String,
    pub generics: Vec<HirGenericParam>,
    pub ty: HirType,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct HirUseDecl {
    pub span: Span,
    pub path: HirUsePath,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub enum HirUsePath {
    Simple { span: Span, path: HirTypePath },
    Glob { span: Span, prefix: HirTypePath },
    Group { span: Span, prefix: HirTypePath, items: Vec<HirUsePath> },
}

#[derive(Debug, Clone)]
pub struct HirModDecl {
    pub span: Span,
    pub name: String,
    pub body: Option<HirModule>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct HirPolicyDef {
    pub span: Span,
    pub name: String,
    pub generics: Vec<HirGenericParam>,
    pub claims: Vec<HirPolicyClaim>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct HirEffectDef {
    pub span: Span,
    pub name: String,
    pub operations: Vec<HirEffectOperation>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct HirEffectOperation {
    pub span: Span,
    pub name: String,
    pub params: Vec<HirParam>,
    pub ret_type: Option<HirType>,
}

#[derive(Debug, Clone)]
pub struct HirPolicyClaim {
    pub span: Span,
    pub name: String,
    pub ty: HirType,
    pub evidence: Vec<HirEvidenceReq>,
}

#[derive(Debug, Clone)]
pub struct HirEvidenceReq {
    pub span: Span,
    pub kind: HirEvidenceKind,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum HirEvidenceKind {
    SignedAttestation,
    CryptographicProof,
    AuditLog,
    HumanReview,
    Custom(String),
}