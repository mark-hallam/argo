//! `query_scored` action helpers for the `memory` tool. Hybrid
//! lexical/vector retrieval against the worker's bound namespace.
//! Split out of `memory.rs` to keep that file under the 500-LOC cap;
//! the action's argument shape, hit envelope, top-k clamps, and
//! namespace formatter all live here so the parent file is just the
//! `Tool` impl + private-action helpers.
//!
//! v1 scope (mission-memory-query-scored-action): the bound namespace
//! only. Explicit-namespace addressing (`namespace: Some("worker:…")`)
//! returns a typed error pointing at the follow-up brief
//! `mission-memory-namespace-addressing` (#43).
use super::WiredMemory;
use argo_identity::MemoryNs;
use argo_memory::ScoredMemoryEntry;
use argo_types::{ArgoError, ArgoResult};
use serde::{Deserialize, Serialize};

/// Arguments for the `query_scored` action. Parsed once the
/// `dispatch` arm has confirmed the action name, so a missing
/// `query` surfaces as a clear tool-level error rather than a
/// JSON-schema rejection.
#[derive(Debug, Deserialize)]
struct QueryScoredArgs {
    /// Search text to rank against. Required.
    query: String,
    /// Top-k cap. Defaults to `5`; values > [`MAX_QUERY_SCORED_LIMIT`]
    /// clamp silently. A literal `0` returns an empty hit list (no
    /// error) — the runtime layer already short-circuits on `k == 0`.
    #[serde(default)]
    limit: Option<usize>,
    /// Explicit namespace selector. Not yet wired in v1 (see brief
    /// `mission-memory-namespace-addressing`, #43). When `Some`, the
    /// action returns a clear error pointing at that follow-up
    /// brief. When `None`, the wired tool's bound namespace is used.
    #[serde(default)]
    namespace: Option<String>,
}

/// Per-hit envelope returned by `query_scored`. Intentionally
/// narrower than [`argo_memory::ScoredMemoryEntry`] so internal
/// fields (vector cache, raw row id, lock state) stay invisible
/// to the agent.
#[derive(Debug, Serialize)]
struct MemoryHit {
    entry_id: uuid::Uuid,
    snippet: String,
    score: f32,
    namespace: String,
    written_at: chrono::DateTime<chrono::Utc>,
}

/// Soft cap on `query_scored.limit` — protects the prompt from a
/// runaway top-k. The action clamps silently rather than erroring
/// so a model that asks for 100 still gets a useful answer.
const MAX_QUERY_SCORED_LIMIT: usize = 20;
/// Default top-k when the caller omits `limit`.
const DEFAULT_QUERY_SCORED_LIMIT: usize = 5;

/// `query_scored` — agent-callable hybrid lexical/vector retrieval
/// against the worker's bound namespace.
pub(super) async fn query_scored_action(
    wired: &WiredMemory,
    args: &serde_json::Value,
) -> ArgoResult<String> {
    let parsed: QueryScoredArgs =
        serde_json::from_value(args.clone()).map_err(|e| ArgoError::Tool {
            tool: "memory".into(),
            msg: format!("invalid query_scored arguments: {e}"),
        })?;
    if parsed.namespace.is_some() {
        return Err(ArgoError::Tool {
            tool: "memory".into(),
            msg: "explicit `namespace` not yet supported by query_scored — \
                 omit it to query the worker's bound namespace. \
                 Tracked by mission-memory-namespace-addressing (#43)."
                .into(),
        });
    }
    let ns = wired.namespace.as_ref().ok_or_else(|| ArgoError::Tool {
        tool: "memory".into(),
        msg: "memory tool not bound to a worker namespace — \
             register via `Memory::with_namespace(store, ws, vertical_id, ns)` \
             instead of `Memory::new(store, ws, vertical_id)`."
            .into(),
    })?;
    let k = parsed
        .limit
        .unwrap_or(DEFAULT_QUERY_SCORED_LIMIT)
        .min(MAX_QUERY_SCORED_LIMIT);
    let hits = wired.store.query_scored(ns, &parsed.query, k).await?;
    let ns_label = format_namespace(ns);
    let envelope: Vec<MemoryHit> = hits.into_iter().map(|h| to_hit(h, &ns_label)).collect();
    serde_json::to_string(&envelope).map_err(|e| ArgoError::Tool {
        tool: "memory".into(),
        msg: format!("query_scored serialise: {e}"),
    })
}

fn to_hit(scored: ScoredMemoryEntry, namespace: &str) -> MemoryHit {
    MemoryHit {
        entry_id: scored.entry.id,
        snippet: scored.entry.body,
        score: scored.score,
        namespace: namespace.to_string(),
        written_at: scored.entry.learned_at,
    }
}

fn format_namespace(ns: &MemoryNs) -> String {
    let scope = match ns.scope {
        argo_identity::MemoryScope::WorkerSelf => "worker_self",
        argo_identity::MemoryScope::Role => "role",
        argo_identity::MemoryScope::SharedTeam => "shared_team",
    };
    format!("{}/{}/{}", ns.tenant_id.0, ns.worker_id.0, scope)
}
