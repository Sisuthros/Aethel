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

// ─────────────────────────────────────────────────────────────────────────────
// AST → HIR lowering
// ─────────────────────────────────────────────────────────────────────────────
//
// `lower_module` is the entrypoint that walks an `aethel_syntax::ast::Module`
// and produces a fully-desugared `HirModule` whose `Ident` names have been
// converted to plain `String`s and whose node types are those consumed by the
// HIR-level `check_module` and `resolve_module` passes.
//
// This pass does *not* run name resolution, type checking, or effect
// inference — it is a structural, lossless (modulo syntactic sugar) lowering.

/// Lower a parsed AST module into a HIR module.
pub fn lower_module(module: &aethel_syntax::ast::Module, file_id: FileId) -> HirModule {
    HirModule {
        file_id,
        items: module.items.iter().map(lower_item).collect(),
    }
}

// ── Items ────────────────────────────────────────────────────────────────────

fn lower_item(item: &Item) -> HirItem {
    match item {
        Item::Fn(f) => HirItem::Fn(lower_fn(f)),
        Item::Struct(s) => HirItem::Struct(lower_struct(s)),
        Item::Enum(e) => HirItem::Enum(lower_enum(e)),
        Item::TypeAlias(t) => HirItem::TypeAlias(lower_type_alias(t)),
        Item::Use(u) => HirItem::Use(lower_use(u)),
        Item::Mod(m) => HirItem::Mod(lower_mod(m)),
        Item::Policy(p) => HirItem::Policy(lower_policy(p)),
        Item::Effect(e) => HirItem::Effect(lower_effect(e)),
    }
}

fn lower_fn(f: &FnDef) -> HirFnDef {
    HirFnDef {
        span: f.span,
        name: f.name.name.clone(),
        generics: f.generics.iter().map(lower_generic_param).collect(),
        params: f.params.iter().map(lower_param).collect(),
        ret_type: f.ret_type.as_ref().map(lower_type),
        effects: lower_effect_set(&f.effects),
        body: f.body.as_ref().map(lower_block),
        is_pub: f.is_pub,
    }
}

fn lower_generic_param(p: &GenericParam) -> HirGenericParam {
    HirGenericParam {
        span: p.span,
        name: p.name.name.clone(),
        bounds: p.bounds.iter().map(lower_type_bound).collect(),
    }
}

fn lower_type_bound(b: &TypeBound) -> HirTypeBound {
    HirTypeBound {
        span: b.span,
        path: lower_type_path(&b.path),
    }
}

fn lower_param(p: &Param) -> HirParam {
    HirParam {
        span: p.span,
        name: p.name.name.clone(),
        ty: lower_type(&p.ty),
        is_mut: p.is_mut,
    }
}

fn lower_effect_set(es: &EffectSet) -> HirEffectSet {
    HirEffectSet {
        span: es.span,
        effects: es.effects.iter().map(lower_effect_ref).collect(),
    }
}

fn lower_effect_ref(e: &EffectRef) -> HirEffectRef {
    HirEffectRef {
        span: e.span,
        path: lower_type_path(&e.path),
    }
}

fn lower_struct(s: &StructDef) -> HirStructDef {
    HirStructDef {
        span: s.span,
        name: s.name.name.clone(),
        generics: s.generics.iter().map(lower_generic_param).collect(),
        fields: s.fields.iter().map(lower_struct_field).collect(),
        is_pub: s.is_pub,
    }
}

fn lower_struct_field(f: &StructField) -> HirStructField {
    HirStructField {
        span: f.span,
        name: f.name.name.clone(),
        ty: lower_type(&f.ty),
        is_pub: f.is_pub,
    }
}

fn lower_enum(e: &EnumDef) -> HirEnumDef {
    HirEnumDef {
        span: e.span,
        name: e.name.name.clone(),
        generics: e.generics.iter().map(lower_generic_param).collect(),
        variants: e.variants.iter().map(lower_enum_variant).collect(),
        is_pub: e.is_pub,
    }
}

fn lower_enum_variant(v: &EnumVariant) -> HirEnumVariant {
    HirEnumVariant {
        span: v.span,
        name: v.name.name.clone(),
        fields: v.fields.iter().map(lower_enum_field).collect(),
    }
}

