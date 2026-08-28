//! Runtime policy authorisation for effect operations.
//!
//! NG6: at runtime, before an effect operation is recorded as executed, the
//! interpreter checks that the argument's `Verified` policy is accepted by the
//! operation's declared parameter policy. This is defence in depth: the static
//! checker already enforces this, but the interpreter refuses to proceed if the
//! runtime value's policy does not match the operation's requirement.
//!
//! The design is intentionally minimal. A `PolicyAuthorizer` is constructed from
//! the effect definitions present in the compiled module. It answers the
//! question: "given an effect operation that requires `Verified<T, P_op>`, does
//! the runtime value's policy `P_val` satisfy it?"
//!
//! For v0.3 the authorisation rule is structural equality of policy names. The
//! effect registry carries the operation signatures; the interpreter compares the
//! policy path embedded in the `Verified` runtime value against the policy path
//! declared in the operation's parameter type. A mismatch produces an
//! `AE-EPISTEMIC-003`-style runtime violation and the effect is blocked.

use aethel_effects::EffectRegistry;
use aethel_ir::lower::IrType;

/// Authorisation decision returned by the policy authorizer.
#[derive(Debug, Clone, PartialEq)]
pub enum AuthorizeResult {
    /// The value's policy satisfies the operation's declared policy.
    Allow,
    /// The operation is unknown; fail closed.
    UnknownOperation { effect: String, operation: String },
    /// The operation expects a verified argument but the value's policy differs.
    PolicyMismatch {
        operation: String,
        expected: String,
        found: String,
    },
    /// The argument is not `Verified<T, Policy>` at all.
    NotVerified { operation: String },
}

impl AuthorizeResult {
    /// True if the operation should be permitted.
    #[must_use]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }

    /// Human-readable violation message when not allowed.
    #[must_use]
    pub fn violation_message(&self) -> Option<String> {
        match self {
            Self::Allow => None,
            Self::UnknownOperation { effect, operation } => Some(format!(
                "unknown operation `{effect}.{operation}` at runtime"
            )),
            Self::PolicyMismatch {
                operation,
                expected,
                found,
            } => Some(format!(
                "runtime policy mismatch for `{operation}`: expected `{expected}`, found `{found}`"
            )),
            Self::NotVerified { operation } => Some(format!(
                "runtime effect `{operation}` requires a verified argument"
            )),
        }
    }
}

/// Runtime policy authorizer.
#[derive(Debug, Default, Clone)]
pub struct PolicyAuthorizer {
    effects: EffectRegistry,
}

impl PolicyAuthorizer {
    /// Create an authorizer from an effect registry.
    pub fn new(effects: EffectRegistry) -> Self {
        Self { effects }
    }

    /// Authorize an effect operation call.
    ///
    /// `effect_name` is the declared effect (e.g. `PaymentGateway`).
    /// `operation_name` is the method being invoked (e.g. `refund`).
    /// `argument_policy` is the policy name carried by the runtime `Verified`
    /// value, if any.
    pub fn authorize(
        &self,
        effect_name: &str,
        operation_name: &str,
        argument_policy: Option<&str>,
    ) -> AuthorizeResult {
        let Some((_effect, operation)) = self
            .effects
            .resolve_operation_by_hint(effect_name, operation_name)
        else {
            return AuthorizeResult::UnknownOperation {
                effect: effect_name.to_string(),
                operation: operation_name.to_string(),
            };
        };

        // The first parameter that requires Verified<T, Policy> defines the
        // required policy. If the operation has no such parameter, we still
        // require the argument to be verified when one is provided.
        let required = operation
            .params
            .iter()
            .find_map(|param| policy_name_from_type(&param.ty));

        match (required, argument_policy) {
            (Some(expected), Some(found)) => {
                if expected == found {
                    AuthorizeResult::Allow
                } else {
                    AuthorizeResult::PolicyMismatch {
                        operation: operation_name.to_string(),
                        expected,
                        found: found.to_string(),
                    }
                }
            }
            (Some(_), None) => AuthorizeResult::NotVerified {
                operation: operation_name.to_string(),
            },
            // Operation does not declare a Verified parameter: no runtime
            // policy check is required for this argument position.
            (None, _) => AuthorizeResult::Allow,
        }
    }
}

