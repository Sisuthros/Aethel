//! Semantic validation over resolved HIR.

#![allow(clippy::all, clippy::pedantic, clippy::nursery)]

use std::collections::{HashMap, HashSet};

use aethel_hir::lower as hir;
use aethel_ir::lower as ir;
use aethel_syntax::diagnostic::{codes, DiagnosticBuilder, DiagnosticCode, Diagnostics};
use aethel_syntax::span::Span;

use super::util::{
    alias_matches, canonical, expr_path_name, expr_span, hir_type_name, ir_path_name, lower_type,
    lower_type_path, type_path_name,
};

// Import HirEvidenceKind for policy evidence tracking
use aethel_hir::lower::HirEvidenceKind;

mod expr;
mod types;

#[derive(Clone)]
struct OperationSig {
    params: Vec<ir::IrType>,
    ret: ir::IrType,
}

#[derive(Clone)]
struct FunctionSig {
    params: Vec<ir::IrType>,
    ret: ir::IrType,
}

#[derive(Default)]
pub(super) struct SemanticChecker {
    diagnostics: Diagnostics,
    effects: HashMap<String, HashMap<String, OperationSig>>,
    policies: HashMap<String, Vec<ir::IrType>>,
    // Track evidence kinds required by each policy's claims
    policy_evidence: HashMap<String, Vec<HirEvidenceKind>>,
    functions: HashMap<String, FunctionSig>,
    structs: HashMap<String, HashMap<String, ir::IrType>>,
    aliases: HashMap<String, ir::IrType>,
    known_types: HashSet<String>,
    scopes: Vec<HashMap<String, ir::IrType>>,
    current_return: Option<ir::IrType>,
    current_effects: Vec<String>,
    /// Names of Claim-typed parameters in the current function that must be consumed.
    linear_params: Vec<(String, Span)>,
    /// Names of Claim-typed parameters already consumed via `verify` in the current function.
    linear_consumed: HashSet<String>,
    /// Bindings that alias a linear Claim parameter (`let x = c;` maps x → c),
    /// so consumption through the alias reaches the parameter's account.
    linear_aliases: HashMap<String, String>,
}

impl SemanticChecker {
    pub(super) fn new(module: &hir::HirModule) -> Self {
        let mut checker = Self::default();
        checker.collect(module);
        checker
    }

    pub(super) fn collect(&mut self, module: &hir::HirModule) {
        for item in &module.items {
            match item {
                hir::HirItem::Struct(def) => {
                    self.known_types.insert(def.name.clone());
                    self.structs.insert(
                        def.name.clone(),
                        def.fields
                            .iter()
                            .map(|field| (field.name.clone(), lower_type(&field.ty)))
                            .collect(),
                    );
                }
                hir::HirItem::Enum(def) => {
                    self.known_types.insert(def.name.clone());
                }
                hir::HirItem::TypeAlias(def) => {
                    self.known_types.insert(def.name.clone());
                    self.aliases.insert(def.name.clone(), lower_type(&def.ty));
                }
                hir::HirItem::Policy(def) => {
                    self.known_types.insert(def.name.clone());
                    self.policies.insert(
                        def.name.clone(),
                        def.claims
                            .iter()
                            .map(|claim| lower_type(&claim.ty))
                            .collect(),
                    );
                    // Track evidence kinds required by this policy's claims
                    let evidence_kinds: Vec<HirEvidenceKind> = def
                        .claims
                        .iter()
                        .flat_map(|claim| claim.evidence.iter().map(|ev| ev.kind.clone()))
                        .collect();
                    if !evidence_kinds.is_empty() {
                        self.policy_evidence
                            .insert(def.name.clone(), evidence_kinds);
                    }
                }
                hir::HirItem::Effect(def) => {
                    self.effects.insert(
                        def.name.clone(),
                        def.operations
                            .iter()
                            .map(|operation| {
                                (
                                    operation.name.clone(),
                                    OperationSig {
                                        params: operation
                                            .params
                                            .iter()
                                            .map(|param| lower_type(&param.ty))
                                            .collect(),
                                        ret: operation.ret_type.as_ref().map_or(
                                            ir::IrType::Unit {
                                                span: operation.span,
                                            },
                                            lower_type,
                                        ),
                                    },
                                )
                            })
                            .collect(),
                    );
                }
                hir::HirItem::Fn(def) => {
                    self.functions.insert(
                        def.name.clone(),
                        FunctionSig {
                            params: def
                                .params
                                .iter()
                                .map(|param| lower_type(&param.ty))
                                .collect(),
                            ret: def
                                .ret_type
                                .as_ref()
                                .map_or(ir::IrType::Unit { span: def.span }, lower_type),
                        },
                    );
                }
                hir::HirItem::Mod(def) => {
                    if let Some(body) = &def.body {
                        self.collect(body);
                    }
                }
                hir::HirItem::Use(_) => {}
            }
        }
    }