fn lower_enum_field(f: &EnumField) -> HirEnumField {
    match f {
        EnumField::Tuple { span, ty } => HirEnumField::Tuple {
            span: *span,
            ty: lower_type(ty),
        },
        EnumField::Named { span, name, ty } => HirEnumField::Named {
            span: *span,
            name: name.name.clone(),
            ty: lower_type(ty),
        },
    }
}

fn lower_type_alias(t: &TypeAlias) -> HirTypeAlias {
    HirTypeAlias {
        span: t.span,
        name: t.name.name.clone(),
        generics: t.generics.iter().map(lower_generic_param).collect(),
        ty: lower_type(&t.ty),
        is_pub: t.is_pub,
    }
}

fn lower_use(u: &UseDecl) -> HirUseDecl {
    HirUseDecl {
        span: u.span,
        path: lower_use_path(&u.path),
        is_pub: u.is_pub,
    }
}

fn lower_use_path(p: &UsePath) -> HirUsePath {
    match p {
        UsePath::Simple { span, path } => HirUsePath::Simple {
            span: *span,
            path: lower_type_path(path),
        },
        UsePath::Glob { span, prefix } => HirUsePath::Glob {
            span: *span,
            prefix: lower_type_path(prefix),
        },
        UsePath::Group { span, prefix, items } => HirUsePath::Group {
            span: *span,
            prefix: lower_type_path(prefix),
            items: items.iter().map(lower_use_path).collect(),
        },
    }
}

fn lower_mod(m: &ModDecl) -> HirModDecl {
    HirModDecl {
        span: m.span,
        name: m.name.name.clone(),
        body: m
            .body
            .as_ref()
            .map(|nested| lower_module(nested, FileId::new(0))),
        is_pub: m.is_pub,
    }
}

fn lower_policy(p: &PolicyDef) -> HirPolicyDef {
    HirPolicyDef {
        span: p.span,
        name: p.name.name.clone(),
        generics: p.generics.iter().map(lower_generic_param).collect(),
        claims: p.claims.iter().map(lower_policy_claim).collect(),
        is_pub: p.is_pub,
    }
}

fn lower_policy_claim(c: &PolicyClaim) -> HirPolicyClaim {
    HirPolicyClaim {
        span: c.span,
        name: c.name.name.clone(),
        ty: lower_type(&c.ty),
        evidence: c.evidence.iter().map(lower_evidence_req).collect(),
    }
}

fn lower_evidence_req(e: &EvidenceReq) -> HirEvidenceReq {
    HirEvidenceReq {
        span: e.span,
        kind: lower_evidence_kind(&e.kind),
        description: e.description.clone(),
    }
}

fn lower_evidence_kind(k: &EvidenceKind) -> HirEvidenceKind {
    match k {
        EvidenceKind::SignedAttestation => HirEvidenceKind::SignedAttestation,
        EvidenceKind::CryptographicProof => HirEvidenceKind::CryptographicProof,
        EvidenceKind::AuditLog => HirEvidenceKind::AuditLog,
        EvidenceKind::HumanReview => HirEvidenceKind::HumanReview,
        EvidenceKind::Custom(s) => HirEvidenceKind::Custom(s.clone()),
    }
}

fn lower_effect(e: &EffectDef) -> HirEffectDef {
    HirEffectDef {
        span: e.span,
        name: e.name.name.clone(),
        operations: e.operations.iter().map(lower_effect_operation).collect(),
        is_pub: e.is_pub,
    }
}

fn lower_effect_operation(o: &EffectOperation) -> HirEffectOperation {
    HirEffectOperation {
        span: o.span,
        name: o.name.name.clone(),
        params: o.params.iter().map(lower_param).collect(),
        ret_type: o.ret_type.as_ref().map(lower_type),
    }
}

// ── Types ────────────────────────────────────────────────────────────────────

