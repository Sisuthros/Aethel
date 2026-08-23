//! Expression checking.

use super::*;

impl SemanticChecker {
    pub(super) fn check_expr(&mut self, expr: &hir::HirExpr) -> ir::IrType {
        match expr {
            hir::HirExpr::Literal { span, lit } => match lit {
                hir::HirLiteral::Unit { .. } => ir::IrType::Unit { span: *span },
                hir::HirLiteral::Bool { .. } => ir::IrType::Bool { span: *span },
                hir::HirLiteral::Int { .. } => ir::IrType::Int { span: *span },
                hir::HirLiteral::Float { .. } => ir::IrType::Float { span: *span },
                hir::HirLiteral::String { .. } => ir::IrType::String { span: *span },
            },
            hir::HirExpr::Path { span, path } => {
                let name = expr_path_name(path);
                self.lookup(&name).unwrap_or_else(|| {
                    self.error(
                        codes::UNDEFINED_VAR(),
                        format!("unknown value `{name}`"),
                        *span,
                    );
                    ir::IrType::Unit { span: *span }
                })
            }
            hir::HirExpr::Tuple { span, exprs } => ir::IrType::Tuple {
                span: *span,
                types: exprs.iter().map(|expr| self.check_expr(expr)).collect(),
            },
            hir::HirExpr::Array { span, exprs } => {
                let types: Vec<_> = exprs.iter().map(|expr| self.check_expr(expr)).collect();
                let element = types
                    .first()
                    .cloned()
                    .unwrap_or(ir::IrType::Unit { span: *span });
                for actual in types.iter().skip(1) {
                    self.require_assignable(actual, &element, *span, "array element");
                }
                ir::IrType::Array {
                    span: *span,
                    ty: Box::new(element),
                    size: None,
                }
            }
            hir::HirExpr::Struct {
                span,
                path,
                fields,
                base,
            } => {
                let name = type_path_name(path);
                let expected = self.structs.get(&name).cloned();
                if let Some(expected) = expected {
                    for field in fields {
                        let actual = self.check_expr(&field.expr);
                        if let Some(field_ty) = expected.get(&field.name) {
                            self.require_assignable(&actual, field_ty, field.span, "struct field");
                        } else {
                            self.error(
                                codes::UNDEFINED_VAR(),
                                format!("unknown field `{}` on `{name}`", field.name),
                                field.span,
                            );
                        }
                    }
                } else {
                    self.error(
                        codes::UNDEFINED_TYPE(),
                        format!("unknown struct `{name}`"),
                        *span,
                    );
                }
                if let Some(base) = base {
                    self.check_expr(base);
                }
                ir::IrType::Path {
                    span: *span,
                    path: lower_type_path(path),
                }
            }
            hir::HirExpr::Call { span, callee, args } => self.check_call(*span, callee, args),
            hir::HirExpr::MethodCall {
                span,
                receiver,
                method,
                args,
            } => self.check_effect_call(*span, receiver, method, args),
            hir::HirExpr::Field { span, base, field } => {
                let checked_base = self.check_expr(base);
                let base_ty = self.expand_alias(&checked_base);
                if let ir::IrType::Path { path, .. } = base_ty {
                    let name = ir_path_name(&path);
                    if let Some(fields) = self.structs.get(&name) {
                        if let Some(ty) = fields.get(field) {
                            return ty.clone();
                        }
                    }
                }
                self.error(
                    codes::UNDEFINED_VAR(),
                    format!("unknown field `{field}`"),
                    *span,
                );
                ir::IrType::Unit { span: *span }
            }
            hir::HirExpr::Index { span, base, index } => {
                let base_ty = self.check_expr(base);
                let index_ty = self.check_expr(index);
                self.require_assignable(
                    &index_ty,
                    &ir::IrType::Int { span: *span },
                    expr_span(index),
                    "array index",
                );
                match base_ty {
                    ir::IrType::Array { ty, .. } => *ty,
                    _ => {
                        self.error(
                            codes::TYPE_MISMATCH(),
                            "cannot index non-array value",
                            *span,
                        );
                        ir::IrType::Unit { span: *span }
                    }
                }
            }
            hir::HirExpr::Unary { span, op, expr } => {
                let actual = self.check_expr(expr);
                match op {
                    hir::HirUnaryOp::Not => {
                        self.require_bool(&actual, *span);
                        ir::IrType::Bool { span: *span }
                    }
                    hir::HirUnaryOp::Neg => {
                        if !matches!(actual, ir::IrType::Int { .. } | ir::IrType::Float { .. }) {
                            self.error(
                                codes::UNSUPPORTED_TYPE_OP(),
                                "numeric negation requires int or float",
                                *span,
                            );
                        }
                        actual
                    }
                    hir::HirUnaryOp::Deref => match actual {
                        ir::IrType::Ref { ty, .. } => *ty,
                        _ => {
                            self.error(
                                codes::TYPE_MISMATCH(),
                                "cannot dereference non-reference",
                                *span,
                            );
                            ir::IrType::Unit { span: *span }
                        }
                    },
                }
            }
            hir::HirExpr::Binary {
                span,
                op,
                left,
                right,
            } => self.check_binary(*span, op, left, right),
            hir::HirExpr::If {
                span,
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.check_expr(cond);
                self.require_bool(&cond_ty, expr_span(cond));
                let then_ty = self.check_expr(then_branch);
                if let Some(else_branch) = else_branch {
                    let else_ty = self.check_expr(else_branch);
                    self.require_assignable(&else_ty, &then_ty, *span, "if branches");
                    then_ty
                } else {
                    ir::IrType::Unit { span: *span }
                }
            }
            hir::HirExpr::Match {
                span,
                scrutinee,
                arms,
            } => {
                let scrutinee_ty = self.check_expr(scrutinee);
                let mut result: Option<ir::IrType> = None;
                for arm in arms {
                    self.push_scope();
                    self.bind_pattern(&arm.pat, &scrutinee_ty);
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.check_expr(guard);
                        self.require_bool(&guard_ty, expr_span(guard));
                    }
                    let arm_ty = self.check_expr(&arm.body);
                    if let Some(expected) = &result {
                        self.require_assignable(&arm_ty, expected, arm.span, "match arm");
                    } else {
                        result = Some(arm_ty);
                    }
                    self.pop_scope();
                }
                result.unwrap_or(ir::IrType::Unit { span: *span })
            }
            hir::HirExpr::Block { span, block } => self
                .check_block(block)
                .unwrap_or(ir::IrType::Unit { span: *span }),
            hir::HirExpr::Let {
                span,
                pat,
                ty,
                init,
                ..
            } => {
                let actual = self.check_expr(init);
                let declared = ty.as_ref().map_or_else(|| actual.clone(), lower_type);
                self.require_assignable(&actual, &declared, *span, "let expression");
                self.bind_pattern(pat, &declared);
                ir::IrType::Unit { span: *span }
            }
            hir::HirExpr::Return { span, expr } => {
                let actual = expr
                    .as_ref()
                    .map_or(ir::IrType::Unit { span: *span }, |expr| {
                        self.check_expr(expr)
                    });
                if let Some(expected) = self.current_return.clone() {
                    self.require_assignable(&actual, &expected, *span, "return value");
                }
                ir::IrType::Never { span: *span }
            }
            hir::HirExpr::Break { span, expr } => {
                if let Some(expr) = expr {
                    self.check_expr(expr);
                }
                ir::IrType::Never { span: *span }
            }
            hir::HirExpr::Continue { span } => ir::IrType::Never { span: *span },
            hir::HirExpr::Ask {
                span,
                model,
                goal: _,
                input,
                output_ty,
            } => {
                // G4 budget reservation: the model position carries the Budget
                // capability token. It must be a live linear binding and is
                // consumed here — one token pays for exactly one dispatch.
                let budget_name = expr_path_name(model);
                let mut source_name = budget_name.clone();
                if let Some(source) = self.linear_aliases.get(&source_name) {
                    source_name = source.clone();
                }
                let is_live_budget = self.linear_params.iter().any(|(n, _)| *n == source_name)
                    && matches!(self.lookup(&budget_name), Some(ir::IrType::Budget { .. }));
                if !is_live_budget {
                    self.error(
                        codes::CAPABILITY_REQUIRED(),
                        format!(
                            "`ask` requires a live Budget token in its first position, found `{budget_name}`"
                        ),
                        *span,
                    );
                } else if !self.linear_consumed.insert(source_name) {
                    self.error(
                        codes::LINEAR_USE_AFTER_MOVE(),
                        format!(
                            "budget `{budget_name}` is already spent — a Budget token pays for exactly one `ask`"
                        ),
                        *span,
                    );
                }
                self.check_expr(input);
                self.validate_type(output_ty);
                ir::IrType::Claim {
                    span: *span,
                    ty: Box::new(lower_type(output_ty)),
                }
            }
            hir::HirExpr::Verify {
                span,
                claim,
                policy,
                evidence,
            } => self.check_verify(*span, claim, policy, evidence.as_ref()),
            hir::HirExpr::Reason { span, .. } => ir::IrType::Claim {
                span: *span,
                ty: Box::new(ir::IrType::String { span: *span }),
            },
            hir::HirExpr::CommitOnce { span, effect, args } => {
                let effect_name = type_path_name(&effect.path);
                if !self.effects.contains_key(&effect_name) {
                    self.error(
                        codes::UNDEFINED_EFFECT(),
                        format!("unknown effect `{effect_name}`"),
                        *span,
                    );
                } else if !self.current_effects.contains(&effect_name) {
                    self.error(
                        codes::EFFECT_NOT_DECLARED(),
                        format!("effect `{effect_name}` is not declared in `uses`"),
                        *span,
                    );
                }
                for arg in args {
                    let arg_ty = self.check_expr(arg);
                    if !matches!(arg_ty, ir::IrType::Verified { .. }) {
                        self.error(
                            codes::EPISTEMIC_UNVERIFIED_EFFECT(),
                            "commit_once arguments must be Verified values",
                            expr_span(arg),
                        );
                    }
                }
                ir::IrType::Unit { span: *span }
            }
            hir::HirExpr::New { span, ty, args } => {
                let result = lower_type(ty);
                if matches!(result, ir::IrType::Verified { .. }) {
                    self.error(
                        codes::EPISTEMIC_VERIFIED_REQUIRED(),
                        "Verified values can only be created with `verify`",
                        *span,
                    );
                }
                for arg in args {
                    self.check_expr(arg);
                }
                result
            }
        }
    }

    pub(super) fn check_call(
        &mut self,
        span: Span,
        callee: &hir::HirExpr,
        args: &[hir::HirExpr],
    ) -> ir::IrType {
        let name = match callee {
            hir::HirExpr::Path { path, .. } => expr_path_name(path),
            _ => {
                self.error(
                    codes::TYPE_MISMATCH(),
                    "callee must be a named function",
                    span,
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ir::IrType::Unit { span };
            }
        };
        // Built-in functions: reason and ask
        if name == "reason" {
            return self.check_builtin_reason(span, args);
        }
        if name == "ask" {
            return self.check_builtin_ask(span, args);
        }
        let actual: Vec<_> = args.iter().map(|arg| self.check_expr(arg)).collect();
        if let Some(signature) = self.functions.get(&name).cloned() {
            self.check_args(
                &actual,
                &signature.params,
                span,
                &format!("function `{name}`"),
            );
            signature.ret
        } else {
            self.error(
                codes::UNDEFINED_VAR(),
                format!("unknown function `{name}`"),
                span,
            );
            ir::IrType::Unit { span }
        }
    }

    pub(super) fn check_builtin_reason(&mut self, span: Span, args: &[hir::HirExpr]) -> ir::IrType {
        // reason(prompt: string) -> Claim<string>
        if args.len() != 1 {
            self.error(
                codes::TYPE_MISMATCH(),
                format!("reason expects 1 argument, received {}", args.len()),
                span,
            );
            return ir::IrType::Unit { span };
        }
        let prompt_ty = self.check_expr(&args[0]);
        self.require_assignable(
            &prompt_ty,
            &ir::IrType::String { span },
            span,
            "reason argument",
        );
        ir::IrType::Claim {
            span,
            ty: Box::new(ir::IrType::String { span }),
        }
    }

    pub(super) fn check_builtin_ask(&mut self, span: Span, args: &[hir::HirExpr]) -> ir::IrType {
        // ask(budget, input, OutputType) or ask(model, budget, input, OutputType)
        // The FIRST argument must be a live Budget token. `ask` consumes it
        // linearly: one token = one model dispatch (G4 budget reservation).
        if args.len() < 3 || args.len() > 4 {
            self.error(
                codes::CAPABILITY_REQUIRED(),
                format!(
                    "ask requires a Budget capability as its first argument (ask(budget, input, OutputType)), received {} argument(s)",
                    args.len()
                ),
                span,
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ir::IrType::Unit { span };
        }

        // Budget consumption: the first argument must be a path naming a live
        // Budget binding — a parameter or an alias of one.
        match args.first() {
            Some(hir::HirExpr::Path { path, .. }) => {
                let mut name = expr_path_name(path);
                if let Some(source) = self.linear_aliases.get(&name) {
                    name = source.clone();
                }
                let is_budget_param = self.linear_params.iter().any(|(n, _)| *n == name)
                    && matches!(
                        self.lookup(&expr_path_name(path)),
                        Some(ir::IrType::Budget { .. })
                    );
                if !is_budget_param {
                    self.error(
                        codes::CAPABILITY_REQUIRED(),
                        format!(
                            "`ask` requires a live Budget token as its first argument, found `{}`",
                            expr_path_name(path)
                        ),
                        expr_span(args.first().expect("arg checked above")),
                    );
                } else if !self.linear_consumed.insert(name) {
                    // Duplication guard: the same token already paid for a call.
                    self.error(
                        codes::LINEAR_USE_AFTER_MOVE(),
                        format!(
                            "budget `{}` is already spent — a Budget token pays for exactly one `ask`",
                            expr_path_name(path)
                        ),
                        expr_span(args.first().expect("arg checked above")),
                    );
                }
            }
            _ => {
                self.error(
                    codes::CAPABILITY_REQUIRED(),
                    "`ask` requires a live Budget token as its first argument",
                    args.first().map_or(span, expr_span),
                );
            }
        }

        // Check all provided arguments
        for arg in args {
            self.check_expr(arg);
        }
        // The last argument is always the output type - extract from HIR
        // The semantic type comes from the output_ty annotation
        // Default to Claim<string> since we can't extract the type path from parsed call
        ir::IrType::Claim {
            span,
            ty: Box::new(ir::IrType::String { span }),
        }
    }

    pub(super) fn check_effect_call(
        &mut self,
        span: Span,
        receiver: &hir::HirExpr,
        method: &str,
        args: &[hir::HirExpr],
    ) -> ir::IrType {
        let receiver_name = match receiver {
            hir::HirExpr::Path { path, .. } => expr_path_name(path),
            _ => String::new(),
        };
        let mut candidates = Vec::new();
        for effect_name in &self.current_effects {
            if let Some(operation) = self
                .effects
                .get(effect_name)
                .and_then(|operations| operations.get(method))
            {
                candidates.push((effect_name.clone(), operation.clone()));
            }
        }
        let selected = if candidates.len() == 1 {
            candidates.first().cloned()
        } else {
            let key = canonical(&receiver_name);
            candidates
                .iter()
                .find(|(effect, _)| alias_matches(&key, effect))
                .cloned()
        };
        let actual: Vec<_> = args.iter().map(|arg| self.check_expr(arg)).collect();
        if let Some((effect, operation)) = selected {
            self.check_args(
                &actual,
                &operation.params,
                span,
                &format!("effect `{effect}.{method}`"),
            );
            operation.ret
        } else {
            let (code, message) = if candidates.is_empty() {
                (
                    codes::EFFECT_NOT_DECLARED(),
                    format!("operation `{method}` is not available from declared effects"),
                )
            } else {
                (
                    codes::AMBIGUOUS_NAME(),
                    format!("ambiguous effect operation `{method}` for `{receiver_name}`"),
                )
            };
            self.error(code, message, span);
            ir::IrType::Unit { span }
        }
    }

    pub(super) fn check_verify(
        &mut self,
        span: Span,
        claim: &hir::HirExpr,
        policy: &hir::HirTypePath,
        evidence: Option<&hir::HirEvidenceKind>,
    ) -> ir::IrType {
        // Linear consumption: verifying a Claim path consumes it. The path may
        // name the parameter directly or an alias binding (`let x = c;`).
        if let hir::HirExpr::Path { path, .. } = claim {
            let mut name = expr_path_name(path);
            if let Some(source) = self.linear_aliases.get(&name) {
                name = source.clone();
            }
            if self.linear_params.iter().any(|(n, _)| *n == name) {
                // Duplication guard: a second verify of the same linear Claim
                // is a use-after-move (double charge under two policies).
                if !self.linear_consumed.insert(name.clone()) {
                    let span = expr_span(claim);
                    self.error(
                        codes::LINEAR_USE_AFTER_MOVE(),
                        format!(
                            "claim `{name}` is already consumed — a linear Claim can be verified only once"
                        ),
                        span,
                    );
                    return ir::IrType::Unit { span };
                }
            }
        }
        let claim_ty = self.check_expr(claim);
        let inner = match claim_ty {
            ir::IrType::Claim { ty, .. } => *ty,
            other => {
                self.error(
                    codes::EPISTEMIC_VERIFIED_REQUIRED(),
                    format!(
                        "verify requires Claim<T>, found `{}`",
                        self.format_type(&other)
                    ),
                    span,
                );
                ir::IrType::Unit { span }
            }
        };
        let policy_name = type_path_name(policy);
        if let Some(accepted) = self.policies.get(&policy_name).cloned() {
            if !accepted.iter().any(|ty| self.types_equal(ty, &inner)) {
                self.error(
                    codes::EPISTEMIC_VERIFY_FAILED(),
                    format!(
                        "policy `{policy_name}` does not accept Claim<{}>",
                        self.format_type(&inner)
                    ),
                    span,
                );
            }
            // Evidence-kind matching: the evidence `verify` provides must be
            // one of the kinds the policy claim requires. No third argument
            // means no evidence was provided.
            match self.policy_evidence.get(&policy_name) {
                Some(required_evidence) if !required_evidence.is_empty() => match evidence {
                    None => {
                        self.error(
                                codes::EPISTEMIC_VERIFY_FAILED(),
                                format!(
                                    "verification failed: policy `{policy_name}` requires evidence {:?} for claim, but verify() provides none",
                                    required_evidence
                                ),
                                span,
                            );
                    }
                    Some(provided_kind) => {
                        if !required_evidence
                            .iter()
                            .any(|required| hir_evidence_kinds_equal(required, provided_kind))
                        {
                            self.error(
                                    codes::EPISTEMIC_POLICY_MISMATCH(),
                                    format!(
                                        "policy `{policy_name}` requires evidence kind(s) {:?}, but verify() provides {:?}",
                                        required_evidence, provided_kind
                                    ),
                                    span,
                                );
                        }
                    }
                },
                _ => {}
            }
        } else {
            self.error(
                codes::UNDEFINED_TYPE(),
                format!("unknown verification policy `{policy_name}`"),
                span,
            );
        }
        ir::IrType::Verified {
            span,
            ty: Box::new(inner),
            policy: Box::new(ir::IrType::Path {
                span,
                path: lower_type_path(policy),
            }),
        }
    }

    pub(super) fn check_binary(
        &mut self,
        span: Span,
        op: &hir::HirBinaryOp,
        left: &hir::HirExpr,
        right: &hir::HirExpr,
    ) -> ir::IrType {
        let left = self.check_expr(left);
        let right = self.check_expr(right);
        match op {
            hir::HirBinaryOp::Eq
            | hir::HirBinaryOp::Ne
            | hir::HirBinaryOp::Lt
            | hir::HirBinaryOp::Le
            | hir::HirBinaryOp::Gt
            | hir::HirBinaryOp::Ge => {
                self.require_assignable(&right, &left, span, "comparison");
                ir::IrType::Bool { span }
            }
            hir::HirBinaryOp::And | hir::HirBinaryOp::Or => {
                self.require_bool(&left, span);
                self.require_bool(&right, span);
                ir::IrType::Bool { span }
            }
            hir::HirBinaryOp::Assign
            | hir::HirBinaryOp::AddAssign
            | hir::HirBinaryOp::SubAssign
            | hir::HirBinaryOp::MulAssign
            | hir::HirBinaryOp::DivAssign
            | hir::HirBinaryOp::RemAssign => {
                self.require_assignable(&right, &left, span, "assignment");
                ir::IrType::Unit { span }
            }
            _ => {
                self.require_assignable(&right, &left, span, "binary operation");
                if !matches!(left, ir::IrType::Int { .. } | ir::IrType::Float { .. }) {
                    self.error(
                        codes::UNSUPPORTED_TYPE_OP(),
                        "numeric operation requires int or float",
                        span,
                    );
                }
                left
            }
        }
    }
}

/// Structural equality for evidence kinds, including `Custom` names.
fn hir_evidence_kinds_equal(left: &hir::HirEvidenceKind, right: &hir::HirEvidenceKind) -> bool {
    use hir::HirEvidenceKind as K;
    match (left, right) {
        (K::SignedAttestation, K::SignedAttestation)
        | (K::CryptographicProof, K::CryptographicProof)
        | (K::AuditLog, K::AuditLog)
        | (K::HumanReview, K::HumanReview) => true,
        (K::Custom(a), K::Custom(b)) => a == b,
        _ => false,
    }
}