    pub(super) fn check(mut self, module: &hir::HirModule) -> Diagnostics {
        self.validate_declarations(module);
        for item in &module.items {
            if let hir::HirItem::Fn(def) = item {
                self.check_fn(def);
            }
        }
        self.diagnostics
    }

    pub(super) fn validate_declarations(&mut self, module: &hir::HirModule) {
        for item in &module.items {
            match item {
                hir::HirItem::Fn(def) => {
                    for param in &def.params {
                        self.validate_type(&param.ty);
                    }
                    if let Some(ret) = &def.ret_type {
                        self.validate_type(ret);
                    }
                    for effect in &def.effects.effects {
                        let name = type_path_name(&effect.path);
                        if !self.effects.contains_key(&name) {
                            self.error(
                                codes::UNDEFINED_EFFECT(),
                                format!("unknown effect `{name}` in `uses` clause"),
                                effect.span,
                            );
                        }
                    }
                }
                hir::HirItem::Struct(def) => {
                    for field in &def.fields {
                        self.validate_type(&field.ty);
                    }
                }
                hir::HirItem::Enum(def) => {
                    for variant in &def.variants {
                        for field in &variant.fields {
                            match field {
                                hir::HirEnumField::Tuple { ty, .. }
                                | hir::HirEnumField::Named { ty, .. } => self.validate_type(ty),
                            }
                        }
                    }
                }
                hir::HirItem::TypeAlias(def) => self.validate_type(&def.ty),
                hir::HirItem::Policy(def) => {
                    for claim in &def.claims {
                        self.validate_type(&claim.ty);
                    }
                }
                hir::HirItem::Effect(def) => {
                    for operation in &def.operations {
                        for param in &operation.params {
                            self.validate_type(&param.ty);
                        }
                        if let Some(ret) = &operation.ret_type {
                            self.validate_type(ret);
                        }
                    }
                }
                hir::HirItem::Mod(def) => {
                    if let Some(body) = &def.body {
                        self.validate_declarations(body);
                    }
                }
                hir::HirItem::Use(_) => {}
            }
        }
    }

