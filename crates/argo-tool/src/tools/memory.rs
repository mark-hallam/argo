//! `memory` — declared as in-core per §9 so the schema is always
//! present. Default-constructed `Memory` carries no store handle and
//! returns the historical "memory store not wired" error so the 15
//! existing call sites of `builtin_tools(cwd)` keep their old
//! behaviour. The agent runner uses the new
//! `builtin_tools_with_memory(cwd, store, ws, vertical_id)` constructor
//! to hand the tool a live `MemoryStore`, after which dispatch goes
//! through `MemoryStore::append` / `append_for_vertical` /
//! `list_active` honouring `Lock::UserProtected` rejection.
use crate::tool::{error_text, success_text, Tool, ToolContext};
use argo_identity::MemoryNs;
use argo_memory::{MemoryFile, MemoryStore, PrincipalOverride, ScoredMemoryEntry};
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, ToolCallResult, WorkspaceId};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// `memory` tool. `Default::default()` returns a stub that errors at
/// dispatch — used by the 15 existing `builtin_tools(cwd)` sites that
/// don't have a `MemoryStore` to thread through. The wired form
/// `Memory::new(store, ws, vertical_id)` actually executes the action.
#[derive(Debug, Default)]
pub struct Memory {
    wired: Option<WiredMemory>,
}

#[derive(Debug, Clone)]
struct WiredMemory {
    store: Arc<MemoryStore>,
    workspace_id: WorkspaceId,
    /// Vertical scope from the bound session (#680). When `Some`,
    /// writes go through `append_for_vertical` so vertical-scoped
    /// reads (`list_for_vertical`) see them. When `None`, writes
    /// stay workspace-global (NULL `vertical_id`).
    vertical_id: Option<String>,
    /// Worker namespace this tool instance is bound to. Required
    /// by the `query_scored` action; legacy `list` / `read` /
    /// `write` actions ignore it. Older callers of [`Memory::new`]
    /// leave this `None`; new callers use [`Memory::with_namespace`].
    namespace: Option<MemoryNs>,
}

impl Memory {
    /// Build a Memory tool wired to a live `MemoryStore`. The
    /// agent runner calls this once per turn so dispatch can
    /// actually persist.
    #[must_use]
    pub fn new(
        store: Arc<MemoryStore>,
        workspace_id: WorkspaceId,
        vertical_id: Option<String>,
    ) -> Self {
        Self {
            wired: Some(WiredMemory {
                store,
                workspace_id,
                vertical_id,
                namespace: None,
            }),
        }
    }

