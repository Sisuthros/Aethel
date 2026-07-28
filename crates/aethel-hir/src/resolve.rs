//! Name resolution for HIR.

use crate::lower::*;
use indexmap::IndexMap;
use std::collections::HashMap;

/// Symbol table for name resolution.
#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    pub types: IndexMap<String, TypeSymbol>,
    pub values: IndexMap<String, ValueSymbol>,
    pub effects: IndexMap<String, EffectSymbol>,
    pub modules: IndexMap<String, ModuleSymbol>,
    pub scopes: Vec<Scope>,
}

#[derive(Debug, Clone)]
pub enum TypeSymbol {
    Struct(HirStructDef),
    Enum(HirEnumDef),
    TypeAlias(HirTypeAlias),
    Policy(HirPolicyDef),
    Generic {
        name: String,
        bounds: Vec<HirTypeBound>,
    },
}

#[derive(Debug, Clone)]
pub enum ValueSymbol {
    Fn(HirFnDef),
    Const {
        name: String,
        ty: HirType,
        init: Option<HirExpr>,
    },
    Let {
        name: String,
        ty: Option<HirType>,
        is_mut: bool,
    },
    Param {
        name: String,
        ty: HirType,
        is_mut: bool,
    },
}

#[derive(Debug, Clone)]
pub struct EffectSymbol {
    pub name: String,
    pub operations: Vec<EffectOperation>,
}

#[derive(Debug, Clone)]
pub struct EffectOperation {
    pub name: String,
    pub params: Vec<HirParam>,
    pub ret_type: Option<HirType>,
}

#[derive(Debug, Clone)]
pub struct ModuleSymbol {
    pub name: String,
    pub items: SymbolTable,
}
/// A scope in the symbol table.
#[derive(Debug, Default, Clone)]
pub struct Scope {
    pub type_names: Vec<String>,
    pub value_names: Vec<String>,
    pub effect_names: Vec<String>,
    pub parent: Option<usize>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self::default();
        table.enter_scope();
        table.add_prelude();
        table
    }

    fn add_prelude(&mut self) {
        // Add built-in types
        self.types.insert(
            "bool".into(),
            TypeSymbol::Generic {
                name: "bool".into(),
                bounds: Vec::new(),
            },
        );
        self.types.insert(
            "int".into(),
            TypeSymbol::Generic {
                name: "int".into(),
                bounds: Vec::new(),
            },
        );
        self.types.insert(
            "float".into(),
            TypeSymbol::Generic {
                name: "float".into(),
                bounds: Vec::new(),
            },
        );
        self.types.insert(
            "string".into(),
            TypeSymbol::Generic {
                name: "string".into(),
                bounds: Vec::new(),
            },
        );
        self.types.insert(
            "Claim".into(),
            TypeSymbol::Generic {
                name: "Claim".into(),
                bounds: Vec::new(),
            },
        );
        self.types.insert(
            "Verified".into(),
            TypeSymbol::Generic {
                name: "Verified".into(),
                bounds: Vec::new(),
            },
        );
    }

    pub fn enter_scope(&mut self) {
        let parent = if self.scopes.is_empty() {
            None
        } else {
            Some(self.scopes.len() - 1)
        };
        self.scopes.push(Scope {
            parent,
            ..Default::default()
        });
    }

    pub fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            for name in scope.type_names {
                self.types.shift_remove(&name);
            }
            for name in scope.value_names {
                self.values.shift_remove(&name);
            }
            for name in scope.effect_names {
                self.effects.shift_remove(&name);
            }
        }
    }

    pub fn add_type(&mut self, name: String, symbol: TypeSymbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.type_names.push(name.clone());
        }
        self.types.insert(name, symbol);
    }

    pub fn add_value(&mut self, name: String, symbol: ValueSymbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.value_names.push(name.clone());
        }
        self.values.insert(name, symbol);
    }

    pub fn add_effect(&mut self, name: String, symbol: EffectSymbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.effect_names.push(name.clone());
        }
        self.effects.insert(name, symbol);
    }

    pub fn add_module(&mut self, name: String, symbol: ModuleSymbol) {
        self.modules.insert(name, symbol);
    }

    pub fn resolve_type(&self, name: &str) -> Option<&TypeSymbol> {
        self.types.get(name)
    }

    pub fn resolve_value(&self, name: &str) -> Option<&ValueSymbol> {
        self.values.get(name)
    }

    pub fn resolve_effect(&self, name: &str) -> Option<&EffectSymbol> {
        self.effects.get(name)
    }
}

/// Resolve names in a HIR module.
pub fn resolve_module(module: &mut HirModule) -> Vec<String> {
    let mut table = SymbolTable::new();
    let mut errors = Vec::new();

    // First pass: collect all top-level definitions
    for item in &module.items {
        collect_item(&mut table, item, &mut errors);
    }

    // Second pass: resolve bodies
    for item in &mut module.items {
        resolve_item(&mut table, item, &mut errors);
    }

    errors
}

