//! Shared-team action helpers for the `memory` tool. Mirrors the
//! `list_action` / `read_action` / `write_action` shape from the
//! parent `memory.rs`, but addresses the tenant-shared `shared_team`
//! namespace (per `argo-memory::shared_team`) instead of the
//! worker's private file namespace.
//!
//! The store-side namespace is tenant-scoped, so an optional
//! `team_id` parameter is interpreted as a tenant id and must match
//! the worker's bound tenant when supplied (cross-tenant addressing
//! is rejected explicitly). Future per-team sub-namespacing within
//! a tenant requires a store-side API change tracked separately.
use super::WiredMemory;
use argo_identity::WorkerId;
use argo_memory::shared_team::{shared_team, SharedEntry, SharedTeamNs};
use argo_types::{ArgoError, ArgoResult, TenantId};
use uuid::Uuid;

/// Soft, non-error degradation returned by the team actions when the
/// `memory` tool is not bound to a worker namespace (a bare operator
/// session). Returned in place of a hard `ArgoError::Tool` so the line
/// never surfaces as a tool error in a customer transcript
/// (defense-in-depth alongside the run-path binding in
/// `argo-agent-runner::turn_helpers_run`). See
/// `fix-worker-run-memory-namespace-unbound-team-actions`.
const UNBOUND_TEAM_MEMORY: &str =
    "(no team memory available — this session is not bound to a worker namespace)\n";

/// Resolve the shared-team namespace for the action, plus the worker
/// id used as the author stamp on writes. Returns `Ok(None)` when the
/// tool is not bound to a worker namespace (via `Memory::with_namespace`)
/// so callers degrade gracefully instead of raising a hard tool error.
/// A malformed or cross-tenant `team_id` is still a hard `Err` — those
/// are genuine misuse, not a missing binding.
fn try_resolve_namespace(
    wired: &WiredMemory,
    team_id: Option<&str>,
) -> ArgoResult<Option<(SharedTeamNs, WorkerId)>> {
    let Some(ns) = wired.namespace.as_ref() else {
        return Ok(None);
    };
    let tenant = match team_id {
        None => ns.tenant_id,
        Some(raw) => {
            let parsed = Uuid::parse_str(raw).map_err(|e| ArgoError::Tool {
                tool: "memory".into(),
                msg: format!("invalid team_id (expected uuid): {e}"),
            })?;
            if parsed != ns.tenant_id.0 {
                return Err(ArgoError::Tool {
                    tool: "memory".into(),
                    msg: "cross-team addressing rejected: supplied team_id does \
                         not match the worker's bound tenant"
                        .into(),
                });
            }
            TenantId(parsed)
        }
    };
    Ok(Some((shared_team(tenant), ns.worker_id)))
}

/// Format every shared-team row visible to this tenant. One row per
/// line: `id  author=<worker_id>  bytes=<len>  written_at=<iso>`.
/// The model uses this listing to discover entry ids, then calls
/// `read_team` for the specific row body.
pub(super) async fn list_team_action(
    wired: &WiredMemory,
    team_id: Option<&str>,
) -> ArgoResult<String> {
    let Some((ns, _author)) = try_resolve_namespace(wired, team_id)? else {
        return Ok(UNBOUND_TEAM_MEMORY.into());
    };
    let entries = wired.store.read_shared_team(&ns).await?;
    if entries.is_empty() {
        return Ok("(no shared-team entries)\n".into());
    }
    let mut out = String::new();
    for SharedEntry {
        id,
        author_worker_id,
        content,
        written_at,
    } in &entries
    {
        out.push_str(&format!(
            "- {id} author={} bytes={} written_at={}\n",
            author_worker_id.0,
            content.len(),
            written_at.to_rfc3339(),
        ));
    }
    Ok(out)
}

/// Read one shared-team row by its uuid (`entry`). The
/// shared-team store has no canonical filenames, so `entry` is the
/// `SharedEntry::id` returned by `list_team`.
pub(super) async fn read_team_action(
    wired: &WiredMemory,
    entry: Option<&str>,
    team_id: Option<&str>,
) -> ArgoResult<String> {
    let raw = entry.ok_or_else(|| ArgoError::Tool {
        tool: "memory".into(),
        msg: "missing required `entry` (shared-team uuid) for read_team".into(),
    })?;
    let target = Uuid::parse_str(raw).map_err(|e| ArgoError::Tool {
        tool: "memory".into(),
        msg: format!("invalid read_team `entry` (expected uuid): {e}"),
    })?;
    let Some((ns, _author)) = try_resolve_namespace(wired, team_id)? else {
        return Ok(UNBOUND_TEAM_MEMORY.into());
    };
    let entries = wired.store.read_shared_team(&ns).await?;
    entries
        .into_iter()
        .find(|row| row.id == target)
        .map(|row| row.content)
        .ok_or_else(|| ArgoError::Tool {
            tool: "memory".into(),
            msg: format!("no shared-team entry with id {target}"),
        })
}

/// Append a new shared-team row stamped with the worker's id. The
/// caller-side capability check (`Capability::MemoryWriteTeam`) is
/// enforced by the registry through `required_capabilities`; this
/// helper does not re-check.
pub(super) async fn write_team_action(
    wired: &WiredMemory,
    content: Option<&str>,
    team_id: Option<&str>,
) -> ArgoResult<String> {
    let body = content.ok_or_else(|| ArgoError::Tool {
        tool: "memory".into(),
        msg: "missing required `content` for write_team".into(),
    })?;
    // NO THEATER: when there is no bound worker namespace we cannot write,
    // so report the skip truthfully rather than claiming a write happened.
    let Some((ns, author)) = try_resolve_namespace(wired, team_id)? else {
        return Ok("shared-team write skipped: this session is not bound to a worker \
                   namespace, so there is no team memory to write to."
            .into());
    };
    let written = wired
        .store
        .write_shared_team(&ns, author, body.to_string())
        .await?;
    Ok(format!(
        "wrote shared-team entry (id={}, {} bytes, author={})",
        written.id,
        written.content.len(),
        written.author_worker_id.0,
    ))
}