fn lower_type(ty: &Type) -> HirType {
    match ty {
        Type::Unit { span } => HirType::Unit { span: *span },
        Type::Never { span } => HirType::Never { span: *span },
        Type::Bool { span } => HirType::Bool { span: *span },
        Type::Int { span } => HirType::Int { span: *span },
        Type::Float { span } => HirType::Float { span: *span },
        Type::String { span } => HirType::String { span: *span },
        Type::Path { span, path } => HirType::Path {
            span: *span,
            path: lower_type_path(path),
        },
        Type::Ref { span, is_mut, ty } => HirType::Ref {
            span: *span,
            is_mut: *is_mut,
            ty: Box::new(lower_type(ty)),
        },
        Type::Owned { span, ty } => HirType::Owned {
            span: *span,
            ty: Box::new(lower_type(ty)),
        },
        Type::Claim { span, ty } => HirType::Claim {
            span: *span,
            ty: Box::new(lower_type(ty)),
        },
        Type::Verified { span, ty, policy } => HirType::Verified {
            span: *span,
            ty: Box::new(lower_type(ty)),
            policy: Box::new(lower_type(policy)),
        },
        Type::Array { span, ty, size } => HirType::Array {
            span: *span,
            ty: Box::new(lower_type(ty)),
            size: size.as_ref().map(|e| Box::new(lower_expr(e))),
        },
        Type::Tuple { span, types } => HirType::Tuple {
            span: *span,
            types: types.iter().map(lower_type).collect(),
        },
        Type::Fn {
            span,
            params,
            ret,
            effects,
        } => HirType::Fn {
            span: *span,
            params: params.iter().map(lower_type).collect(),
            ret: Box::new(lower_type(ret)),
            effects: lower_effect_set(effects),
        },
    }
}

fn lower_type_path(p: &TypePath) -> HirTypePath {
    HirTypePath {
        span: p.span,
        segments: p.segments.iter().map(lower_path_segment).collect(),
    }
}

fn lower_expr_path(p: &ExprPath) -> HirExprPath {
    HirExprPath {
        span: p.span,
        segments: p.segments.iter().map(lower_path_segment).collect(),
    }
}

fn lower_path_segment(s: &PathSegment) -> HirPathSegment {
    HirPathSegment {
        span: s.span,
        name: s.name.name.clone(),
        args: s.args.as_ref().map(lower_generic_args),
    }
}

fn lower_generic_args(args: &GenericArgs) -> HirGenericArgs {
    HirGenericArgs {
        span: args.span,
        args: args.args.iter().map(lower_generic_arg).collect(),
    }
}

fn lower_generic_arg(a: &GenericArg) -> HirGenericArg {
    match a {
        GenericArg::Type { span, ty } => HirGenericArg::Type {
            span: *span,
            ty: lower_type(ty),
        },
        GenericArg::Const { span, expr } => HirGenericArg::Const {
            span: *span,
            expr: lower_expr(expr),
        },
    }
}

// ── Blocks / statements ──────────────────────────────────────────────────────

fn lower_block(b: &Block) -> HirBlock {
    HirBlock {
        span: b.span,
        stmts: b.stmts.iter().map(lower_stmt).collect(),
        tail: b.tail.as_ref().map(|e| Box::new(lower_expr(e))),
    }
}

fn lower_stmt(s: &Stmt) -> HirStmt {
    match s {
        Stmt::Let {
            span,
            name,
            ty,
            is_mut,
            init,
        } => HirStmt::Let {
            span: *span,
            name: name.name.clone(),
            ty: ty.as_ref().map(lower_type),
            is_mut: *is_mut,
            init: init.as_ref().map(lower_expr),
        },
        Stmt::Expr { span, expr } => HirStmt::Expr {
            span: *span,
            expr: lower_expr(expr),
        },
        Stmt::Return { span, expr } => HirStmt::Return {
            span: *span,
            expr: expr.as_ref().map(lower_expr),
        },
        Stmt::If {
            span,
            cond,
            then_branch,
            else_branch,
        } => HirStmt::If {
            span: *span,
            cond: lower_expr(cond),
            then_branch: lower_block(then_branch),
            else_branch: else_branch.as_ref().map(|e| Box::new(lower_stmt(e))),
        },
        Stmt::While {
            span,
            cond,
            body,
        } => HirStmt::While {
            span: *span,
            cond: lower_expr(cond),
            body: lower_block(body),
        },
        Stmt::For {
            span,
            pat,
            iter,
            body,
        } => HirStmt::For {
            span: *span,
            pat: lower_pat(pat),
            iter: lower_expr(iter),
            body: lower_block(body),
        },
        Stmt::Match {
            span,
            scrutinee,
            arms,
        } => HirStmt::Match {
            span: *span,
            scrutinee: lower_expr(scrutinee),
            arms: arms.iter().map(lower_match_arm).collect(),
        },
        Stmt::Block { span, block } => HirStmt::Block {
            span: *span,
            block: lower_block(block),
        },
        // `use` inside a block is treated as no-op in HIR — use declarations
        // are handled at module level. We lower it to a no-op Expr statement
        // so we don't lose span information.
        Stmt::Use { span, .. } => HirStmt::Expr {
            span: *span,
            expr: HirExpr::Literal {
                span: *span,
                lit: HirLiteral::Unit { span: *span },
            },
        },
    }
}