fn collect_item(table: &mut SymbolTable, item: &HirItem, errors: &mut Vec<String>) {
    match item {
        HirItem::Fn(f) => {
            table.add_value(f.name.clone(), ValueSymbol::Fn(f.clone()));
        }
        HirItem::Struct(s) => {
            table.add_type(s.name.clone(), TypeSymbol::Struct(s.clone()));
        }
        HirItem::Enum(e) => {
            table.add_type(e.name.clone(), TypeSymbol::Enum(e.clone()));
        }
        HirItem::TypeAlias(t) => {
            table.add_type(t.name.clone(), TypeSymbol::TypeAlias(t.clone()));
        }
        HirItem::Policy(p) => {
            table.add_type(p.name.clone(), TypeSymbol::Policy(p.clone()));
        }
        HirItem::Effect(e) => {
            // Effects are registered for boundary checks (simplified for v0.1)
            table.add_effect(
                e.name.clone(),
                EffectSymbol {
                    name: e.name.clone(),
                    operations: vec![], // populated in full lowering
                },
            );
        }
        HirItem::Use(u) => {
            // Use declarations don't add to symbol table directly
        }
        HirItem::Mod(m) => {
            // Modules create nested namespaces
        }
    }
}

fn resolve_item(table: &mut SymbolTable, item: &mut HirItem, errors: &mut Vec<String>) {
    match item {
        HirItem::Fn(f) => {
            resolve_fn(table, f, errors);
        }
        HirItem::Struct(s) => {
            resolve_struct(table, s, errors);
        }
        HirItem::Enum(e) => {
            resolve_enum(table, e, errors);
        }
        HirItem::TypeAlias(t) => {
            resolve_type_alias(table, t, errors);
        }
        HirItem::Policy(p) => {
            resolve_policy(table, p, errors);
        }
        HirItem::Mod(m) => {
            if let Some(body) = &mut m.body {
                resolve_module(body);
            }
        }
        _ => {}
    }
}

fn resolve_fn(table: &mut SymbolTable, f: &mut HirFnDef, errors: &mut Vec<String>) {
    table.enter_scope();
    for param in &f.params {
        table.add_value(
            param.name.clone(),
            ValueSymbol::Param {
                name: param.name.clone(),
                ty: param.ty.clone(),
                is_mut: param.is_mut,
            },
        );
    }
    if let Some(body) = &mut f.body {
        resolve_block(table, body, errors);
    }
    table.exit_scope();
}

fn resolve_struct(table: &mut SymbolTable, s: &mut HirStructDef, errors: &mut Vec<String>) {
    table.enter_scope();
    for param in &s.generics {
        table.add_type(
            param.name.clone(),
            TypeSymbol::Generic {
                name: param.name.clone(),
                bounds: param.bounds.clone(),
            },
        );
    }
    for field in &mut s.fields {
        resolve_type(table, &mut field.ty, errors);
    }
    table.exit_scope();
}

fn resolve_enum(table: &mut SymbolTable, e: &mut HirEnumDef, errors: &mut Vec<String>) {
    table.enter_scope();
    for param in &e.generics {
        table.add_type(
            param.name.clone(),
            TypeSymbol::Generic {
                name: param.name.clone(),
                bounds: param.bounds.clone(),
            },
        );
    }
    for variant in &mut e.variants {
        for field in &mut variant.fields {
            match field {
                HirEnumField::Tuple { ty, .. } => resolve_type(table, ty, errors),
                HirEnumField::Named { ty, .. } => resolve_type(table, ty, errors),
            }
        }
    }
    table.exit_scope();
}

fn resolve_type_alias(table: &mut SymbolTable, t: &mut HirTypeAlias, errors: &mut Vec<String>) {
    table.enter_scope();
    for param in &t.generics {
        table.add_type(
            param.name.clone(),
            TypeSymbol::Generic {
                name: param.name.clone(),
                bounds: param.bounds.clone(),
            },
        );
    }
    resolve_type(table, &mut t.ty, errors);
    table.exit_scope();
}

fn resolve_policy(table: &mut SymbolTable, p: &mut HirPolicyDef, errors: &mut Vec<String>) {
    table.enter_scope();
    for param in &p.generics {
        table.add_type(
            param.name.clone(),
            TypeSymbol::Generic {
                name: param.name.clone(),
                bounds: param.bounds.clone(),
            },
        );
    }
    for claim in &mut p.claims {
        resolve_type(table, &mut claim.ty, errors);
        for ev in &claim.evidence {
            // Evidence kinds don't need resolution
        }
    }
    table.exit_scope();
}