/// Extract the policy name from a `Verified<T, Policy>` IR type.
fn policy_name_from_type(ty: &IrType) -> Option<String> {
    match ty {
        IrType::Verified { policy, .. } => policy_name_from_path(policy),
        _ => None,
    }
}

fn policy_name_from_path(ty: &IrType) -> Option<String> {
    match ty {
        IrType::Path { path, .. } => path.segments.last().map(|segment| segment.name.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aethel_effects::EffectRegistry;
    use aethel_ir::lower::IrType;
    use aethel_ir::lower::{IrPathSegment, IrTypePath};
    use aethel_syntax::span::Span;

    fn zero_span() -> Span {
        use aethel_syntax::span::{ByteOffset, FileId};
        Span::new(FileId::new(0), ByteOffset::new(0), ByteOffset::new(0))
    }

    #[allow(dead_code)]
    fn path(name: &str) -> IrType {
        IrType::Path {
            span: zero_span(),
            path: IrTypePath {
                span: zero_span(),
                segments: vec![IrPathSegment {
                    span: zero_span(),
                    name: name.to_string(),
                    args: None,
                }],
            },
        }
    }

    #[allow(dead_code)]
    fn verified(policy: &str) -> IrType {
        IrType::Verified {
            span: zero_span(),
            ty: Box::new(path("Data")),
            policy: Box::new(path(policy)),
        }
    }

    fn registry_with_policy_op() -> EffectRegistry {
        let mut registry = EffectRegistry::new();
        registry.register_builtin(
            "PaymentGateway",
            &[(
                "refund",
                &[("decision", "Verified<Data, RefundPolicy>")],
                Some("int"),
            )],
        );
        registry
    }

    #[test]
    fn allow_matching_policy() {
        let authorizer = PolicyAuthorizer::new(registry_with_policy_op());
        let result = authorizer.authorize("PaymentGateway", "refund", Some("RefundPolicy"));
        assert_eq!(result, AuthorizeResult::Allow);
    }

    #[test]
    fn reject_wrong_policy() {
        let authorizer = PolicyAuthorizer::new(registry_with_policy_op());
        let result = authorizer.authorize("PaymentGateway", "refund", Some("OtherPolicy"));
        assert!(matches!(
            result,
            AuthorizeResult::PolicyMismatch {
                operation,
                expected,
                found,
            } if operation == "refund" && expected == "RefundPolicy" && found == "OtherPolicy"
        ));
    }

    #[test]
    fn reject_missing_policy() {
        let authorizer = PolicyAuthorizer::new(registry_with_policy_op());
        let result = authorizer.authorize("PaymentGateway", "refund", None);
        assert!(
            matches!(result, AuthorizeResult::NotVerified { operation } if operation == "refund")
        );
    }

    #[test]
    fn unknown_effect_fails_closed() {
        let authorizer = PolicyAuthorizer::new(EffectRegistry::new());
        let result = authorizer.authorize("Missing", "op", Some("P"));
        assert!(matches!(
            result,
            AuthorizeResult::UnknownOperation { effect, operation }
            if effect == "Missing" && operation == "op"
        ));
    }

    #[test]
    fn unknown_operation_fails_closed() {
        let authorizer = PolicyAuthorizer::new(registry_with_policy_op());
        let result = authorizer.authorize("PaymentGateway", "charge", Some("RefundPolicy"));
        assert!(matches!(
            result,
            AuthorizeResult::UnknownOperation { effect, operation }
            if effect == "PaymentGateway" && operation == "charge"
        ))
    }
}