// ── Expressions ─────────────────────────────────────────────────────────────

fn lower_expr(e: &Expr) -> HirExpr {
    match e {
        Expr::Literal { span, lit } => HirExpr::Literal {
            span: *span,
            lit: lower_literal(lit),
        },
        Expr::Path { span, path } => HirExpr::Path {
            span: *span,
            path: lower_expr_path(path),
        },
        Expr::Tuple { span, exprs } => HirExpr::Tuple {
            span: *span,
            exprs: exprs.iter().map(lower_expr).collect(),
        },
        Expr::Array { span, exprs } => HirExpr::Array {
            span: *span,
            exprs: exprs.iter().map(lower_expr).collect(),
        },
        Expr::Struct {
            span,
            path,
            fields,
            base,
        } => HirExpr::Struct {
            span: *span,
            path: lower_type_path(path),
            fields: fields.iter().map(lower_struct_expr_field).collect(),
            base: base.as_ref().map(|e| Box::new(lower_expr(e))),
        },
        Expr::Call {
            span,
            callee,
            args,
        } => HirExpr::Call {
            span: *span,
            callee: Box::new(lower_expr(callee)),
            args: args.iter().map(lower_expr).collect(),
        },
        Expr::MethodCall {
            span,
            receiver,
            method,
            args,
        } => HirExpr::MethodCall {
            span: *span,
            receiver: Box::new(lower_expr(receiver)),
            method: method.name.clone(),
            args: args.iter().map(lower_expr).collect(),
        },
        Expr::Field {
            span,
            base,
            field,
        } => HirExpr::Field {
            span: *span,
            base: Box::new(lower_expr(base)),
            field: field.name.clone(),
        },
        Expr::Index {
            span,
            base,
            index,
        } => HirExpr::Index {
            span: *span,
            base: Box::new(lower_expr(base)),
            index: Box::new(lower_expr(index)),
        },
        Expr::Unary { span, op, expr } => HirExpr::Unary {
            span: *span,
            op: lower_unary_op(*op),
            expr: Box::new(lower_expr(expr)),
        },
        Expr::Binary {
            span,
            op,
            left,
            right,
        } => HirExpr::Binary {
            span: *span,
            op: lower_binary_op(*op),
            left: Box::new(lower_expr(left)),
            right: Box::new(lower_expr(right)),
        },
        Expr::If {
            span,
            cond,
            then_branch,
            else_branch,
        } => HirExpr::If {
            span: *span,
            cond: Box::new(lower_expr(cond)),
            then_branch: Box::new(lower_expr(then_branch)),
            else_branch: else_branch.as_ref().map(|e| Box::new(lower_expr(e))),
        },
        Expr::Match {
            span,
            scrutinee,
            arms,
        } => HirExpr::Match {
            span: *span,
            scrutinee: Box::new(lower_expr(scrutinee)),
            arms: arms.iter().map(lower_match_arm).collect(),
        },
        Expr::Block { span, block } => HirExpr::Block {
            span: *span,
            block: lower_block(block),
        },
        Expr::Let {
            span,
            pat,
            ty,
            is_mut,
            init,
        } => HirExpr::Let {
            span: *span,
            pat: lower_pat(pat),
            ty: ty.as_ref().map(lower_type),
            is_mut: *is_mut,
            init: Box::new(lower_expr(init)),
        },
        Expr::Return { span, expr } => HirExpr::Return {
            span: *span,
            expr: expr.as_ref().map(|e| Box::new(lower_expr(e))),
        },
        Expr::Break { span, expr } => HirExpr::Break {
            span: *span,
            expr: expr.as_ref().map(|e| Box::new(lower_expr(e))),
        },
        Expr::Continue { span } => HirExpr::Continue { span: *span },
        Expr::Ask {
            span,
            model,
            goal,
            input,
            output_ty,
        } => HirExpr::Ask {
            span: *span,
            model: lower_expr_path(model),
            goal: goal.clone(),
            input: Box::new(lower_expr(input)),
            output_ty: lower_type(output_ty),
        },
        Expr::Verify {
            span,
            claim,
            policy,
        } => HirExpr::Verify {
            span: *span,
            claim: Box::new(lower_expr(claim)),
            policy: lower_type_path(policy),
        },
        Expr::Reason { span, prompt } => HirExpr::Reason {
            span: *span,
            prompt: prompt.clone(),
        },
        Expr::CommitOnce {
            span,
            effect,
            args,
        } => HirExpr::CommitOnce {
            span: *span,
            effect: lower_effect_ref(effect),
            args: args.iter().map(lower_expr).collect(),
        },
        Expr::New { span, ty, args } => HirExpr::New {
            span: *span,
            ty: lower_type(ty),
            args: args.iter().map(lower_expr).collect(),
        },
    }
}

