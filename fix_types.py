#!/usr/bin/env python3
"""Fix E0308 type mismatches in checker.rs - wrap HIR type values with lower_hir_type"""
import re

path = r"C:\Users\Ismael\aethel\crates\aethel-check\src\checker.rs"
with open(path, 'r', encoding='utf-8') as f:
    content = f.read()

fixes = 0

# 1) Enum variant fields - lines ~151-153
# ty.clone() -> crate::types::lower_hir_type(&ty) in match arms
for pattern, replacement in [
    # Enum field clones
    (r'ty\.clone\(\),?\s*\}', 'crate::types::lower_hir_type(&ty) },'),
    # but only for HirEnumField matches (not the IrType ones later)
]:
    content = re.sub(pattern, replacement, content)

# Actually, let me be more targeted.

# Specific known errors:
# Line 150-154: HirEnumField ty.clone() -> lower_hir_type
content = content.replace(
    '                        aethel_hir::lower::HirEnumField::Tuple { ty, .. } => ty.clone(),\n                        aethel_hir::lower::HirEnumField::Named { ty, .. } => ty.clone(),',
    '                        aethel_hir::lower::HirEnumField::Tuple { ty, .. } => crate::types::lower_hir_type(&ty),\n                        aethel_hir::lower::HirEnumField::Named { ty, .. } => crate::types::lower_hir_type(&ty),'
)

# Line 234: ty: p.ty.clone() -> lower_hir_type
# This is in a map closure building IrParam. p.ty is HirType.
content = content.replace(
    '            ty: p.ty.clone(),',
    '            ty: crate::types::lower_hir_type(&p.ty),'
)

# Line 237: ret_type: f.ret_type.clone().unwrap_or_else(...
# f.ret_type is Option<HirType>, need IrType
content = content.replace(
    '        ret_type: f.ret_type.clone().unwrap_or_else(|| aethel_ir::lower::IrType::Unit { span: f.span }),',
    '        ret_type: f.ret_type.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or(aethel_ir::lower::IrType::Unit { span: f.span }),'
)

# Line 267-270: ty_ir type inference: ty.clone() gives Option<HirType>, need Option<IrType>
old = '''            let ty_ir = ty.clone().unwrap_or_else(|| {
                // Infer from init
                init_ir.as_ref().map(|e| e.ty().clone()).unwrap_or(aethel_ir::lower::IrType::Unit { span: *span })
            });'''
new = '''            let ty_ir = ty.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or_else(|| {
                // Infer from init
                aethel_ir::lower::IrType::Unit { span: *span }
            });'''
content = content.replace(old, new)

# Line 417: HirExpr::Let ty field - ty.clone() gives Option<HirType>, need IrType
old = "                ty: ty.clone().unwrap_or_else(|| aethel_ir::lower::IrType::Unit { span }),"
new = "                ty: ty.as_ref().map(|t| crate::types::lower_hir_type(t)).unwrap_or(aethel_ir::lower::IrType::Unit { span }),"
content = content.replace(old, new)

# Line 443: output_ty: output_ty.clone() -> lower from HirType
old = "                output_ty: output_ty.clone(),"
new = "                output_ty: crate::types::lower_hir_type(&output_ty),"
content = content.replace(old, new)

# Line 485: IrExpr::New ty: ty.clone()
old = "            ty: ty.clone(),"
new = "            ty: crate::types::lower_hir_type(&ty),"
content = content.replace(old, new)

# Line 552: IrStructField ty: f.ty.clone()
old = "            ty: f.ty.clone(),"
new = "            ty: crate::types::lower_hir_type(&f.ty),"
content = content.replace(old, new)

# Line 575-576: IrEnumField ty.clone() for HirEnumField variants
content = content.replace(
    'aethel_hir::lower::HirEnumField::Tuple { span, ty } => aethel_ir::lower::IrEnumField::Tuple { span: *span, ty: ty.clone() },',
    'aethel_hir::lower::HirEnumField::Tuple { span, ty } => aethel_ir::lower::IrEnumField::Tuple { span: *span, ty: crate::types::lower_hir_type(&ty) },'
)
content = content.replace(
    'aethel_hir::lower::HirEnumField::Named { span, name, ty } => aethel_ir::lower::IrEnumField::Named { span: *span, name: name.clone(), ty: ty.clone() },',
    'aethel_hir::lower::HirEnumField::Named { span, name, ty } => aethel_ir::lower::IrEnumField::Named { span: *span, name: name.clone(), ty: crate::types::lower_hir_type(&ty) },'
)

with open(path, 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixes applied")