    pub(super) fn validate_type(&mut self, ty: &hir::HirType) {
        match ty {
            hir::HirType::Path { span, path } => {
                let name = type_path_name(path);
                if !self.is_known_type(&name) {
                    self.error(
                        codes::UNDEFINED_TYPE(),
                        format!("unknown type `{name}`"),
                        *span,
                    );
                }
            }
            hir::HirType::Ref { ty, .. }
            | hir::HirType::Owned { ty, .. }
            | hir::HirType::Claim { ty, .. }
            | hir::HirType::Array { ty, .. } => self.validate_type(ty),
            hir::HirType::Verified { span, ty, policy } => {
                self.validate_type(ty);
                self.validate_type(policy);
                if let Some(name) = hir_type_name(policy) {
                    if !self.policies.contains_key(&name) {
                        self.error(
                            codes::UNDEFINED_TYPE(),
                            format!("unknown verification policy `{name}`"),
                            *span,
                        );
                    }
                }
            }
            // A Budget token is a built-in capability type — nothing to resolve.
            hir::HirType::Budget { .. } => {}
            hir::HirType::Tuple { types, .. } => {
                for inner in types {
                    self.validate_type(inner);
                }
            }
            hir::HirType::Fn {
                params,
                ret,
                effects,
                ..
            } => {
                for param in params {
                    self.validate_type(param);
                }
                self.validate_type(ret);
                for effect in &effects.effects {
                    let name = type_path_name(&effect.path);
                    if !self.effects.contains_key(&name) {
                        self.error(
                            codes::UNDEFINED_EFFECT(),
                            format!("unknown effect `{name}`"),
                            effect.span,
                        );
                    }
                }
            }
            hir::HirType::Unit { .. }
            | hir::HirType::Never { .. }
            | hir::HirType::Bool { .. }
            | hir::HirType::Int { .. }
            | hir::HirType::Float { .. }
            | hir::HirType::String { .. } => {}
        }
    }

    pub(super) fn is_known_type(&self, name: &str) -> bool {
        matches!(name, "bool" | "int" | "float" | "string")
            || self.known_types.contains(name)
            || self.policies.contains_key(name)
    }

    pub(super) fn check_fn(&mut self, def: &hir::HirFnDef) {
        let old_return = self.current_return.clone();
        let old_effects = self.current_effects.clone();
        let old_linear_params = std::mem::take(&mut self.linear_params);
        let old_linear_consumed = std::mem::take(&mut self.linear_consumed);
        let old_linear_aliases = std::mem::take(&mut self.linear_aliases);
        self.current_return = Some(
            def.ret_type
                .as_ref()
                .map_or(ir::IrType::Unit { span: def.span }, lower_type),
        );
        self.current_effects = def
            .effects
            .effects
            .iter()
            .map(|effect| type_path_name(&effect.path))
            .collect();

        // Linear types: Claim- and Budget-typed parameters must be consumed
        // before the function body ends. A Claim is consumed by `verify`,
        // a Budget by `ask`.
        for param in &def.params {
            if matches!(
                lower_type(&param.ty),
                ir::IrType::Claim { .. } | ir::IrType::Budget { .. }
            ) {
                self.linear_params.push((param.name.clone(), param.span));
            }
        }

        self.push_scope();
        for param in &def.params {
            self.bind(param.name.clone(), lower_type(&param.ty), param.span);
        }
        if let Some(body) = &def.body {
            let tail = self.check_block(body);
            if let (Some(actual), Some(expected)) = (tail, self.current_return.clone()) {
                self.require_assignable(&actual, &expected, body.span, "function tail expression");
            }
        }
        self.pop_scope();

        // Linear consumption check: every Claim-typed parameter must have been
        // consumed by `verify` before the function body ends.
        let unconsumed: Vec<(String, Span)> = self
            .linear_params
            .iter()
            .filter(|(name, _)| !self.linear_consumed.contains(name))
            .cloned()
            .collect();
        for (name, span) in unconsumed {
            self.error(
                codes::LINEAR_NOT_CONSUMED(),
                format!(
                    "claim parameter `{name}` is never verified — linear Claim values must be consumed with `verify`"
                ),
                span,
            );
        }

        self.linear_params = old_linear_params;
        self.linear_consumed = old_linear_consumed;
        self.linear_aliases = old_linear_aliases;
        self.current_return = old_return;
        self.current_effects = old_effects;
    }

    pub(super) fn check_block(&mut self, block: &hir::HirBlock) -> Option<ir::IrType> {
        self.push_scope();
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
        let tail = block.tail.as_ref().map(|expr| self.check_expr(expr));
        self.pop_scope();
        tail
    }

