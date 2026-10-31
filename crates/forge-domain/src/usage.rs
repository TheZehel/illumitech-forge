//! Observed counters only. Cache is a subset of input; reasoning is a subset of output.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}
impl TokenUsage {
    pub fn new(input: Option<u64>, cached: Option<u64>, output: Option<u64>) -> Self {
        Self {
            input_tokens: input,
            cached_input_tokens: cached.filter(|c| input.is_none_or(|i| *c <= i)),
            output_tokens: output,
            total_tokens: input.zip(output).and_then(|(i, o)| i.checked_add(o)),
        }
    }
}

/// A turn or cumulative session snapshot, never an incremental delta.
/// The adapter supplies a stable key; the engine scopes it to client and repo root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageUnit {
    pub key: String,
    pub session: String,
    pub cumulative: bool,
    pub token_usage: TokenUsage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageObservation {
    pub units: Vec<UsageUnit>,
    pub complete: bool,
    pub limitations: Vec<String>,
}

/// One real native invocation, journaled before spawn and finalized independently of success.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCall {
    pub id: String,
    pub task_id: String,
    pub attempt: u32,
    pub phase: String,
    pub repo_scope: String,
    pub client: String,
    pub model: Option<String>,
    pub outcome: String,
    pub elapsed_ms: Option<u64>,
    pub observation: UsageObservation,
}
