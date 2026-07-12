#!/usr/bin/env python3
"""Precise fix of E0308 type mismatches in checker.rs"""
import re

path = r"C:\Users\Ismael\aethel\crates\ethel-check\src\checker.rs"
alt_path = r"C:\Users\Ismael\aethel\crates\aethel-check\src\checker.rs"
import os
for p in [path, alt_path]:
    if os.path.exists(p):
        path = p
        break

with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

fixes = {}

# Fix 1: Enum variant field types (line ~152-153)
fixes['enum_fields'] = (
    'aethel_hir::lower::HirEnumField::Tuple { ty, .. } => ty.clone(),\n'
    '                        aethel_hir::lower::HirEnumField::Named { ty, .. } => ty.clone(),',
    'aethel_hir::lower::HirEnumField::Tuple { ty, .. } => crate::types::lower_hir_type(&ty),\n'
    '                        aethel_hir::lower::HirEnumField::Named { ty, .. } => crate::types::lower_hir_type(&ty),'
)

# Fix 2: IrParam ty clone (line ~234)
fixes['irparam_ty'] = (
    'aethel_ir::lower::IrParam {\n'
    '            span: p.span,\n'
    '            name: p.name.clone(),\n'
    '            ty: p.ty.clone(),\n'
    '            is_mut: p.is_mut,\n'
    '        }).collect(),',
    'aethel_ir::lower::IrParam {\n'
    '            span: p.span,\n'
    '            name: p.name.clone(),\n'
    '            ty: crate::types::lower_hir_type(&p.ty),\n'
    '            is_mut: p.is_mut,\n'
    '        }).collect(),'
)

# Fix 3: ret_type clone (line ~237)
fixes['ret_type'] = (
    'ret_type: f.ret_type.clone().unwrap_or_else(|| aethel_ir::lower::IrType::Unit { span: f.span }),',
    'ret_type: f.ret_type.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or(aethel_ir::lower::IrType::Unit { span: f.span }),'
)

# Fix 4: Let statement ty_ir (line ~267-270)
fixes['stmt_let_ty'] = (
    'let ty_ir = ty.clone().unwrap_or_else(|| {\n'
    '                // Infer from init\n'
    '                init_ir.as_ref().map(|e| e.ty().clone()).unwrap_or(aethel_ir::lower::IrType::Unit { span: *span })\n'
    '            });',
    'let ty_ir = ty.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or_else(|| {\n'
    '                // Infer from init\n'
    '                aethel_ir::lower::IrType::Unit { span: *span }\n'
    '            });'
)

# Fix 5: HirExpr::Let ty field (line ~417)
fixes['expr_let_ty'] = (
    'ty: ty.clone().unwrap_or_else(|| aethel_ir::lower::IrType::Unit { span }),',
    'ty: ty.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or(aethel_ir::lower::IrType::Unit { span }),'
)

# Fix 6: Ask output_ty (line ~443)
fixes['ask_output'] = (
    'output_ty: output_ty.clone(),',
    'output_ty: crate::types::lower_hir_type(&output_ty),'
)

# Fix 7: IrExpr::New ty (line ~485)  - unique context
fixes['expr_new_ty'] = (
    'HirExpr::New { ty, args, .. } => {\n'
    '            Some(IrExpr::New {\n'
    '                span,\n'
    '                ty: ty.clone(),\n'
    '                args: args.iter().map(|a| check_expr(ctx, a)).collect::<Option<Vec<_>>>()?,\n'
    '            })\n'
    '        }',
    'HirExpr::New { ty, args, .. } => {\n'
    '            Some(IrExpr::New {\n'
    '                span,\n'
    '                ty: crate::types::lower_hir_type(&ty),\n'
    '                args: args.iter().map(|a| check_expr(ctx, a)).collect::<Option<Vec<_>>>()?,\n'
    '            })\n'
    '        }'
)

# Fix 8: IrStructField ty (line ~552)
fixes['struct_field_ty'] = (
    'aethel_ir::lower::IrStructField {\n'
    '            span: f.span,\n'
    '            name: f.name.clone(),\n'
    '            ty: f.ty.clone(),\n'
    '            is_pub: f.is_pub,\n'
    '        }).collect(),',
    'aethel_ir::lower::IrStructField {\n'
    '            span: f.span,\n'
    '            name: f.name.clone(),\n'
    '            ty: crate::types::lower_hir_type(&f.ty),\n'
    '            is_pub: f.is_pub,\n'
    '        }).collect(),'
)

# Fix 9: IrEnumField::Tuple (line ~575)
fixes['enum_field_tuple'] = (
    'aethel_hir::lower::HirEnumField::Tuple { span, ty } => {\n'
    '                            aethel_ir::lower::IrEnumField::Tuple {\n'
    '                                span: *span,\n'
    '                                ty: ty.clone(),\n'
    '                            }\n'
    '                        }',
    'aethel_hir::lower::HirEnumField::Tuple { span, ty } => {\n'
    '                            aethel_ir::lower::IrEnumField::Tuple {\n'
    '                                span: *span,\n'
    '                                ty: crate::types::lower_hir_type(&ty),\n'
    '                            }\n'
    '                        }'
)

# Fix 10: IrEnumField::Named (line ~576)  
fixes['enum_field_named'] = (
    'aethel_hir::lower::HirEnumField::Named { span, name, ty } => {\n'
    '                            aethel_ir::lower::IrEnumField::Named {\n'
    '                                span: *span,\n'
    '                                name: name.clone(),\n'
    '                                ty: ty.clone(),\n'
    '                            }\n'
    '                        }',
    'aethel_hir::lower::HirEnumField::Named { span, name, ty } => {\n'
    '                            aethel_ir::lower::IrEnumField::Named {\n'
    '                                span: *span,\n'
    '                                name: name.clone(),\n'
    '                                ty: crate::types::lower_hir_type(&ty),\n'
    '                            }\n'
    '                        }'
)

count = 0
for name, (old, new) in fixes.items():
    if old in content:
        content = content.replace(old, new, 1)
        count += 1
        print(f"Applied fix: {name}")
    else:
        print(f"SKIP (not found): {name}")

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)
print(f"\nTotal: {count} fixes applied")