fn lower_match_arm(a: &MatchArm) -> HirMatchArm {
    HirMatchArm {
        span: a.span,
        pat: lower_pat(&a.pat),
        guard: a.guard.as_ref().map(lower_expr),
        body: lower_expr(&a.body),
    }
}

fn lower_struct_expr_field(f: &StructExprField) -> HirStructExprField {
    HirStructExprField {
        span: f.span,
        name: f.name.name.clone(),
        expr: lower_expr(&f.expr),
    }
}

fn lower_literal(l: &Literal) -> HirLiteral {
    match l {
        Literal::Unit { span } => HirLiteral::Unit { span: *span },
        Literal::Bool { span, value } => HirLiteral::Bool {
            span: *span,
            value: *value,
        },
        Literal::Int { span, value } => HirLiteral::Int {
            span: *span,
            value: *value,
        },
        Literal::Float { span, value } => HirLiteral::Float {
            span: *span,
            value: *value,
        },
        Literal::String { span, value } => HirLiteral::String {
            span: *span,
            value: value.clone(),
        },
    }
}

fn lower_unary_op(op: UnaryOp) -> HirUnaryOp {
    match op {
        UnaryOp::Neg => HirUnaryOp::Neg,
        UnaryOp::Not => HirUnaryOp::Not,
        UnaryOp::Deref => HirUnaryOp::Deref,
    }
}

fn lower_binary_op(op: BinaryOp) -> HirBinaryOp {
    match op {
        BinaryOp::Add => HirBinaryOp::Add,
        BinaryOp::Sub => HirBinaryOp::Sub,
        BinaryOp::Mul => HirBinaryOp::Mul,
        BinaryOp::Div => HirBinaryOp::Div,
        BinaryOp::Rem => HirBinaryOp::Rem,
        BinaryOp::Eq => HirBinaryOp::Eq,
        BinaryOp::Ne => HirBinaryOp::Ne,
        BinaryOp::Lt => HirBinaryOp::Lt,
        BinaryOp::Le => HirBinaryOp::Le,
        BinaryOp::Gt => HirBinaryOp::Gt,
        BinaryOp::Ge => HirBinaryOp::Ge,
        BinaryOp::And => HirBinaryOp::And,
        BinaryOp::Or => HirBinaryOp::Or,
        BinaryOp::Assign => HirBinaryOp::Assign,
        BinaryOp::AddAssign => HirBinaryOp::AddAssign,
        BinaryOp::SubAssign => HirBinaryOp::SubAssign,
        BinaryOp::MulAssign => HirBinaryOp::MulAssign,
        BinaryOp::DivAssign => HirBinaryOp::DivAssign,
        BinaryOp::RemAssign => HirBinaryOp::RemAssign,
    }
}

