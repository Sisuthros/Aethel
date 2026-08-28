//! Convert semantic effect signatures into an `aethel_effects::EffectRegistry`.
//!
//! The semantic checker keeps effect operations as `OperationSig` structs keyed
//! by name. The runtime interpreter and the policy authorizer need an
//! `EffectRegistry` whose `EffectOperation::ty` is an `aethel_ir::lower::IrType`
//! so they can inspect the declared `Verified` policy. This module performs that
//! structural conversion without re-parsing source strings.

use super::semantic::SemanticChecker;
use aethel_effects::{EffectDefinition, EffectOperation, EffectParam, EffectRegistry};

/// Build an `EffectRegistry` from the signatures collected by the semantic
/// checker. This is used by the runtime interpreter for NG6 runtime
/// authorisation.
pub(crate) fn registry_from_semantic(checker: &SemanticChecker) -> EffectRegistry {
    let mut registry = EffectRegistry::new();
    for (effect_name, operations) in &checker.effects {
        let ops: Vec<EffectOperation> = operations
            .iter()
            .map(|(op_name, sig)| EffectOperation {
                name: op_name.clone(),
                params: sig
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| EffectParam {
                        name: format!("arg{i}"),
                        ty: ty.clone(),
                    })
                    .collect(),
                ret_type: Some(sig.ret.clone()),
            })
            .collect();
        registry.effects.insert(
            effect_name.clone(),
            EffectDefinition {
                name: effect_name.clone(),
                operations: ops,
            },
        );
    }
    registry
}
