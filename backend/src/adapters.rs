//! Adapter contracts. No physical Campbell or ChirpStack transport is implemented yet.
use crate::PushEvent;

pub trait Decoder: Send + Sync {
    fn adapter_kind(&self) -> &'static str;
    fn decode(&self, payload: &[u8]) -> anyhow::Result<PushEvent>;
}

/// Our own normalized protocol; not a Campbell native wire format.
pub struct NormalizedJson;
impl Decoder for NormalizedJson {
    fn adapter_kind(&self) -> &'static str {
        "normalized_json_v1"
    }
    fn decode(&self, payload: &[u8]) -> anyhow::Result<PushEvent> {
        let event: PushEvent = serde_json::from_slice(payload)?;
        event
            .validate(chrono::Utc::now())
            .map_err(anyhow::Error::msg)?;
        Ok(event)
    }
}

/// Future poll adapters must persist a batch and advance its cursor in one transaction.
#[derive(Debug)]
pub struct PollBatch {
    pub events: Vec<PushEvent>,
    pub next_cursor: serde_json::Value,
}
