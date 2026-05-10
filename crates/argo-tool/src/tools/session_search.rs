//! `session_search` — model-callable wrapper over the existing FTS5
//! session-message search (`SessionStore::search_messages`). The same
//! query path that backs `argo session search` and the gateway's
//! `GET /search` is exposed to the agent so workers can ask "have I
//! done this before / what did the user say last time about X"
//! without leaving the agent loop.
//!
//! Read-only by construction; tenant-scoped at the filesystem layer
//! (one SQLite DB per tenant). The wired tool sees only the tenant
//! whose store was passed to [`SessionSearch::new`], so the FTS5
//! query never crosses tenant boundaries. Dispatch is gated on
//! `Capability::ToolDispatch { tool_name_glob: "session_search" }`
//! by the registry before this tool runs; no further per-call
//! capabilities are declared.
//!
//! ```
//! use argo_tool::{SessionSearch, Tool};
//! let t = SessionSearch::default();
//! assert_eq!(t.name(), "session_search");
//! ```
use crate::tool::{error_text, success_text, Concurrency, Tool, ToolContext};
use argo_store::{MessageSearchHit, SessionStore};
use argo_types::{ArgoError, ArgoResult, CapabilitySet, ToolCallResult};
use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

/// Hard cap on `limit` so a model can't request a huge page.
const MAX_LIMIT: u32 = 200;

/// Default `limit` when the caller omits it.
const DEFAULT_LIMIT: u32 = 20;

/// `session_search` tool. `Default::default()` returns a stub that
/// errors at dispatch — used by the existing `builtin_tools(cwd)`
/// sites that don't have a `SessionStore` to thread through. The
/// wired form `SessionSearch::new(store)` actually executes the
/// query.
#[derive(Default, Clone)]
pub struct SessionSearch {
    wired: Option<Arc<SessionStore>>,
}

impl std::fmt::Debug for SessionSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionSearch")
            .field("wired", &self.wired.is_some())
            .finish()
    }
}

impl SessionSearch {
    /// Build a tool wired to a live `SessionStore`. Callers that
    /// already opened a per-tenant store (`db_path(home)`) hand the
    /// `Arc` here and dispatch goes through
    /// [`SessionStore::search_messages`].
    #[must_use]
    pub fn new(store: Arc<SessionStore>) -> Self {
        Self { wired: Some(store) }
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    query: String,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    since_minutes: Option<u32>,
}

#[async_trait]
impl Tool for SessionSearch {
    fn name(&self) -> &str {
        "session_search"
    }

    fn description(&self) -> &str {
        "Full-text search the local session message log via FTS5. \
         Returns ranked hits as `{ hits: [{ session_id, seq, role, \
         snippet, ts_micros }] }`. Use to recall what the user said \
         previously about a topic, or whether you've already worked on \
         a similar task. Read-only; never crosses tenant boundaries."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "FTS5 query expression (e.g. \
                                    `migration AND postgres`).",
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": MAX_LIMIT,
                    "description": "Max hits to return (default 20, hard cap 200).",
                },
                "since_minutes": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "Only return hits whose source message \
                                    was created within the last N minutes.",
                },
            },
            "additionalProperties": false,
        })
    }

    fn concurrency(&self) -> Concurrency {
        Concurrency::Parallel
    }

    fn required_capabilities(&self, _args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        Ok(CapabilitySet(Vec::new()))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed: Args = match serde_json::from_value(args) {
            Ok(a) => a,
            Err(e) => {
                return Ok(error_text(
                    ctx.call_id,
                    format!("invalid arguments: {e}"),
                    started.elapsed().as_millis() as u64,
                ));
            }
        };
        let Some(store) = self.wired.as_ref() else {
            return Ok(error_text(
                ctx.call_id,
                "session store not wired — register via \
                 `SessionSearch::new(store)` instead of \
                 `SessionSearch::default()`."
                    .into(),
                started.elapsed().as_millis() as u64,
            ));
        };
        let limit = parsed.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
        let raw = match store.search_messages(&parsed.query, limit).await {
            Ok(hits) => hits,
            Err(ArgoError::Provider(msg)) => {
                return Ok(error_text(
                    ctx.call_id,
                    format!("search failed: {msg}"),
                    started.elapsed().as_millis() as u64,
                ));
            }
            Err(other) => return Err(other),
        };
        let cutoff_micros = parsed
            .since_minutes
            .map(|m| Utc::now().timestamp_micros() - i64::from(m) * 60 * 1_000_000);
        let hits: Vec<&MessageSearchHit> = raw
            .iter()
            .filter(|h| match cutoff_micros {
                Some(cut) => h.created_at_micros >= cut,
                None => true,
            })
            .collect();
        let payload = serde_json::json!({
            "hits": hits
                .iter()
                .map(|h| serde_json::json!({
                    "session_id": h.session_id.0.to_string(),
                    "seq": h.seq,
                    "role": h.role,
                    "snippet": h.snippet,
                    "ts_micros": h.created_at_micros,
                }))
                .collect::<Vec<_>>(),
        });
        let body = match serde_json::to_string(&payload) {
            Ok(s) => s,
            Err(e) => {
                return Ok(error_text(
                    ctx.call_id,
                    format!("serialize payload: {e}"),
                    started.elapsed().as_millis() as u64,
                ));
            }
        };
        Ok(success_text(
            ctx.call_id,
            body,
            started.elapsed().as_millis() as u64,
        ))
    }
}

#[cfg(test)]
#[path = "session_search_tests.rs"]
mod tests;
