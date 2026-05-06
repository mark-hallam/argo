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
use argo_memory::{MemoryFile, MemoryStore, PrincipalOverride};
use argo_types::{ArgoError, ArgoResult, Capability, CapabilitySet, ToolCallResult, WorkspaceId};
use async_trait::async_trait;
use serde::Deserialize;
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

#[async_trait]
impl Tool for Memory {
    fn name(&self) -> &str {
        "memory"
    }
    fn description(&self) -> &str {
        "List, read, or write entries in Argo's memory store. \
         `action` is one of `list`, `read`, or `write`. `entry` \
         is one of `MEMORY.md` / `USER.md` / `SOUL.md` / `AGENTS.md` \
         (or the bare form `memory` / `user` / `soul` / `agents`)."
    }
    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["action"],
            "properties": {
                "action": { "type": "string", "enum": ["list", "read", "write"] },
                "entry": { "type": "string", "description": "Entry name (`read`, `write`)." },
                "content": { "type": "string", "description": "Body to write (`write`)." },
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
            other => Err(ArgoError::Tool {
                tool: "memory".into(),
                msg: format!("invalid action '{other}' (expected list / read / write)"),
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
