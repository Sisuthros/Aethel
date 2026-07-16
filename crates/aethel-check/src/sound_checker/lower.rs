//! Structural HIR to IR lowering after semantic validation.

#![allow(clippy::all, clippy::pedantic, clippy::nursery)]

use aethel_hir::lower as hir;
use aethel_ir::lower as ir;
use aethel_syntax::span::FileId;

use super::util::{lower_effect_set, lower_generics, lower_type, lower_type_path, lower_use_path};

pub(super) fn lower_module(module: &hir::HirModule, file_id: FileId) -> ir::IrModule {
    ir::IrModule {
        file_id,
        items: module.items.iter().filter_map(lower_item).collect(),
    }
}

fn lower_item(item: &hir::HirItem) -> Option<ir::IrItem> {
    match item {
        hir::HirItem::Fn(def) => Some(ir::IrItem::Fn(ir::IrFnDef {
            span: def.span,
            name: def.name.clone(),
            generics: lower_generics(&def.generics),
            params: def
                .params
                .iter()
                .map(|param| ir::IrParam {
                    span: param.span,
                    name: param.name.clone(),
                    ty: lower_type(&param.ty),
                    is_mut: param.is_mut,
                })
                .collect(),
            ret_type: def
                .ret_type
                .as_ref()
                .map_or(ir::IrType::Unit { span: def.span }, lower_type),
            effects: lower_effect_set(&def.effects),
            body: def.body.as_ref().map(lower_block),
            is_pub: def.is_pub,
        })),
        hir::HirItem::Struct(def) => Some(ir::IrItem::Struct(ir::IrStructDef {
            span: def.span,
            name: def.name.clone(),
            generics: lower_generics(&def.generics),
            fields: def
                .fields
                .iter()
                .map(|field| ir::IrStructField {
                    span: field.span,
                    name: field.name.clone(),
                    ty: lower_type(&field.ty),
                    is_pub: field.is_pub,
                })
                .collect(),
            is_pub: def.is_pub,
        })),
        hir::HirItem::Enum(def) => Some(ir::IrItem::Enum(ir::IrEnumDef {
            span: def.span,
            name: def.name.clone(),
            generics: lower_generics(&def.generics),
            variants: def
                .variants
                .iter()
                .map(|variant| ir::IrEnumVariant {
                    span: variant.span,
                    name: variant.name.clone(),
                    fields: variant
                        .fields
                        .iter()
                        .map(|field| match field {
                            hir::HirEnumField::Tuple { span, ty } => ir::IrEnumField::Tuple {
                                span: *span,
                                ty: lower_type(ty),
                            },
                            hir::HirEnumField::Named { span, name, ty } => ir::IrEnumField::Named {
                                span: *span,
                                name: name.clone(),
                                ty: lower_type(ty),
                            },
                        })
                        .collect(),
                })
                .collect(),
            is_pub: def.is_pub,
        })),
        hir::HirItem::TypeAlias(def) => Some(ir::IrItem::TypeAlias(ir::IrTypeAlias {
            span: def.span,
            name: def.name.clone(),
            generics: lower_generics(&def.generics),
            ty: lower_type(&def.ty),
            is_pub: def.is_pub,
        })),
        hir::HirItem::Use(def) => Some(ir::IrItem::Use(ir::IrUseDecl {
            span: def.span,
            path: lower_use_path(&def.path),
            is_pub: def.is_pub,
        })),
        hir::HirItem::Mod(def) => Some(ir::IrItem::Mod(ir::IrModDecl {
            span: def.span,
            name: def.name.clone(),
            body: def
                .body
                .as_ref()
                .map(|body| lower_module(body, body.file_id)),
            is_pub: def.is_pub,
        })),
        hir::HirItem::Policy(def) => Some(ir::IrItem::Policy(ir::IrPolicyDef {
            span: def.span,
            name: def.name.clone(),
            generics: lower_generics(&def.generics),
            claims: def
                .claims
                .iter()
                .map(|claim| ir::IrPolicyClaim {
                    span: claim.span,
                    name: claim.name.clone(),
                    ty: lower_type(&claim.ty),
                    evidence: claim
                        .evidence
                        .iter()
                        .map(|evidence| ir::IrEvidenceReq {
                            span: evidence.span,
                            kind: match &evidence.kind {
                                hir::HirEvidenceKind::SignedAttestation => {
                                    ir::IrEvidenceKind::SignedAttestation
                                }
                                hir::HirEvidenceKind::CryptographicProof => {
                                    ir::IrEvidenceKind::CryptographicProof
                                }
                                hir::HirEvidenceKind::AuditLog => ir::IrEvidenceKind::AuditLog,
                                hir::HirEvidenceKind::HumanReview => {
                                    ir::IrEvidenceKind::HumanReview
                                }
                                hir::HirEvidenceKind::Custom(value) => {
                                    ir::IrEvidenceKind::Custom(value.clone())
                                }
                            },
                            description: evidence.description.clone(),
                        })
                        .collect(),
                })
                .collect(),
            is_pub: def.is_pub,
        })),
        hir::HirItem::Effect(_) => None,
    }
}

