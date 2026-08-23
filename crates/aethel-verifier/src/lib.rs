//! Aethel Verifier - Claim/Verified type validation and policy checking

use aethel_policy_engine::{EvaluationInput, PolicyEngine};
use anyhow::Result;
use serde_json::Value;

/// Verifier errors
#[derive(Debug, thiserror::Error)]
pub enum VerifierError {
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Claim type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    #[error("Policy evaluation failed: {0}")]
    PolicyEvaluationFailed(String),

    #[error("Invalid claim: {0}")]
    InvalidClaim(String),

    #[error("Policy engine error: {0}")]
    PolicyEngineError(#[from] aethel_policy_engine::PolicyError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Verification result
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VerificationResult {
    pub verified: bool,
    pub verified_type: Option<String>,
    pub policy_id: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Claim type information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClaimInfo {
    pub claim_type: String,
    pub inner_type: String,
}

/// Verified type information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerifiedInfo {
    pub verified_type: String,
    pub inner_type: String,
    pub policy_id: String,
}

/// Aethel Verifier - validates Claim/Verified types against policies
pub struct Verifier {
    policy_engine: PolicyEngine,
}

impl Verifier {
    /// Create a new verifier with default configuration
    pub fn new() -> Result<Self, anyhow::Error> {
        let policy_engine = PolicyEngine::new()?;
        Ok(Self { policy_engine })
    }

    /// Create a new verifier with custom policy engine
    pub fn with_policy_engine(policy_engine: PolicyEngine) -> Self {
        Self { policy_engine }
    }

    /// Verify a claim against a policy
    pub async fn verify_claim(
        &self,
        claim_type: &str,
        claim_value: Value,
        policy_id: &str,
    ) -> Result<VerificationResult, anyhow::Error> {
        let input = EvaluationInput {
            policy_id: policy_id.to_string(),
            entrypoint: "main.allow".to_string(),
            input: serde_json::json!({
                "claim_type": claim_type,
                "claim_value": claim_value,
            }),
            data: None,
            timeout_ms: Some(5000),
        };

        let result = self.policy_engine.evaluate(input).await?;

        let verified = match &result.result {
            Value::Object(obj) => obj.get("allow").and_then(|v| v.as_bool()).unwrap_or(false),
            _ => false,
        };

        Ok(VerificationResult {
            verified,
            verified_type: if verified {
                Some("Verified".to_string())
            } else {
                None
            },
            policy_id: if verified {
                Some(policy_id.to_string())
            } else {
                None
            },
            errors: if verified {
                vec![]
            } else {
                vec!["Policy evaluation denied".to_string()]
            },
            warnings: vec![],
        })
    }

    /// Verify a claim and return a Verified type if successful
    pub async fn verify_and_wrap(
        &self,
        claim_type: &str,
        claim_value: Value,
        policy_id: &str,
    ) -> Result<Value, anyhow::Error> {
        let result = self
            .verify_claim(claim_type, claim_value, policy_id)
            .await?;

        if result.verified {
            Ok(serde_json::json!({
                "verified": true,
                "verified_type": result.verified_type,
                "policy_id": result.policy_id,
                "value": Value::Null
            }))
        } else {
            Err(anyhow::anyhow!(
                "Verification failed: {}",
                result.errors.join(", ")
            ))
        }
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new().expect("Failed to create default Verifier")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_verifier_creation() {
        let verifier = Verifier::new();
        assert!(verifier.is_ok());
    }

    #[tokio::test]
    async fn test_verifier_default() {
        let verifier = Verifier::default();
        // PolicyEngine implements Default, just check it exists
        let _ = verifier.policy_engine;
    }
}
