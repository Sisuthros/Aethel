//! Policy module - handles policy loading, compilation, and metadata management

use crate::{PolicyBundle, PolicyError, PolicyMetadata};
use std::collections::HashMap;

/// Policy store - handles policy loading, compilation, and metadata management
pub struct PolicyStore {
    policies: HashMap<String, PolicyBundle>,
}

impl Default for PolicyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyStore {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    /// Load a policy bundle into the store
    pub async fn load_policy(&mut self, bundle: PolicyBundle) -> Result<(), PolicyError> {
        // For now, just store the policy - actual compilation would happen here
        self.policies.insert(bundle.metadata.id.clone(), bundle);
        Ok(())
    }

    /// Get policy metadata by ID
    pub async fn get_metadata(
        &self,
        policy_id: &str,
    ) -> Result<Option<PolicyMetadata>, PolicyError> {
        Ok(self.policies.get(policy_id).map(|b| b.metadata.clone()))
    }

    /// List all loaded policies
    pub async fn list_policies(&self) -> Result<Vec<PolicyMetadata>, PolicyError> {
        Ok(self.policies.values().map(|b| b.metadata.clone()).collect())
    }

    /// Remove a policy
    pub async fn unload_policy(&mut self, policy_id: &str) -> Result<(), PolicyError> {
        self.policies.remove(policy_id);
        Ok(())
    }
}
