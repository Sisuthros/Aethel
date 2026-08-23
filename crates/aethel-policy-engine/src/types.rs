//! Policy engine types

use serde::{Deserialize, Serialize};

/// Policy metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
}

/// Policy bundle containing Rego source and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyBundle {
    pub metadata: PolicyMetadata,
    pub rego_source: String,
    pub entrypoints: Vec<String>,
    #[serde(default)]
    pub data: serde_json::Value,
}

/// Evaluation input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationInput {
    pub policy_id: String,
    pub entrypoint: String,
    pub input: serde_json::Value,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

/// Evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    pub result: serde_json::Value,
    pub metrics: EvaluationMetrics,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationMetrics {
    pub evaluation_time_ms: u64,
    pub cache_hit: bool,
    pub memory_usage_bytes: Option<u64>,
}

/// Policy engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub cache_size: usize,
    pub default_timeout_ms: u64,
    pub enable_cache: bool,
    pub allowed_domains: Vec<String>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            cache_size: 100,
            default_timeout_ms: 5000,
            enable_cache: true,
            allowed_domains: vec!["data".to_string(), "input".to_string()],
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
}
