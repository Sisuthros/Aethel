//! Assignability, type equality, scopes, and diagnostics.

use super::*;

impl SemanticChecker {
    pub(super) fn check_args(
        &mut self,
        actual: &[ir::IrType],
        expected: &[ir::IrType],
        span: Span,
        target: &str,
    ) {
        if actual.len() != expected.len() {
            self.error(
                codes::TYPE_MISMATCH(),
                format!(
                    "{target} expects {} argument(s), received {}",
                    expected.len(),
                    actual.len()
                ),
                span,
            );
        }
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            self.require_assignable(
                actual,
                expected,
                span,
                &format!("argument {} to {target}", index + 1),
            );
        }
    }

    pub(super) fn require_bool(&mut self, actual: &ir::IrType, span: Span) {
        self.require_assignable(actual, &ir::IrType::Bool { span }, span, "condition");
    }

    pub(super) fn require_assignable(
        &mut self,
        actual: &ir::IrType,
        expected: &ir::IrType,
        span: Span,
        context: &str,
    ) -> bool {
        if matches!(actual, ir::IrType::Never { .. }) || self.types_equal(actual, expected) {
            return true;
        }
        let code = match (actual, expected) {
            (ir::IrType::Claim { .. }, ir::IrType::Verified { .. }) => {
                codes::EPISTEMIC_CLAIM_NOT_VERIFIED()
            }
            (
                ir::IrType::Verified {
                    policy: actual_policy,
                    ..
                },
                ir::IrType::Verified {
                    policy: expected_policy,
                    ..
                },
            ) if !self.types_equal(actual_policy, expected_policy) => {
                codes::EPISTEMIC_POLICY_MISMATCH()
            }
            _ => codes::TYPE_MISMATCH(),
        };
        self.error(
            code,
            format!(
                "{context}: expected `{}`, found `{}`",
                self.format_type(expected),
                self.format_type(actual)
            ),
            span,
        );
        false
    }

    pub(super) fn types_equal(&self, left: &ir::IrType, right: &ir::IrType) -> bool {
        let left = self.expand_alias(left);
        let right = self.expand_alias(right);
        match (&left, &right) {
            (ir::IrType::Unit { .. }, ir::IrType::Unit { .. })
            | (ir::IrType::Never { .. }, ir::IrType::Never { .. })
            | (ir::IrType::Bool { .. }, ir::IrType::Bool { .. })
            | (ir::IrType::Int { .. }, ir::IrType::Int { .. })
            | (ir::IrType::Float { .. }, ir::IrType::Float { .. })
            | (ir::IrType::String { .. }, ir::IrType::String { .. }) => true,
            (ir::IrType::Path { path: left, .. }, ir::IrType::Path { path: right, .. }) => {
                ir_path_name(left) == ir_path_name(right)
            }
            (
                ir::IrType::Ref {
                    is_mut: left_mut,
                    ty: left,
                    ..
                },
                ir::IrType::Ref {
                    is_mut: right_mut,
                    ty: right,
                    ..
                },
            ) => left_mut == right_mut && self.types_equal(left, right),
            (ir::IrType::Owned { ty: left, .. }, ir::IrType::Owned { ty: right, .. })
            | (ir::IrType::Claim { ty: left, .. }, ir::IrType::Claim { ty: right, .. }) => {
                self.types_equal(left, right)
            }
            (
                ir::IrType::Verified {
                    ty: left_ty,
                    policy: left_policy,
                    ..
                },
                ir::IrType::Verified {
                    ty: right_ty,
                    policy: right_policy,
                    ..
                },
            ) => self.types_equal(left_ty, right_ty) && self.types_equal(left_policy, right_policy),
            (ir::IrType::Array { ty: left, .. }, ir::IrType::Array { ty: right, .. }) => {
                self.types_equal(left, right)
            }
            (ir::IrType::Tuple { types: left, .. }, ir::IrType::Tuple { types: right, .. }) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right)
                        .all(|(left, right)| self.types_equal(left, right))
            }
            (
                ir::IrType::Fn {
                    params: left_params,
                    ret: left_ret,
                    ..
                },
                ir::IrType::Fn {
                    params: right_params,
                    ret: right_ret,
                    ..
                },
            ) => {
                left_params.len() == right_params.len()
                    && left_params
                        .iter()
                        .zip(right_params)
                        .all(|(left, right)| self.types_equal(left, right))
                    && self.types_equal(left_ret, right_ret)
            }
            _ => false,
        }
    }

    pub(super) fn expand_alias(&self, ty: &ir::IrType) -> ir::IrType {
        if let ir::IrType::Path { path, .. } = ty {
            if let Some(alias) = self.aliases.get(&ir_path_name(path)) {
                return alias.clone();
            }
        }
        ty.clone()
    }

    pub(super) fn format_type(&self, ty: &ir::IrType) -> String {
        match ty {
            ir::IrType::Unit { .. } => "()".into(),
            ir::IrType::Never { .. } => "!".into(),
            ir::IrType::Bool { .. } => "bool".into(),
            ir::IrType::Int { .. } => "int".into(),
            ir::IrType::Float { .. } => "float".into(),
            ir::IrType::String { .. } => "string".into(),
            ir::IrType::Path { path, .. } => ir_path_name(path),
            ir::IrType::Budget { .. } => "Budget".into(),
            ir::IrType::Ref { is_mut, ty, .. } => {
                format!(
                    "&{}{}",
                    if *is_mut { "mut " } else { "" },
                    self.format_type(ty)
                )
            }
            ir::IrType::Owned { ty, .. } => format!("owned {}", self.format_type(ty)),
            ir::IrType::Claim { ty, .. } => format!("Claim<{}>", self.format_type(ty)),
            ir::IrType::Verified { ty, policy, .. } => format!(
                "Verified<{}, {}>",
                self.format_type(ty),
                self.format_type(policy)
            ),
            ir::IrType::Array { ty, .. } => format!("[{}]", self.format_type(ty)),
            ir::IrType::Tuple { types, .. } => format!(
                "({})",
                types
                    .iter()
                    .map(|ty| self.format_type(ty))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            ir::IrType::Fn { params, ret, .. } => format!(
                "fn({}) -> {}",
                params
                    .iter()
                    .map(|ty| self.format_type(ty))
                    .collect::<Vec<_>>()
                    .join(", "),
                self.format_type(ret)
            ),
        }
    }

    pub(super) fn bind_pattern(&mut self, pat: &hir::HirPat, ty: &ir::IrType) {
        match pat {
            hir::HirPat::Ident { name, span, .. } => {
                self.bind(name.clone(), ty.clone(), *span);
            }
            hir::HirPat::Tuple { pats, span } => {
                let types = match ty {
                    ir::IrType::Tuple { types, .. } => types.clone(),
                    _ => {
                        self.error(
                            codes::TYPE_MISMATCH(),
                            "tuple pattern requires tuple",
                            *span,
                        );
                        vec![ir::IrType::Unit { span: *span }; pats.len()]
                    }
                };
                for (index, pat) in pats.iter().enumerate() {
                    let field_ty = types
                        .get(index)
                        .cloned()
                        .unwrap_or(ir::IrType::Unit { span: *span });
                    self.bind_pattern(pat, &field_ty);
                }
            }
            hir::HirPat::Struct { fields, .. } => {
                for field in fields {
                    if let Some(pat) = &field.pat {
                        let field_ty = ir::IrType::Unit { span: field.span };
                        self.bind_pattern(pat, &field_ty);
                    }
                }
            }
            hir::HirPat::Enum { fields, span, .. } | hir::HirPat::Or { pats: fields, span } => {
                for field in fields {
                    let field_ty = ir::IrType::Unit { span: *span };
                    self.bind_pattern(field, &field_ty);
                }
            }
            hir::HirPat::Ref { pat, .. } => self.bind_pattern(pat, ty),
            hir::HirPat::Wild { .. } | hir::HirPat::Literal { .. } => {}
        }
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub(super) fn bind(&mut self, name: String, ty: ir::IrType, span: Span) {
        let duplicate = self
            .scopes
            .last()
            .is_some_and(|scope| scope.contains_key(&name));
        if duplicate {
            self.error(
                codes::SHADOWED_NAME(),
                format!("duplicate binding `{name}` in the same scope"),
                span,
            );
        }
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    pub(super) fn lookup(&self, name: &str) -> Option<ir::IrType> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    pub(super) fn error(&mut self, code: DiagnosticCode, message: impl Into<String>, span: Span) {
        self.diagnostics.push(
            DiagnosticBuilder::error(code, message)
                .primary_label(span, "here")
                .build(),
        );
    }
}