// ── Patterns ─────────────────────────────────────────────────────────────────

fn lower_pat(p: &Pat) -> HirPat {
    match p {
        Pat::Wild { span } => HirPat::Wild { span: *span },
        Pat::Ident {
            span,
            name,
            is_mut,
        } => HirPat::Ident {
            span: *span,
            name: name.name.clone(),
            is_mut: *is_mut,
        },
        Pat::Literal { span, lit } => HirPat::Literal {
            span: *span,
            lit: lower_literal(lit),
        },
        Pat::Tuple { span, pats } => HirPat::Tuple {
            span: *span,
            pats: pats.iter().map(lower_pat).collect(),
        },
        Pat::Struct {
            span,
            path,
            fields,
        } => HirPat::Struct {
            span: *span,
            path: lower_type_path(path),
            fields: fields.iter().map(lower_pat_field).collect(),
        },
        Pat::Enum {
            span,
            path,
            fields,
        } => HirPat::Enum {
            span: *span,
            path: lower_type_path(path),
            fields: fields.iter().map(lower_pat).collect(),
        },
        Pat::Or { span, pats } => HirPat::Or {
            span: *span,
            pats: pats.iter().map(lower_pat).collect(),
        },
        Pat::Ref {
            span,
            is_mut,
            pat,
        } => HirPat::Ref {
            span: *span,
            is_mut: *is_mut,
            pat: Box::new(lower_pat(pat)),
        },
    }
}

fn lower_pat_field(f: &PatField) -> HirPatField {
    HirPatField {
        span: f.span,
        name: f.name.name.clone(),
        pat: f.pat.as_ref().map(lower_pat),
    }
}

#[cfg(test)]
mod tests {
    //! Ad-hoc verification tests for the AST → HIR lowering entrypoint.
    //! Exercises `lower_module` against real parser output to confirm
    //! every AST node type is correctly mapped to its HIR counterpart.

    use super::*;
    use aethel_syntax::lexer::lex;
    use aethel_syntax::parser::parse;
    use aethel_syntax::span::FileId;

    fn parse_source(src: &str) -> Option<Module> {
        let fid = FileId::new(0);
        let tokens = lex(src, fid);
        let (module, diags) = parse(&tokens, fid);
        if diags.has_errors() {
            None
        } else {
            Some(module)
        }
    }

    fn lower(src: &str) -> HirModule {
        let ast = parse_source(src).expect("source must parse");
        lower_module(&ast, FileId::new(0))
    }

    fn names(items: &[HirItem]) -> Vec<String> {
        items
            .iter()
            .map(|i| match i {
                HirItem::Fn(f) => format!("fn {}", f.name),
                HirItem::Struct(s) => format!("struct {}", s.name),
                HirItem::Enum(e) => format!("enum {}", e.name),
                HirItem::TypeAlias(t) => format!("type {}", t.name),
                HirItem::Use(_) => "use".to_string(),
                HirItem::Mod(m) => format!("mod {}", m.name),
                HirItem::Policy(p) => format!("policy {}", p.name),
                HirItem::Effect(e) => format!("effect {}", e.name),
            })
            .collect()
    }

    #[test]
    fn lowers_full_pipeline_example() {
        // The canonical full-pipeline example exercises effects, policies,
        // structs, `verify`, `reason`, and method calls — i.e. the full
        // breadth of AST node types that the lowering must handle.
        let src = include_str!("../../../examples/full_pipeline.aet");
        let ast = parse_source(src).expect("full_pipeline.aet must parse");

        // Sanity: 2 structs + 2 policies + 2 effects + 1 fn = 7 top-level items.
        assert_eq!(ast.items.len(), 7, "AST item count");

        let hir = lower_module(&ast, FileId::new(0));
        assert_eq!(hir.items.len(), 7, "HIR item count");

        let got = names(&hir.items);
        assert!(got.contains(&"struct UserAction".to_string()));
        assert!(got.contains(&"struct ActionResult".to_string()));
        assert!(got.contains(&"policy RiskAssessment".to_string()));
        assert!(got.contains(&"policy AdminOverride".to_string()));
        assert!(got.contains(&"effect AuditService".to_string()));
        assert!(got.contains(&"effect ExecutionService".to_string()));
        assert!(got.contains(&"fn process_action".to_string()));
    }