fn resolve_block(table: &mut SymbolTable, block: &mut HirBlock, errors: &mut Vec<String>) {
    table.enter_scope();
    for stmt in &mut block.stmts {
        resolve_stmt(table, stmt, errors);
    }
    if let Some(tail) = &mut block.tail {
        resolve_expr(table, tail, errors);
    }
    table.exit_scope();
}

fn resolve_stmt(table: &mut SymbolTable, stmt: &mut HirStmt, errors: &mut Vec<String>) {
    match stmt {
        HirStmt::Let {
            name,
            ty,
            init,
            is_mut,
            ..
        } => {
            if let Some(ty) = ty {
                resolve_type(table, ty, errors);
            }
            if let Some(init) = init {
                resolve_expr(table, init, errors);
            }
            table.add_value(
                name.clone(),
                ValueSymbol::Let {
                    name: name.clone(),
                    ty: ty.clone(),
                    is_mut: *is_mut,
                },
            );
        }
        HirStmt::Expr { expr, .. } => {
            resolve_expr(table, expr, errors);
        }
        HirStmt::Return { expr, .. } => {
            if let Some(expr) = expr {
                resolve_expr(table, expr, errors);
            }
        }
        HirStmt::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            resolve_expr(table, cond, errors);
            resolve_block(table, then_branch, errors);
            if let Some(else_branch) = else_branch {
                resolve_stmt(table, else_branch, errors);
            }
        }
        HirStmt::While { cond, body, .. } => {
            resolve_expr(table, cond, errors);
            resolve_block(table, body, errors);
        }
        HirStmt::For {
            pat, iter, body, ..
        } => {
            resolve_pat(table, pat, errors);
            resolve_expr(table, iter, errors);
            resolve_block(table, body, errors);
        }
        HirStmt::Match {
            scrutinee, arms, ..
        } => {
            resolve_expr(table, scrutinee, errors);
            for arm in arms {
                resolve_pat(table, &mut arm.pat, errors);
                if let Some(guard) = &mut arm.guard {
                    resolve_expr(table, guard, errors);
                }
                resolve_expr(table, &mut arm.body, errors);
            }
        }
        HirStmt::Block { block, .. } => {
            resolve_block(table, block, errors);
        }
    }
}

fn resolve_expr(table: &mut SymbolTable, expr: &mut HirExpr, errors: &mut Vec<String>) {
    match expr {
        HirExpr::Literal { .. } => {}
        HirExpr::Path { path, .. } => {
            resolve_expr_path(table, path, errors);
        }
        HirExpr::Tuple { exprs, .. } => {
            for e in exprs {
                resolve_expr(table, e, errors);
            }
        }
        HirExpr::Array { exprs, .. } => {
            for e in exprs {
                resolve_expr(table, e, errors);
            }
        }
        HirExpr::Struct {
            path, fields, base, ..
        } => {
            resolve_type_path(table, path, errors);
            for f in fields {
                resolve_expr(table, &mut f.expr, errors);
            }
            if let Some(base) = base {
                resolve_expr(table, base, errors);
            }
        }
        HirExpr::Call { callee, args, .. } => {
            resolve_expr(table, callee, errors);
            for arg in args {
                resolve_expr(table, arg, errors);
            }
        }
        HirExpr::MethodCall { receiver, args, .. } => {
            resolve_expr(table, receiver, errors);
            for arg in args {
                resolve_expr(table, arg, errors);
            }
        }
        HirExpr::Field { base, .. } => {
            resolve_expr(table, base, errors);
        }
        HirExpr::Index { base, index, .. } => {
            resolve_expr(table, base, errors);
            resolve_expr(table, index, errors);
        }
        HirExpr::Unary { expr, .. } => {
            resolve_expr(table, expr, errors);
        }
        HirExpr::Binary { left, right, .. } => {
            resolve_expr(table, left, errors);
            resolve_expr(table, right, errors);
        }
        HirExpr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            resolve_expr(table, cond, errors);
            resolve_expr(table, then_branch, errors);
            if let Some(else_branch) = else_branch {
                resolve_expr(table, else_branch, errors);
            }
        }
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            resolve_expr(table, scrutinee, errors);
            for arm in arms {
                resolve_pat(table, &mut arm.pat, errors);
                if let Some(guard) = &mut arm.guard {
                    resolve_expr(table, guard, errors);
                }
                resolve_expr(table, &mut arm.body, errors);
            }
        }
        HirExpr::Block { block, .. } => {
            resolve_block(table, block, errors);
        }
        HirExpr::Let { pat, ty, init, .. } => {
            resolve_pat(table, pat, errors);
            if let Some(ty) = ty {
                resolve_type(table, ty, errors);
            }
            resolve_expr(table, init, errors);
        }
        HirExpr::Return { expr, .. } => {
            if let Some(expr) = expr {
                resolve_expr(table, expr, errors);
            }
        }
        HirExpr::Break { expr, .. } => {
            if let Some(expr) = expr {
                resolve_expr(table, expr, errors);
            }
        }
        HirExpr::Ask {
            model,
            input,
            output_ty,
            ..
        } => {
            resolve_expr_path(table, model, errors);
            resolve_expr(table, input, errors);
            resolve_type(table, output_ty, errors);
        }
        HirExpr::Verify { claim, policy, .. } => {
            resolve_expr(table, claim, errors);
            resolve_type_path(table, policy, errors);
        }
        HirExpr::Reason { prompt, .. } => {
            // Reason is an AI primitive that generates a Claim<T> - no resolution needed for prompt
        }
        HirExpr::CommitOnce { effect, args, .. } => {
            resolve_effect_ref(table, effect, errors);
            for arg in args {
                resolve_expr(table, arg, errors);
            }
        }
        HirExpr::New { ty, args, .. } => {
            resolve_type(table, ty, errors);
            for arg in args {
                resolve_expr(table, arg, errors);
            }
        }
        HirExpr::Continue { .. } => {}
    }
}

