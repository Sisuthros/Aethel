//! Effect registry.

use aethel_syntax::span::Span;
use indexmap::IndexMap;

/// Type alias for builtin effect operation registration
type BuiltinOperation<'a> = (&'a str, &'a [(&'a str, &'a str)], Option<&'a str>);

/// Effect registry for known effects and their operations.
#[derive(Debug, Default, Clone)]
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

    pub fn register_builtin(&mut self, name: &str, operations: &[BuiltinOperation<'_>]) {
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

    /// Case-insensitive lookup by effect name. Used at runtime because the
    /// source variable bound to an effect (e.g. `payment_gateway`) may differ
    /// in case from the declared effect type (`PaymentGateway`).
    pub fn get_case_insensitive(&self, name: &str) -> Option<&EffectDefinition> {
        let name = name.to_lowercase();
        self.effects
            .values()
            .find(|effect| effect.name.to_lowercase() == name)
    }

    pub fn resolve_operation(&self, effect: &str, op: &str) -> Option<&EffectOperation> {
        self.effects
            .get(effect)?
            .operations
            .iter()
            .find(|operation| operation.name == op)
    }

    /// Resolve an effect operation given a possibly-casemismatched variable
    /// name and a method name. Fails closed: returns `None` if the operation
    /// cannot be uniquely identified. If the hint matches a known effect
    /// (case-insensitively) and that effect declares the operation, it wins;
    /// otherwise we search the whole registry for an effect that declares the
    /// operation and return it only if it is unambiguous.
    pub fn resolve_operation_by_hint(
        &self,
        hint: &str,
        op: &str,
    ) -> Option<(&EffectDefinition, &EffectOperation)> {
        // 1. Exact effect name.
        if let Some(effect) = self.get(hint) {
            if let Some(operation) = effect.operations.iter().find(|o| o.name == op) {
                return Some((effect, operation));
            }
        }

        // 2. Case-insensitive effect name.
        if let Some(effect) = self.get_case_insensitive(hint) {
            if let Some(operation) = effect.operations.iter().find(|o| o.name == op) {
                return Some((effect, operation));
            }
        }

        // 3. Last resort: search all effects for the operation. Only succeed
        // when exactly one effect declares it to avoid authorising the wrong
        // capability.
        let mut matches = self.effects.values().filter_map(|effect| {
            effect
                .operations
                .iter()
                .find(|o| o.name == op)
                .map(|operation| (effect, operation))
        });
        let first = matches.next()?;
        matches.next().is_none().then_some(first)
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
        _ => {
            let policy = value.strip_prefix("Verified<").and_then(|rest| {
                rest.strip_suffix(">")
                    .and_then(|inner| inner.split(", ").nth(1))
            });
            if let Some(policy) = policy {
                IrType::Verified {
                    span: Span::zero(),
                    ty: Box::new(IrType::Path {
                        span: Span::zero(),
                        path: IrTypePath::single("Data"),
                    }),
                    policy: Box::new(IrType::Path {
                        span: Span::zero(),
                        path: IrTypePath::single(policy.trim()),
                    }),
                }
            } else {
                IrType::Path {
                    span: Span::zero(),
                    path: IrTypePath::single(value),
                }
            }
        }
    }
}
