//! IR lowering from HIR.

use aethel_hir::lower::*;
use aethel_syntax::span::{FileId, Span};

/// IR module after type checking.
#[derive(Debug, Clone)]
pub struct IrModule {
    pub file_id: FileId,
    pub items: Vec<IrItem>,
}

/// IR items.
#[derive(Debug, Clone)]
pub enum IrItem {
    Fn(IrFnDef),
    Struct(IrStructDef),
    Enum(IrEnumDef),
    TypeAlias(IrTypeAlias),
    Use(IrUseDecl),
    Mod(IrModDecl),
    Policy(IrPolicyDef),
}

/// IR function with fully resolved types.
#[derive(Debug, Clone)]
pub struct IrFnDef {
    pub span: Span,
    pub name: String,
    pub generics: Vec<IrGenericParam>,
    pub params: Vec<IrParam>,
    pub ret_type: IrType,
    pub effects: IrEffectSet,
    pub body: Option<IrBlock>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IrGenericParam {
    pub span: Span,
    pub name: String,
    pub bounds: Vec<IrTypeBound>,
}

#[derive(Debug, Clone)]
pub struct IrTypeBound {
    pub span: Span,
    pub path: IrTypePath,
}

#[derive(Debug, Clone)]
pub struct IrParam {
    pub span: Span,
    pub name: String,
    pub ty: IrType,
    pub is_mut: bool,
}

#[derive(Debug, Clone)]
pub struct IrEffectSet {
    pub span: Span,
    pub effects: Vec<IrEffectRef>,
}

#[derive(Debug, Clone)]
pub struct IrEffectRef {
    pub span: Span,
    pub path: IrTypePath,
}

#[derive(Debug, Clone)]
pub struct IrBlock {
    pub span: Span,
    pub stmts: Vec<IrStmt>,
    pub tail: Option<Box<IrExpr>>,
}

#[derive(Debug, Clone)]
pub enum IrStmt {
    Let {
        span: Span,
        name: String,
        ty: IrType,
        is_mut: bool,
        init: Option<IrExpr>,
    },
    Expr {
        span: Span,
        expr: IrExpr,
    },
    Return {
        span: Span,
        expr: Option<IrExpr>,
    },
    If {
        span: Span,
        cond: IrExpr,
        then_branch: IrBlock,
        else_branch: Option<Box<IrStmt>>,
    },
    While {
        span: Span,
        cond: IrExpr,
        body: IrBlock,
    },
    For {
        span: Span,
        pat: IrPat,
        iter: IrExpr,
        body: IrBlock,
    },
    Match {
        span: Span,
        scrutinee: IrExpr,
        arms: Vec<IrMatchArm>,
    },
    Block {
        span: Span,
        block: IrBlock,
    },
}

#[derive(Debug, Clone)]
pub enum IrExpr {
    Literal { span: Span, lit: IrLiteral },
    Path { span: Span, path: IrExprPath },
    Tuple { span: Span, exprs: Vec<IrExpr> },
    Array { span: Span, exprs: Vec<IrExpr> },
    Struct {
        span: Span,
        path: IrTypePath,
        fields: Vec<IrStructExprField>,
        base: Option<Box<IrExpr>>,
    },
    Call { span: Span, callee: Box<IrExpr>, args: Vec<IrExpr> },
    MethodCall {
        span: Span,
        receiver: Box<IrExpr>,
        method: String,
        args: Vec<IrExpr>,
    },
    Field { span: Span, base: Box<IrExpr>, field: String },
    Index { span: Span, base: Box<IrExpr>, index: Box<IrExpr> },
    Unary { span: Span, op: IrUnaryOp, expr: Box<IrExpr> },
    Binary { span: Span, op: IrBinaryOp, left: Box<IrExpr>, right: Box<IrExpr> },
    If {
        span: Span,
        cond: Box<IrExpr>,
        then_branch: Box<IrExpr>,
        else_branch: Option<Box<IrExpr>>,
    },
    Match {
        span: Span,
        scrutinee: Box<IrExpr>,
        arms: Vec<IrMatchArm>,
    },
    Block { span: Span, block: IrBlock },
    Let {
        span: Span,
        pat: IrPat,
        ty: IrType,
        is_mut: bool,
        init: Box<IrExpr>,
    },
    Return { span: Span, expr: Option<Box<IrExpr>> },
    Break { span: Span, expr: Option<Box<IrExpr>> },
    Continue { span: Span },
    Ask {
        span: Span,
        model: IrExprPath,
        goal: String,
        input: Box<IrExpr>,
        output_ty: IrType,
    },
    Verify {
        span: Span,
        claim: Box<IrExpr>,
        policy: IrTypePath,
    },
    CommitOnce {
        span: Span,
        effect: IrEffectRef,
        args: Vec<IrExpr>,
    },
    New { span: Span, ty: IrType, args: Vec<IrExpr> },
}

#[derive(Debug, Clone)]
pub struct IrMatchArm {
    pub span: Span,
    pub pat: IrPat,
    pub guard: Option<IrExpr>,
    pub body: IrExpr,
}

#[derive(Debug, Clone)]
pub enum IrPat {
    Wild { span: Span },
    Ident { span: Span, name: String, is_mut: bool },
    Literal { span: Span, lit: IrLiteral },
    Tuple { span: Span, pats: Vec<IrPat> },
    Struct { span: Span, path: IrTypePath, fields: Vec<IrPatField> },
    Enum { span: Span, path: IrTypePath, fields: Vec<IrPat> },
    Or { span: Span, pats: Vec<IrPat> },
    Ref { span: Span, is_mut: bool, pat: Box<IrPat> },
}

#[derive(Debug, Clone)]
pub struct IrPatField {
    pub span: Span,
    pub name: String,
    pub pat: Option<IrPat>,
}

#[derive(Debug, Clone)]
pub enum IrLiteral {
    Unit { span: Span },
    Bool { span: Span, value: bool },
    Int { span: Span, value: i64 },
    Float { span: Span, value: f64 },
    String { span: Span, value: String },
}

#[derive(Debug, Clone)]
pub enum IrUnaryOp {
    Neg,
    Not,
    Deref,
}

#[derive(Debug, Clone)]
pub enum IrBinaryOp {
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
pub struct IrExprPath {
    pub span: Span,
    pub segments: Vec<IrPathSegment>,
}

#[derive(Debug, Clone)]
pub struct IrPathSegment {
    pub span: Span,
    pub name: String,
    pub args: Option<IrGenericArgs>,
}

#[derive(Debug, Clone)]
pub struct IrGenericArgs {
    pub span: Span,
    pub args: Vec<IrGenericArg>,
}

#[derive(Debug, Clone)]
pub enum IrGenericArg {
    Type { span: Span, ty: IrType },
    Const { span: Span, expr: IrExpr },
}

#[derive(Debug, Clone)]
pub struct IrStructExprField {
    pub span: Span,
    pub name: String,
    pub expr: IrExpr,
}

#[derive(Debug, Clone)]
pub struct IrTypePath {
    pub span: Span,
    pub segments: Vec<IrPathSegment>,
}

impl IrTypePath {
    pub fn as_ident(&self) -> Option<String> {
        if self.segments.len() == 1 && self.segments[0].args.is_none() {
            Some(self.segments[0].name.clone())
        } else {
            None
        }
    }