    #[test]
    fn lowers_effects_with_operations() {
        let src = r#"
            effect AuditService {
                fn log_action(action: Verified<UserAction, RiskAssessment>) -> ActionResult
                fn log_override(action: Verified<UserAction, AdminOverride>) -> ActionResult
            }
        "#;
        let hir = lower(src);
        assert_eq!(hir.items.len(), 1);

        let effect = match &hir.items[0] {
            HirItem::Effect(e) => e,
            other => panic!("expected HirItem::Effect, got {:?}", other),
        };
        assert_eq!(effect.name, "AuditService");
        assert_eq!(effect.operations.len(), 2);
        assert_eq!(effect.operations[0].name, "log_action");
        assert_eq!(effect.operations[1].name, "log_override");
        // The `Verified<...>` param type must be lowered into HirType::Verified
        // with a HirType::Path policy.
        let param_ty = &effect.operations[0].params[0].ty;
        match param_ty {
            HirType::Verified { ty, policy, .. } => {
                // inner: UserAction → HirType::Path
                assert!(matches!(ty.as_ref(), HirType::Path { .. }));
                // policy: RiskAssessment → HirType::Path
                assert!(matches!(policy.as_ref(), HirType::Path { .. }));
            }
            other => panic!("expected HirType::Verified, got {:?}", other),
        }
    }

    #[test]
    fn lowers_policy_claims_and_evidence() {
        let src = r#"
            policy RiskAssessment {
                ActionRisk: UserAction {
                    evidence SignedAttestation "Risk model v2.1 assessment"
                }
            }
        "#;
        let ast = parse_source(src).expect("policy must parse");
        let hir = lower_module(&ast, FileId::new(0));
        // Whatever the parser decides about claim count, the lowering must
        // faithfully mirror it. Assert the AST and HIR agree.
        let ast_policy = match &ast.items[0] {
            Item::Policy(p) => p,
            other => panic!("expected AST Item::Policy, got {:?}", other),
        };
        let hir_policy = match &hir.items[0] {
            HirItem::Policy(p) => p,
            other => panic!("expected HirItem::Policy, got {:?}", other),
        };
        assert_eq!(hir_policy.name, "RiskAssessment");
        assert_eq!(
            hir_policy.claims.len(),
            ast_policy.claims.len(),
            "HIR claim count must match AST claim count"
        );
        // Verify the claim lowered correctly (name, type, evidence kind).
        if let Some(claim) = hir_policy.claims.first() {
            assert_eq!(claim.name, ast_policy.claims[0].name.name);
            assert!(matches!(claim.ty, HirType::Path { .. }));
            if let Some(ev) = claim.evidence.first() {
                assert!(matches!(ev.kind, HirEvidenceKind::SignedAttestation));
            }
        }
    }