    pub(super) fn check_stmt(&mut self, stmt: &hir::HirStmt) {
        match stmt {
            hir::HirStmt::Let {
                span,
                name,
                ty,
                init,
                ..
            } => {
                let actual = init.as_ref().map(|expr| self.check_expr(expr));
                let declared = ty.as_ref().map(lower_type);
                let binding_ty = match (declared, actual) {
                    (Some(expected), Some(actual)) => {
                        self.require_assignable(&actual, &expected, *span, "let binding");
                        expected
                    }
                    (Some(expected), None) => {
                        // Origin enforcement (docs/guarantees.md G5): a bare
                        // declaration is not a constructor. A `Verified<_, _>`
                        // binding without an initialiser never passed through
                        // `verify`, so it must not be treated as verified.
                        if matches!(expected, ir::IrType::Verified { .. }) {
                            self.error(
                                codes::EPISTEMIC_VERIFIED_REQUIRED(),
                                format!(
                                    "binding `{name}` declares `Verified` without an initialiser — only `verify(claim, policy)` may produce a Verified value"
                                ),
                                *span,
                            );
                        }
                        expected
                    }
                    (None, Some(actual)) => actual,
                    (None, None) => {
                        self.error(
                            codes::TYPE_ANNOTATION_REQUIRED(),
                            format!("binding `{name}` needs a type or initializer"),
                            *span,
                        );
                        ir::IrType::Unit { span: *span }
                    }
                };
                // Linear bookkeeping: a bare `let x = c;` where `c` is a
                // linear Claim parameter aliases it. Consumption through the
                // alias must reach the parameter's account.
                if let ir::IrType::Claim { .. } = &binding_ty {
                    if let Some(hir::HirExpr::Path { path, .. }) = init.as_ref() {
                        let source = expr_path_name(path);
                        if self.linear_params.iter().any(|(n, _)| *n == source) {
                            self.linear_aliases.insert(name.clone(), source);
                        }
                    }
                }
                self.bind(name.clone(), binding_ty, *span);
            }
            hir::HirStmt::Expr { expr, .. } => {
                self.check_expr(expr);
            }
            hir::HirStmt::Return { span, expr } => {
                let actual = expr
                    .as_ref()
                    .map_or(ir::IrType::Unit { span: *span }, |expr| {
                        self.check_expr(expr)
                    });
                if let Some(expected) = self.current_return.clone() {
                    self.require_assignable(&actual, &expected, *span, "return value");
                }
            }
            hir::HirStmt::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                let cond_ty = self.check_expr(cond);
                self.require_bool(&cond_ty, expr_span(cond));
                self.check_block(then_branch);
                if let Some(stmt) = else_branch {
                    self.check_stmt(stmt);
                }
            }
            hir::HirStmt::While { cond, body, .. } => {
                let cond_ty = self.check_expr(cond);
                self.require_bool(&cond_ty, expr_span(cond));
                self.check_block(body);
            }
            hir::HirStmt::For {
                pat, iter, body, ..
            } => {
                let iter_ty = self.check_expr(iter);
                let element = match iter_ty {
                    ir::IrType::Array { ty, .. } => *ty,
                    _ => {
                        self.error(
                            codes::TYPE_MISMATCH(),
                            "for-loop requires an array",
                            expr_span(iter),
                        );
                        ir::IrType::Unit {
                            span: expr_span(iter),
                        }
                    }
                };
                self.push_scope();
                self.bind_pattern(pat, &element);
                self.check_block(body);
                self.pop_scope();
            }
            hir::HirStmt::Match {
                scrutinee, arms, ..
            } => {
                let scrutinee_ty = self.check_expr(scrutinee);
                for arm in arms {
                    self.push_scope();
                    self.bind_pattern(&arm.pat, &scrutinee_ty);
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.check_expr(guard);
                        self.require_bool(&guard_ty, expr_span(guard));
                    }
                    self.check_expr(&arm.body);
                    self.pop_scope();
                }
            }
            hir::HirStmt::Block { block, .. } => {
                self.check_block(block);
            }
        }
    }
}
