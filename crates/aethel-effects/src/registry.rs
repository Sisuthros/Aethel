//! Effect registry.

use aethel_syntax::span::Span;
use indexmap::IndexMap;

/// Effect registry for known effects and their operations.
#[derive(Debug, Default)]
pub struct EffectRegistry {
    pub effects: IndexMap<String, EffectDefinition>,
}

#[derive(Debug, Clone)]
pub struct EffectDefinition {
    pub name: String,
    pub operations: Vec<EffectOperation>,
}

#[derive(Debug, Clone)]
pub struct EffectOperation {
    pub name: String,
    pub params: Vec<EffectParam>,
    pub ret_type: Option<aethel_ir::lower::IrType>,
}

#[derive(Debug, Clone)]
pub struct EffectParam {
    pub name: String,
    pub ty: aethel_ir::lower::IrType,
}

impl EffectRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::type_complexity)]
    pub fn register_builtin(
        &mut self,
        name: &str,
        operations: &[(&str, &[(&str, &str)], Option<&str>)],
    ) {
        let ops = operations
            .iter()
            .map(|(op_name, params, ret)| EffectOperation {
                name: op_name.to_string(),
                params: params
                    .iter()
                    .map(|(param_name, param_type)| EffectParam {
                        name: param_name.to_string(),
                        ty: parse_type(param_type),
                    })
                    .collect(),
                ret_type: ret.map(parse_type),
            })
            .collect();

        self.effects.insert(
            name.to_string(),
            EffectDefinition {
                name: name.to_string(),
                operations: ops,
            },
        );
    }

    pub fn get(&self, name: &str) -> Option<&EffectDefinition> {
        self.effects.get(name)
    }

    pub fn resolve_operation(&self, effect: &str, op: &str) -> Option<&EffectOperation> {
        self.effects
            .get(effect)?
            .operations
            .iter()
            .find(|operation| operation.name == op)
    }
}

fn parse_type(value: &str) -> aethel_ir::lower::IrType {
    use aethel_ir::lower::{IrType, IrTypePath};

    match value {
        "int" => IrType::Int { span: Span::zero() },
        "bool" => IrType::Bool { span: Span::zero() },
        "string" => IrType::String { span: Span::zero() },
        "Receipt" => IrType::Path {
            span: Span::zero(),
            path: IrTypePath::single("Receipt"),
        },
        _ => IrType::Path {
            span: Span::zero(),
            path: IrTypePath::single(value),
        },
    }
}
