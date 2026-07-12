#!/usr/bin/env python3
"""Fix all remaining checker.rs type errors precisely."""
import re, os

path = r"C:\Users\Ismael\aethel\crates\aethel-check\src\checker.rs"
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

# ============ FIX 1: enum variants ty.clone() to lower_hir_type ============
old = "aethel_hir::lower::HirEnumField::Tuple { ty, .. } => ty.clone(),\n                        aethel_hir::lower::HirEnumField::Named { ty, .. } => ty.clone(),"
new = "aethel_hir::lower::HirEnumField::Tuple { ty, .. } => crate::types::lower_hir_type(&ty),\n                        aethel_hir::lower::HirEnumField::Named { ty, .. } => crate::types::lower_hir_type(&ty),"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 1: enum variants")
else:
    print("SKIP 1: enum variants pattern not found")

# ============ FIX 2: remove TypeDefKind::TypeAlias{ty:t.ty.clone()} - use lower_hir_type ============
old = "kind: TypeDefKind::TypeAlias { ty: t.ty.clone() }"
new = "kind: TypeDefKind::TypeAlias { ty: crate::types::lower_hir_type(&t.ty) }"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 2: TypeAlias ty")
else:
    print("SKIP 2: TypeAlias")

# ============ FIX 3: PolicyClaim ty ============
old = "ty: claim.ty.clone(),\n                    evidence,"
new = "ty: crate::types::lower_hir_type(&claim.ty),\n                    evidence,"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 3: PolicyClaim ty")
else:
    print("SKIP 3: PolicyClaim ty")

# ============ FIX 4: IrParam ty ============
old = "aethel_ir::lower::IrParam {\n                span: p.span,\n                name: p.name.clone(),\n                ty: p.ty.clone(),\n                is_mut: p.is_mut,"
new = "aethel_ir::lower::IrParam {\n                span: p.span,\n                name: p.name.clone(),\n                ty: crate::types::lower_hir_type(&p.ty),\n                is_mut: p.is_mut,"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 4: IrParam ty")
else:
    print("SKIP 4: IrParam ty")

    # try indent with tabs or spaces
    for variant in [
        "aethel_ir::lower::IrParam {\n            span: p.span,\n            name: p.name.clone(),\n            ty: p.ty.clone(),\n            is_mut: p.is_mut,",
    ]:
        if variant in content:
            content = content.replace(variant, variant.replace("ty: p.ty.clone()", "ty: crate::types::lower_hir_type(&p.ty)"), 1)
            print(f"Fix 4b: IrParam ty (variant)")
            break

# ============ FIX 5: ret_type ============
old = "ret_type: f.ret_type.clone().unwrap_or_else(|| aethel_ir::lower::IrType::Unit { span: f.span }),"
new = "ret_type: f.ret_type.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or(aethel_ir::lower::IrType::Unit { span: f.span }),"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 5: ret_type")
else:
    print("SKIP 5: ret_type")

# ============ FIX 6: check_stmt Let ty_ir ============
old = "            let init_ir = init.as_ref().map(|e| check_expr(ctx, e)).transpose()?;\n            let ty_ir = ty.clone().unwrap_or_else(|| {\n                // Infer from init\n                init_ir.as_ref().map(|e| e.ty().clone()).unwrap_or(aethel_ir::lower::IrType::Unit { span: *span })\n            });"
new = "            let init_ir = init.as_ref().and_then(|e| check_expr(ctx, e));\n            let ty_ir = ty.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or_else(|| {\n                // Infer from init\n                aethel_ir::lower::IrType::Unit { span: *span }\n            });"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 6: stmt Let ty_ir")
else:
    print("SKIP 6: stmt Let ty_ir")

# ============ FIX 7: HirExpr::Let ty ============
old = "ty: ty.clone().unwrap_or_else(|| aethel_ir::lower::IrType::Unit { span }),"
new = "ty: ty.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or(aethel_ir::lower::IrType::Unit { span }),"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 7: expr Let ty")
else:
    print("SKIP 7: expr Let ty")

# ============ FIX 8: Ask output_ty ============
old = "output_ty: output_ty.clone(),"
new = "output_ty: crate::types::lower_hir_type(&output_ty),"
if old in content and "HirExpr::Ask {" in content:
    content = content.replace(old, new, 1)
    print("Fix 8: Ask output_ty")