    #[test]
    fn lowers_function_body_with_verify_and_method_call() {
        // Note: `reason("...")` parses as a regular function call (no
        // `reason` keyword exists in the current grammar), so we don't
        // assert HirExpr::Reason here. `verify` *is* a keyword.
        let src = r#"
            fn process_action(raw_action: Claim<UserAction>) -> ActionResult
            uses AuditService, ExecutionService:
                {
                    let reason_call = reason("Analyzing action risk");
                    let assessed = verify(raw_action, RiskAssessment);
                    let log = audit_service.log_action(assessed);
                    let result = execution_service.execute(assessed);
                    return result;
                }
        "#;
        let hir = lower(src);
        let func = match &hir.items[0] {
            HirItem::Fn(f) => f,
            other => panic!("expected HirItem::Fn, got {:?}", other),
        };
        assert_eq!(func.name, "process_action");
        assert_eq!(func.params.len(), 1);
        assert_eq!(func.params[0].name, "raw_action");
        assert_eq!(func.effects.effects.len(), 2);
        assert!(func.body.is_some(), "body must be lowered");

        let body = func.body.as_ref().expect("body present");
        assert_eq!(body.stmts.len(), 5);

        // Stmt 0: `let reason_call = reason("...")` → HirExpr::Call
        if let HirStmt::Let { init: Some(init), .. } = &body.stmts[0] {
            assert!(matches!(init, HirExpr::Call { .. }));
        } else {
            panic!("stmt[0] should be Let with Call init");
        }

        // Stmt 1: `let assessed = verify(raw_action, RiskAssessment)` → Verify
        if let HirStmt::Let { init: Some(init), .. } = &body.stmts[1] {
            match init {
                HirExpr::Verify { claim, policy, .. } => {
                    assert!(matches!(claim.as_ref(), HirExpr::Path { .. }));
                    assert_eq!(policy.segments.len(), 1);
                    assert_eq!(policy.segments[0].name, "RiskAssessment");
                }
                other => panic!("stmt[1] init should be Verify, got {:?}", other),
            }
        } else {
            panic!("stmt[1] should be Let with Verify init");
        }

        // Stmt 2: `let log = audit_service.log_action(assessed)` → MethodCall
        if let HirStmt::Let { init: Some(init), .. } = &body.stmts[2] {
            match init {
                HirExpr::MethodCall {
                    receiver, method, args, ..
                } => {
                    assert!(matches!(receiver.as_ref(), HirExpr::Path { .. }));
                    assert_eq!(method, "log_action");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("stmt[2] init should be MethodCall, got {:?}", other),
            }
        } else {
            panic!("stmt[2] should be Let with MethodCall init");
        }

        // Final stmt: `return result;` → HirStmt::Return
        assert!(matches!(body.stmts[4], HirStmt::Return { .. }));
    }

    #[test]
    fn lowers_reason_expr_form_directly() {
        // The AST has an Expr::Reason variant even though the parser does
        // not yet emit it for real source. Lowering must still handle it
        // when present in the AST (e.g. synthesized in future passes).
        let ast = Module {
            span: Span::single(FileId::new(0), crate::span::ByteOffset(0)),
            items: vec![Item::Fn(FnDef {
                span: Span::single(FileId::new(0), crate::span::ByteOffset(0)),
                name: Ident::dummy("synth"),
                generics: Vec::new(),
                params: Vec::new(),
                ret_type: None,
                effects: EffectSet::default(),
                body: Some(Block {
                    span: Span::single(FileId::new(0), crate::span::ByteOffset(0)),
                    stmts: vec![Stmt::Expr {
                        span: Span::single(FileId::new(0), crate::span::ByteOffset(0)),
                        expr: Expr::Reason {
                            span: Span::single(FileId::new(0), crate::span::ByteOffset(0)),
                            prompt: "synth-prompt".into(),
                        },
                    }],
                    tail: None,
                }),
                is_pub: false,
            })],
        };
        let hir = lower_module(&ast, FileId::new(0));
        let func = match &hir.items[0] {
            HirItem::Fn(f) => f,
            _ => panic!("expected Fn"),
        };
        let body = func.body.as_ref().expect("body");
        if let HirStmt::Expr { expr, .. } = &body.stmts[0] {
            match expr {
                HirExpr::Reason { prompt, .. } => {
                    assert_eq!(prompt, "synth-prompt");
                }
                other => panic!("expected HirExpr::Reason, got {:?}", other),
            }
        } else {
            panic!("expected Expr stmt");
        }
    }

    #[test]
    fn lowers_struct_def() {
        let src = r#"
            struct UserAction {
                id: string,
                description: string,
                risk_score: int,
            }
        "#;
        let hir = lower(src);
        let s = match &hir.items[0] {
            HirItem::Struct(s) => s,
            other => panic!("expected HirItem::Struct, got {:?}", other),
        };
        assert_eq!(s.name, "UserAction");
        assert_eq!(s.fields.len(), 3);
        assert_eq!(s.fields[0].name, "id");
        assert!(matches!(s.fields[0].ty, HirType::String { .. }));
        assert_eq!(s.fields[2].name, "risk_score");
        assert!(matches!(s.fields[2].ty, HirType::Int { .. }));
    }

    #[test]
    fn lowers_empty_module() {
        // A single newline parses to an empty module (verified empirically).
        let src = "\n";
        let hir = lower(src);
        assert!(hir.items.is_empty());
    }
}