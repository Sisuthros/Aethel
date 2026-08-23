//! Model adapter trait.

use aethel_ir::lower::{IrExpr, IrExprPath, IrType};
use anyhow::Result;
use async_trait::async_trait;

/// Adapter for model providers (LLMs, etc.).
#[async_trait]
pub trait ModelAdapter: Send + Sync {
    /// Execute an ask operation.
    async fn ask(
        &self,
        model: &IrExprPath,
        goal: &str,
        input: &IrExpr,
        output_ty: &IrType,
    ) -> Result<IrExpr>;

    /// Verify a claim against a policy.
    async fn verify(&self, claim: &IrExpr, policy: &IrExprPath) -> Result<IrExpr>;
}

/// Registry of model adapters.
#[derive(Default)]
pub struct ModelRegistry {
    adapters: std::collections::HashMap<String, Box<dyn ModelAdapter>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, adapter: Box<dyn ModelAdapter>) {
        self.adapters.insert(name, adapter);
    }

    pub fn get(&self, name: &str) -> Option<&dyn ModelAdapter> {
        self.adapters.get(name).map(|b| b.as_ref())
    }
}
