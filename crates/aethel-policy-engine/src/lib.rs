//! Aethel Policy Engine - OPA-based policy evaluation for Aethel
//!
//! This crate provides policy evaluation using Rego policies compiled to WebAssembly,
//! enabling fine-grained authorization and verification logic.

mod compiler;
mod evaluator;
mod policy;
mod types;

pub use compiler::PolicyCompiler;
pub use evaluator::PolicyEvaluator;
pub use policy::PolicyStore;
pub use types::{
    CacheStats, EngineConfig, EvaluationInput, EvaluationMetrics, EvaluationResult, PolicyBundle,
    PolicyMetadata,
};

use thiserror::Error;

/// Policy engine error types
#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Policy compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Policy evaluation failed: {0}")]
    EvaluationFailed(String),

    #[error("Policy not found: {0}")]
    NotFound(String),

    #[error("Invalid policy format: {0}")]
    InvalidFormat(String),

    #[error("Policy engine error: {0}")]
    EngineError(String),

    #[error("Timeout during policy evaluation")]
    Timeout,

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Main policy engine struct
pub struct PolicyEngine {
    compiler: PolicyCompiler,
    evaluator: PolicyEvaluator,
    #[expect(dead_code)]
    store: PolicyStore,
}

impl PolicyEngine {
    pub fn new() -> Result<Self, PolicyError> {
        let config = types::EngineConfig::default();
        let compiler = PolicyCompiler::new()?;
        let evaluator = PolicyEvaluator::new(config)?;
        let store = PolicyStore::new();
        Ok(Self {
            compiler,
            evaluator,
            store,
        })
    }

    /// Create a new policy engine with custom configuration
    pub fn with_config(config: types::EngineConfig) -> Result<Self, PolicyError> {
        let compiler = PolicyCompiler::new()?;
        let evaluator = PolicyEvaluator::new(config)?;
        let store = PolicyStore::new();
        Ok(Self {
            compiler,
            evaluator,
            store,
        })
    }

    /// Register a policy bundle
    pub async fn register_policy(&self, bundle: types::PolicyBundle) -> Result<(), PolicyError> {
        self.compiler.compile_bundle(bundle).await
    }

    /// Evaluate a policy decision
    pub async fn evaluate(
        &self,
        input: types::EvaluationInput,
    ) -> Result<types::EvaluationResult, PolicyError> {
        self.evaluator.evaluate(input).await
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new().expect("PolicyEngine default should succeed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_policy_engine_creation() {
        let engine = PolicyEngine::new();
        assert!(engine.is_ok());
    }
}