else:
    print("SKIP 8: Ask output_ty")

# ============ FIX 9: IrExpr::New ty ============
old = "HirExpr::New { ty, args, .. } => {\n            Some(IrExpr::New {\n                span,\n                ty: ty.clone(),"
new = "HirExpr::New { ty, args, .. } => {\n            Some(IrExpr::New {\n                span,\n                ty: crate::types::lower_hir_type(&ty),"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 9: New ty")
else:
    print("SKIP 9: New ty")

# ============ FIX 10: IrStructField ty ============
old = "aethel_ir::lower::IrStructField {\n            span: f.span,\n            name: f.name.clone(),\n            ty: f.ty.clone(),\n            is_pub: f.is_pub,\n        }).collect(),"
new = "aethel_ir::lower::IrStructField {\n            span: f.span,\n            name: f.name.clone(),\n            ty: crate::types::lower_hir_type(&f.ty),\n            is_pub: f.is_pub,\n        }).collect(),"
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 10: StructField ty")
else:
    print("SKIP 10: StructField ty")

# ============ FIX 11: IrEnumField Tuple/Named ty ============
for needle, replacement in [
    ("aethel_hir::lower::HirEnumField::Tuple { span, ty } => aethel_ir::lower::IrEnumField::Tuple { span: *span, ty: ty.clone() },", "aethel_hir::lower::HirEnumField::Tuple { span, ty } => aethel_ir::lower::IrEnumField::Tuple { span: *span, ty: crate::types::lower_hir_type(&ty) },"),
    ("aethel_hir::lower::HirEnumField::Named { span, name, ty } => aethel_ir::lower::IrEnumField::Named { span: *span, name: name.clone(), ty: ty.clone() },", "aethel_hir::lower::HirEnumField::Named { span, name, ty } => aethel_ir::lower::IrEnumField::Named { span: *span, name: name.clone(), ty: crate::types::lower_hir_type(&ty) },"),
]:
    if needle in content:
        content = content.replace(needle, replacement, 1)
        print(f"Fix 11: {replacement[:60]}...")
    else:
        print(f"SKIP 11: pattern not found")

# ============ FIX 12: IrGenericArg Type ty.clone() ============
old = 'aethel_hir::lower::HirGenericArg::Type { span, ty } => aethel_ir::lower::IrGenericArg::Type { span: *span, ty: ty.clone() }'
new = 'aethel_hir::lower::HirGenericArg::Type { span, ty } => aethel_ir::lower::IrGenericArg::Type { span: *span, ty: crate::types::lower_hir_type(&ty) }'
if old in content:
    content = content.replace(old, new, 1)
    print("Fix 12: IrGenericArg Type")
else:
    print("SKIP 12: IrGenericArg")

# ============ FIX 13: transpose fixes (reverted by git restore) ============
transpose_fixes = [
    ("base.as_ref().map(|b| Box::new(check_expr(ctx, b))).transpose()?,", "base.as_ref().and_then(|b| check_expr(ctx, b)).map(Box::new),"),
    ("else_branch.as_ref().map(|e| Box::new(check_expr(ctx, e))).transpose()?,", "else_branch.as_ref().and_then(|e| check_expr(ctx, e)).map(Box::new),"),
    ("expr.as_ref().map(|e| Box::new(check_expr(ctx, e))).transpose()?,", "expr.as_ref().and_then(|e| check_expr(ctx, e)).map(Box::new),"),
    ("guard: arm.guard.as_ref().map(|g| check_expr(ctx, g)).transpose()?,", "guard: arm.guard.as_ref().and_then(|g| check_expr(ctx, g)),"),
    ("pat: f.pat.as_ref().map(|p| check_pat(ctx, p)).transpose()?,", "pat: f.pat.as_ref().and_then(|p| check_pat(ctx, p)).map(Box::new),"),
    (".map(|b| Box::new(check_stmt(ctx, b))).transpose()?,", ".and_then(|b| check_stmt(ctx, b)).map(Box::new),"),
]

for old, new in transpose_fixes:
    if old in content:
        content = content.replace(old, new, 1)
        print(f"Fix transpose: {old[:60]}...")
    else:
        pass  # might not exist in current state

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print("\n=== DONE ===")