fn lower_block(block: &hir::HirBlock) -> ir::IrBlock {
    ir::IrBlock {
        span: block.span,
        stmts: block.stmts.iter().map(lower_stmt).collect(),
        tail: block
            .tail
            .as_ref()
            .map(|expr| Box::new(aethel_check_expr(expr))),
    }
}

fn lower_stmt(stmt: &hir::HirStmt) -> ir::IrStmt {
    match stmt {
        hir::HirStmt::Let {
            span,
            name,
            ty,
            is_mut,
            init,
        } => ir::IrStmt::Let {
            span: *span,
            name: name.clone(),
            ty: ty
                .as_ref()
                .map_or(ir::IrType::Unit { span: *span }, lower_type),
            is_mut: *is_mut,
            init: init.as_ref().map(aethel_check_expr),
        },
        hir::HirStmt::Expr { span, expr } => ir::IrStmt::Expr {
            span: *span,
            expr: aethel_check_expr(expr),
        },
        hir::HirStmt::Return { span, expr } => ir::IrStmt::Return {
            span: *span,
            expr: expr.as_ref().map(aethel_check_expr),
        },
        hir::HirStmt::If {
            span,
            cond,
            then_branch,
            else_branch,
        } => ir::IrStmt::If {
            span: *span,
            cond: aethel_check_expr(cond),
            then_branch: lower_block(then_branch),
            else_branch: else_branch.as_ref().map(|stmt| Box::new(lower_stmt(stmt))),
        },
        hir::HirStmt::While { span, cond, body } => ir::IrStmt::While {
            span: *span,
            cond: aethel_check_expr(cond),
            body: lower_block(body),
        },
        hir::HirStmt::For {
            span,
            pat,
            iter,
            body,
        } => ir::IrStmt::For {
            span: *span,
            pat: lower_pat(pat),
            iter: aethel_check_expr(iter),
            body: lower_block(body),
        },
        hir::HirStmt::Match {
            span,
            scrutinee,
            arms,
        } => ir::IrStmt::Match {
            span: *span,
            scrutinee: aethel_check_expr(scrutinee),
            arms: arms.iter().map(lower_match_arm).collect(),
        },
        hir::HirStmt::Block { span, block } => ir::IrStmt::Block {
            span: *span,
            block: lower_block(block),
        },
    }
}

fn aethel_check_expr(expr: &hir::HirExpr) -> ir::IrExpr {
    crate::types::lower_hir_expr(expr)
}

fn lower_match_arm(arm: &hir::HirMatchArm) -> ir::IrMatchArm {
    ir::IrMatchArm {
        span: arm.span,
        pat: lower_pat(&arm.pat),
        guard: arm.guard.as_ref().map(aethel_check_expr),
        body: aethel_check_expr(&arm.body),
    }
}

fn lower_pat(pat: &hir::HirPat) -> ir::IrPat {
    match pat {
        hir::HirPat::Wild { span } => ir::IrPat::Wild { span: *span },
        hir::HirPat::Ident { span, name, is_mut } => ir::IrPat::Ident {
            span: *span,
            name: name.clone(),
            is_mut: *is_mut,
        },
        hir::HirPat::Literal { span, lit } => ir::IrPat::Literal {
            span: *span,
            lit: match lit {
                hir::HirLiteral::Unit { span } => ir::IrLiteral::Unit { span: *span },
                hir::HirLiteral::Bool { span, value } => ir::IrLiteral::Bool {
                    span: *span,
                    value: *value,
                },
                hir::HirLiteral::Int { span, value } => ir::IrLiteral::Int {
                    span: *span,
                    value: *value,
                },
                hir::HirLiteral::Float { span, value } => ir::IrLiteral::Float {
                    span: *span,
                    value: *value,
                },
                hir::HirLiteral::String { span, value } => ir::IrLiteral::String {
                    span: *span,
                    value: value.clone(),
                },
            },
        },
        hir::HirPat::Tuple { span, pats } => ir::IrPat::Tuple {
            span: *span,
            pats: pats.iter().map(lower_pat).collect(),
        },
        hir::HirPat::Struct { span, path, fields } => ir::IrPat::Struct {
            span: *span,
            path: lower_type_path(path),
            fields: fields
                .iter()
                .map(|field| ir::IrPatField {
                    span: field.span,
                    name: field.name.clone(),
                    pat: field.pat.as_ref().map(lower_pat),
                })
                .collect(),
        },
        hir::HirPat::Enum { span, path, fields } => ir::IrPat::Enum {
            span: *span,
            path: lower_type_path(path),
            fields: fields.iter().map(lower_pat).collect(),
        },
        hir::HirPat::Or { span, pats } => ir::IrPat::Or {
            span: *span,
            pats: pats.iter().map(lower_pat).collect(),
        },
        hir::HirPat::Ref { span, is_mut, pat } => ir::IrPat::Ref {
            span: *span,
            is_mut: *is_mut,
            pat: Box::new(lower_pat(pat)),
        },
    }
}
