//! Prompt context packing for memory layers.
use crate::layers::LayeredMemoryEntry;
use serde::{Deserialize, Serialize};

const CONTEXT_TOKEN_BUDGET: usize = 7_900;

/// Prompt-ready context assembled for a worker action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPack {
    /// Rows included in prompt order.
    pub entries: Vec<LayeredMemoryEntry>,
    /// Prompt-ready body with layer markers.
    pub text: String,
    /// Conservative token estimate, capped below 8k.
    pub token_estimate: usize,
}

pub(crate) fn pack_context(rows: Vec<LayeredMemoryEntry>) -> ContextPack {
    let mut entries = Vec::new();
    let mut text = String::new();
    let mut token_estimate = 0;
    for row in rows {
        let body = row.entry.body.trim();
        if body.is_empty() {
            continue;
        }
        let segment = format!("[{}] {}\n", row.layer.as_str(), body);
        let next_tokens = approx_tokens(&segment);
        if token_estimate + next_tokens > CONTEXT_TOKEN_BUDGET {
            break;
        }
        token_estimate += next_tokens;
        text.push_str(&segment);
        entries.push(row);
    }
    ContextPack {
        entries,
        text,
        token_estimate,
    }
}

fn approx_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}