fn resolve_pat(table: &mut SymbolTable, pat: &mut HirPat, errors: &mut Vec<String>) {
    match pat {
        HirPat::Ident { name, is_mut, .. } => {
            table.add_value(
                name.clone(),
                ValueSymbol::Let {
                    name: name.clone(),
                    ty: None,
                    is_mut: *is_mut,
                },
            );
        }
        HirPat::Tuple { pats, .. } => {
            for p in pats {
                resolve_pat(table, p, errors);
            }
        }
        HirPat::Struct { path, fields, .. } => {
            resolve_type_path(table, path, errors);
            for f in fields {
                if let Some(p) = &mut f.pat {
                    resolve_pat(table, p, errors);
                }
            }
        }
        HirPat::Enum { path, fields, .. } => {
            resolve_type_path(table, path, errors);
            for f in fields {
                resolve_pat(table, f, errors);
            }
        }
        HirPat::Or { pats, .. } => {
            for p in pats {
                resolve_pat(table, p, errors);
            }
        }
        HirPat::Ref { pat, .. } => {
            resolve_pat(table, pat, errors);
        }
        _ => {}
    }
}

fn resolve_type(table: &mut SymbolTable, ty: &mut HirType, errors: &mut Vec<String>) {
    match ty {
        HirType::Ref { ty, .. } => resolve_type(table, ty, errors),
        HirType::Owned { ty, .. } => resolve_type(table, ty, errors),
        HirType::Claim { ty, .. } => resolve_type(table, ty, errors),
        HirType::Verified { ty, policy, .. } => {
            resolve_type(table, ty, errors);
            resolve_type(table, policy, errors);
        }
        HirType::Array { ty, size, .. } => {
            resolve_type(table, ty, errors);
            if let Some(size) = size {
                resolve_expr(table, size, errors);
            }
        }
        HirType::Tuple { types, .. } => {
            for t in types {
                resolve_type(table, t, errors);
            }
        }
        HirType::Fn {
            params,
            ret,
            effects,
            ..
        } => {
            for p in params {
                resolve_type(table, p, errors);
            }
            resolve_type(table, ret, errors);
            resolve_effect_set(table, effects, errors);
        }
        HirType::Path { path, .. } => {
            resolve_type_path(table, path, errors);
        }
        _ => {}
    }
}

fn resolve_type_path(table: &mut SymbolTable, path: &mut HirTypePath, errors: &mut Vec<String>) {
    for segment in &mut path.segments {
        if let Some(args) = &mut segment.args {
            for arg in &mut args.args {
                match arg {
                    HirGenericArg::Type { ty, .. } => resolve_type(table, ty, errors),
                    HirGenericArg::Const { expr, .. } => resolve_expr(table, expr, errors),
                }
            }
        }
    }
}

fn resolve_expr_path(table: &mut SymbolTable, path: &mut HirExprPath, errors: &mut Vec<String>) {
    for segment in &mut path.segments {
        if let Some(args) = &mut segment.args {
            for arg in &mut args.args {
                match arg {
                    HirGenericArg::Type { ty, .. } => resolve_type(table, ty, errors),
                    HirGenericArg::Const { expr, .. } => resolve_expr(table, expr, errors),
                }
            }
        }
    }
}

fn resolve_effect_ref(
    table: &mut SymbolTable,
    effect: &mut HirEffectRef,
    errors: &mut Vec<String>,
) {
    resolve_type_path(table, &mut effect.path, errors);
}

fn resolve_effect_set(
    table: &mut SymbolTable,
    effects: &mut HirEffectSet,
    errors: &mut Vec<String>,
) {
    for effect in &mut effects.effects {
        resolve_effect_ref(table, effect, errors);
    }
}