    /// Build a Memory tool wired to a live `MemoryStore` AND bound
    /// to a worker [`MemoryNs`]. The `query_scored` action requires
    /// the namespace to know which tenant/worker/scope to query;
    /// callers that don't yet plumb a namespace can keep using
    /// [`Memory::new`] for the legacy `list` / `read` / `write`
    /// actions.
    ///
    /// ```
    /// # use std::sync::Arc;
    /// # async fn doc() -> argo_types::ArgoResult<()> {
    /// use argo_identity::{MemoryNs, MemoryScope, WorkerId};
    /// use argo_memory::MemoryStore;
    /// use argo_store::SessionStore;
    /// use argo_tool::Memory;
    /// use argo_types::TenantId;
    /// let session = SessionStore::open(":memory:").await?;
    /// session.migrate().await?;
    /// let (_tenant, ws) = session.ensure_workspace("ws", "/tmp/ws").await?;
    /// let store = Arc::new(MemoryStore::new(Arc::new(session)));
    /// let ns = MemoryNs::new(TenantId::new(), WorkerId::new(), MemoryScope::WorkerSelf);
    /// let _tool = Memory::with_namespace(store, ws, None, ns);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub fn with_namespace(
        store: Arc<MemoryStore>,
        workspace_id: WorkspaceId,
        vertical_id: Option<String>,
        namespace: MemoryNs,
    ) -> Self {
        Self {
            wired: Some(WiredMemory {
                store,
                workspace_id,
                vertical_id,
                namespace: Some(namespace),
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
struct Args {
    action: String,
    #[serde(default)]
    entry: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

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

#[async_trait]
impl Tool for Memory {
    fn name(&self) -> &str {
        "memory"
    }
    fn description(&self) -> &str {
        "List, read, write, or rank entries in Argo's memory store. \
         `action` is one of `list`, `read`, `write`, or `query_scored`. \
         For `list` / `read` / `write`, `entry` selects one of \
         `MEMORY.md` / `USER.md` / `SOUL.md` / `AGENTS.md` (or the bare \
         form). For `query_scored`, `query` is required, `limit` caps \
         top-k (default 5, max 20)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "read", "write", "query_scored"],
                },
                "entry": { "type": "string", "description": "Entry name (`read`, `write`)." },
                "content": { "type": "string", "description": "Body to write (`write`)." },
                "query": {
                    "type": "string",
                    "description": "Search text (`query_scored`).",
                },
                "limit": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "Top-k cap (`query_scored`, default 5, max 20).",
                },
                "namespace": {
                    "type": "string",
                    "description":
                        "Optional explicit namespace (`query_scored`). \
                         Not yet wired; omit to query the worker's bound namespace.",
                },
            },
            "additionalProperties": false,
        })
    }

    fn required_capabilities(&self, args: &serde_json::Value) -> ArgoResult<CapabilitySet> {
        let parsed = parse_args(args)?;
        let mut caps = Vec::new();
        if parsed.action == "write" {
            caps.push(Capability::MemoryWrite);
        }
        Ok(CapabilitySet(caps))
    }

    async fn dispatch(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> ArgoResult<ToolCallResult> {
        let started = Instant::now();
        let parsed = parse_args(&args)?;
        let Some(wired) = self.wired.as_ref() else {
            return Ok(error_text(
                ctx.call_id,
                "memory store not wired — register via \
                 `argo_tool::builtin_tools_with_memory(cwd, store, ws, vertical_id)` \
                 instead of `builtin_tools(cwd)`."
                    .into(),
                started.elapsed().as_millis() as u64,
            ));
        };
        let body = match parsed.action.as_str() {
            "list" => list_action(wired).await,
            "read" => read_action(wired, parsed.entry.as_deref()).await,
            "write" => {
                write_action(
                    wired,
                    parsed.entry.as_deref(),
                    parsed.content.as_deref(),
                )
                .await
            }
            "query_scored" => query_scored_action(wired, &args).await,
            other => Err(ArgoError::Tool {
                tool: "memory".into(),
                msg: format!(
                    "invalid action '{other}' (expected list / read / write / query_scored)"
                ),
            }),
        }?;
        Ok(success_text(
            ctx.call_id,
            body,
            started.elapsed().as_millis() as u64,
        ))
    }
}

fn parse_args(args: &serde_json::Value) -> ArgoResult<Args> {
    serde_json::from_value(args.clone()).map_err(|e| ArgoError::Tool {
        tool: "memory".into(),
        msg: format!("invalid arguments: {e}"),
    })
}

/// Resolve a caller-supplied entry token (canonical filename or
/// bare lowercase form) to a [`MemoryFile`]. Case-insensitive so
/// the model can spell either way.
pub(crate) fn resolve_entry(entry: Option<&str>) -> ArgoResult<MemoryFile> {
    let raw = entry.ok_or_else(|| ArgoError::Tool {
        tool: "memory".into(),
        msg: "missing required `entry`".into(),
    })?;
    let upper = raw.to_ascii_uppercase();
    let canonical = match upper.as_str() {
        "MEMORY" | "MEMORY.MD" => "MEMORY.md",
        "USER" | "USER.MD" => "USER.md",
        "SOUL" | "SOUL.MD" => "SOUL.md",
        "AGENTS" | "AGENTS.MD" => "AGENTS.md",
        other => {
            return Err(ArgoError::Tool {
                tool: "memory".into(),
                msg: format!(
                    "unknown entry '{other}' — valid: MEMORY.md / USER.md / SOUL.md / AGENTS.md"
                ),
            })
        }
    };
    MemoryFile::parse(canonical).ok_or_else(|| ArgoError::Tool {
        tool: "memory".into(),
        msg: format!("MemoryFile::parse rejected canonical '{canonical}' (impossible)"),
    })
}

async fn list_action(wired: &WiredMemory) -> ArgoResult<String> {
    let mut out = String::new();
    for file in [
        MemoryFile::Memory,
        MemoryFile::User,
        MemoryFile::Soul,
        MemoryFile::Agents,
    ] {
        let entries = wired.store.list_active(wired.workspace_id, file).await?;
        if entries.is_empty() {
            continue;
        }
        let head = &entries[0];
        out.push_str(&format!(
            "- {} ({} bytes, {} versions)\n",
            file.filename(),
            head.body.len(),
            entries.len(),
        ));
    }
    if out.is_empty() {
        out.push_str("(no memory entries)\n");
    }
    Ok(out)
}

async fn read_action(wired: &WiredMemory, entry: Option<&str>) -> ArgoResult<String> {
    let file = resolve_entry(entry)?;
    let entries = wired.store.list_active(wired.workspace_id, file).await?;
    let head = entries.first().ok_or_else(|| ArgoError::Tool {
        tool: "memory".into(),
        msg: format!("no entries for {}", file.filename()),
    })?;
    Ok(head.body.clone())
}

async fn write_action(
    wired: &WiredMemory,
    entry: Option<&str>,
    content: Option<&str>,
) -> ArgoResult<String> {
    let file = resolve_entry(entry)?;
    let body = content.ok_or_else(|| ArgoError::Tool {
        tool: "memory".into(),
        msg: "missing required `content` for write".into(),
    })?;
    let existing = wired.store.list_active(wired.workspace_id, file).await?;
    let written = if let Some(prev) = existing.first() {
        // Supersede honours the prev entry's lock — UserProtected
        // rejects with a clear ArgoError::UserProtected, which we
        // translate into the operator-facing hint.
        wired
            .store
            .supersede(prev.id, body.to_string(), None, PrincipalOverride::Agent)
            .await
            .map_err(translate_lock_error)?
    } else if let Some(ref vid) = wired.vertical_id {
        wired
            .store
            .append_for_vertical(wired.workspace_id, file, body.to_string(), None, vid)
            .await?
    } else {
        wired
            .store
            .append(wired.workspace_id, file, body.to_string(), None)
            .await?
    };
    Ok(format!(
        "wrote {} ({} bytes, entry_id={})",
        file.filename(),
        written.body.len(),
        written.id
    ))
}

/// `query_scored` — agent-callable hybrid lexical/vector retrieval
/// against the worker's bound namespace.
///
/// v1 scope (mission-memory-query-scored-action): the bound namespace
/// only. Explicit-namespace addressing (`namespace: Some("worker:…")`)
/// returns a typed error pointing at the follow-up brief
/// `mission-memory-namespace-addressing` (#43).
async fn query_scored_action(
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

/// `MemoryStore::supersede` returns `ArgoError::UserProtected` when
/// the prev entry is `Lock::UserProtected` and the agent isn't
/// `Force`. Translate into a tool-facing message that explicitly
/// names the human-only override path.
fn translate_lock_error(err: ArgoError) -> ArgoError {
    match err {
        ArgoError::UserProtected { entity } => ArgoError::Tool {
            tool: "memory".into(),
            msg: format!(
                "rejected: {entity} is user-protected; only a human can write through with \
                 `argo memory edit --force`"
            ),
        },
        other => other,
    }
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
