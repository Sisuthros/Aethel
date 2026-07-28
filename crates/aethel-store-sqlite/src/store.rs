//! SQLite event store implementation.

use aethel_ir::lower::IrModule;
use aethel_syntax::span::Span;
use anyhow::Result;

/// Event store for durable execution.
pub struct EventStore {
    // Pool would go here
}

impl EventStore {
    pub async fn new(_database_url: &str) -> Result<Self> {
        Ok(Self {})
    }

    pub async fn append_event(&self, _event: &Event) -> Result<()> {
        Ok(())
    }

    pub async fn get_events(&self, _correlation_id: &str) -> Result<Vec<Event>> {
        Ok(vec![])
    }
}

#[derive(Debug, Clone)]
pub struct Event {
    pub id: String,
    pub correlation_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