    pub fn single(name: &str) -> Self {
        IrTypePath {
            span: Span::zero(),
            segments: vec![IrPathSegment {
                span: Span::zero(),
                name: name.to_string(),
                args: None,
            }],
        }
    }
}

#[derive(Debug, Clone)]
pub enum IrType {
    Unit { span: Span },
    Never { span: Span },
    Bool { span: Span },
    Int { span: Span },
    Float { span: Span },
    String { span: Span },
    Path { span: Span, path: IrTypePath },
    Ref { span: Span, is_mut: bool, ty: Box<IrType> },
    Owned { span: Span, ty: Box<IrType> },
    Claim { span: Span, ty: Box<IrType> },
    Verified { span: Span, ty: Box<IrType>, policy: Box<IrType> },
    Array { span: Span, ty: Box<IrType>, size: Option<Box<IrExpr>> },
    Tuple { span: Span, types: Vec<IrType> },
    Fn {
        span: Span,
        params: Vec<IrType>,
        ret: Box<IrType>,
        effects: IrEffectSet,
    },
}

#[derive(Debug, Clone)]
pub struct IrStructDef {
    pub span: Span,
    pub name: String,
    pub generics: Vec<IrGenericParam>,
    pub fields: Vec<IrStructField>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IrStructField {
    pub span: Span,
    pub name: String,
    pub ty: IrType,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IrEnumDef {
    pub span: Span,
    pub name: String,
    pub generics: Vec<IrGenericParam>,
    pub variants: Vec<IrEnumVariant>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IrEnumVariant {
    pub span: Span,
    pub name: String,
    pub fields: Vec<IrEnumField>,
}

#[derive(Debug, Clone)]
pub enum IrEnumField {
    Tuple { span: Span, ty: IrType },
    Named { span: Span, name: String, ty: IrType },
}

#[derive(Debug, Clone)]
pub struct IrTypeAlias {
    pub span: Span,
    pub name: String,
    pub generics: Vec<IrGenericParam>,
    pub ty: IrType,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IrUseDecl {
    pub span: Span,
    pub path: IrUsePath,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub enum IrUsePath {
    Simple { span: Span, path: IrTypePath },
    Glob { span: Span, prefix: IrTypePath },
    Group { span: Span, prefix: IrTypePath, items: Vec<IrUsePath> },
}

#[derive(Debug, Clone)]
pub struct IrModDecl {
    pub span: Span,
    pub name: String,
    pub body: Option<IrModule>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IrPolicyDef {
    pub span: Span,
    pub name: String,
    pub generics: Vec<IrGenericParam>,
    pub claims: Vec<IrPolicyClaim>,
    pub is_pub: bool,
}

#[derive(Debug, Clone)]
pub struct IrPolicyClaim {
    pub span: Span,
    pub name: String,
    pub ty: IrType,
    pub evidence: Vec<IrEvidenceReq>,
}

#[derive(Debug, Clone)]
pub struct IrEvidenceReq {
    pub span: Span,
    pub kind: IrEvidenceKind,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum IrEvidenceKind {
    SignedAttestation,
    CryptographicProof,
    AuditLog,
    HumanReview,
    Custom(String),
}