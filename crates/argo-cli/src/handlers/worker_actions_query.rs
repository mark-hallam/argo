//! Query-side worker_action helpers (Stream
//! `claude-worker-actions-split`). Lifted out of
//! `worker_actions.rs` so that file stays under the per-file
//! 500 LOC budget.
//!
//! Owns the read-side surface — `list`, `show`,
//! `collect_latest_actions` — plus a couple of small helpers
//! (`short`, `lookup_max_attempts`). No behaviour change vs.
//! the historical inline implementations in
//! `worker_actions::{list, show, ...}`; the dispatcher in
//! `worker_actions::run` now calls into this module instead.
use super::vertical::resolve_verticals_dir;
use super::worker_actions::ActionRecord;
use crate::home::{db_path, ensure_dir, resolve_home};
use argo_memory::{MemoryFile, MemoryStore};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult};
use std::collections::HashMap;
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn list(
    state_filter: &str,
    job_id: Option<&str>,
    vertical: Option<&str>,
    item_id_filter: Option<&str>,
    action_type_filter: Option<&str>,
    platform_filter: Option<&str>,
    limit: usize,
    json_out: bool,
) -> ArgoResult<()> {
    let actions = collect_latest_actions(vertical).await?;
    let mut rows = super::worker_actions_list::apply_list_filters(
        actions,
        state_filter,
        job_id,
        item_id_filter,
        action_type_filter,
        platform_filter,
    );
    // Newest-first by created_at; ties broken by action_id for determinism.
    rows.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.action_id.cmp(&b.action_id))
    });
    rows.truncate(limit);

    if json_out {
        for a in &rows {
            println!(
                "{}",
                serde_json::to_string(&a).unwrap_or_else(|_| "{}".into())
            );
        }
    } else if rows.is_empty() {
        println!("(no actions)");
    } else {
        println!("action_id                  state        type                   platform       created_at");
        for a in &rows {
            println!(
                "{:<26} {:<12} {:<22} {:<14} {}",
                short(&a.action_id),
                a.state,
                a.action_type,
                a.platform.as_deref().unwrap_or("-"),
                a.created_at,
            );
        }
    }
    Ok(())
}

pub(crate) async fn show(action_id: &str, json_out: bool) -> ArgoResult<()> {
    let actions = collect_latest_actions(None).await?;
    let Some(found) = actions.into_iter().find(|a| a.action_id == action_id) else {
        return Err(ArgoError::Provider(format!(
            "no action with action_id '{action_id}'"
        )));
    };
    let attempt_number = super::worker_actions_dispatch::count_retry_chain(&found);
    let max_attempts = lookup_max_attempts(&found.action_type).await.unwrap_or(1);
    if json_out {
        let mut v = serde_json::to_value(&found).unwrap_or(serde_json::json!({}));
        if let Some(map) = v.as_object_mut() {
            map.insert("retry_attempt".into(), serde_json::json!(attempt_number));
            map.insert("retry_max_attempts".into(), serde_json::json!(max_attempts));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("action_id     {}", found.action_id);
        println!("job_id        {}", found.job_id);
        if let Some(it) = &found.item_id {
            println!("item_id       {it}");
        }
        println!("action_type   {}", found.action_type);
        println!("state         {}", found.state);
        if let Some(p) = &found.platform {
            println!("platform      {p}");
        }
        println!("created_at    {}", found.created_at);
        println!("attempt       {attempt_number}/{max_attempts}");
        if let Some(by) = &found.approved_by {
            println!("approved_by   {by}");
        }
        if let Some(at) = &found.approved_at {
            println!("approved_at   {at}");
        }
        if let Some(at) = &found.executed_at {
            println!("executed_at   {at}");
        }
        if let Some(reason) = &found.blocked_reason {
            println!("blocked       {reason}");
        }
        if !found.evidence.is_empty() {
            println!("evidence:");
            for e in &found.evidence {
                println!("  - {e}");
            }
        }
        println!("payload:");
        println!(
            "{}",
            serde_json::to_string_pretty(&found.payload).unwrap_or_else(|_| "{}".into())
        );
        if let Some(r) = &found.result {
            println!("result:");
            println!(
                "{}",
                serde_json::to_string_pretty(r).unwrap_or_else(|_| "{}".into())
            );
        }
    }
    Ok(())
}

/// Walk every loaded vertical's memory namespace, collect every
/// `worker_action` row, and reduce to "latest snapshot per
/// `action_id`".
pub(crate) async fn collect_latest_actions(
    vertical: Option<&str>,
) -> ArgoResult<Vec<ActionRecord>> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(&db_path(&home)).await?;
    store.migrate().await?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let root = cwd.to_string_lossy();
    let (_tenant, workspace) = store.ensure_workspace(name, &root).await?;
    let store_arc = Arc::new(store);
    let memory = MemoryStore::new(store_arc.clone());

    let vertical_ids: Vec<String> = match vertical {
        Some(v) => vec![v.to_string()],
        None => {
            let dir = resolve_verticals_dir();
            let (registry, _errors) = argo_vertical::load_dir(&dir)?;
            registry.list().map(|(id, _)| id.to_string()).collect()
        }
    };
    latest_actions_in(&memory, workspace, &vertical_ids).await
}

/// cu1g (DEC-050): tenant-aware variant for gateway surfaces. The
/// cwd+flat-only [`collect_latest_actions`] can never see a Genesis
/// teammate's actions — the rows live in the TENANT's workspace (not the
/// cwd/demo one) under a `.genesis/<tenant>/<vid>` vertical id the flat
/// registry never enumerates. Resolves the tenant's workspace (cwd
/// fallback for the demo/anonymous path, mirroring
/// `routes_worker_run_exec::resolve_workspace`) and scans flat + the
/// tenant's own genesis vertical ids.
pub(crate) async fn collect_latest_actions_for_tenant(
    store: &Arc<SessionStore>,
    tenant_id: argo_types::TenantId,
    vertical: Option<&str>,
) -> ArgoResult<Vec<ActionRecord>> {
    // Fail closed: an authenticated tenant with no workspace of its own
    // sees an empty action list, never the shared cwd/demo workspace's.
    let Some(workspace) = workspace_for_tenant(store, tenant_id).await? else {
        return Ok(Vec::new());
    };
    let memory = MemoryStore::new(Arc::clone(store));
    let vertical_ids: Vec<String> = match vertical {
        Some(v) => vec![v.to_string()],
        None => {
            let dir = resolve_verticals_dir();
            let mut ids: Vec<String> = match argo_vertical::load_dir(&dir) {
                Ok((registry, _errors)) => {
                    registry.list().map(|(id, _)| id.to_string()).collect()
                }
                Err(_) => Vec::new(),
            };
            // cu1g (perf fix): bound the genesis enumeration to verticals
            // that actually have rows in THIS workspace via one
            // workspace-scoped SELECT — not an O(N) `.genesis` fs walk
            // across every tenant (the DoS the adversarial review flagged).
            for vid in memory.distinct_verticals(workspace, MemoryFile::Memory).await? {
                if !ids.contains(&vid) {
                    ids.push(vid);
                }
            }
            ids
        }
    };
    latest_actions_in(&memory, workspace, &vertical_ids).await
}

/// Resolve the workspace a tenant's action rows live in: the tenant's
/// own provisioned workspace when one is registered, else the cwd
/// workspace **only when the caller IS the cwd/anonymous tenant** (the
/// demo path). Returns `None` for any other tenant with no workspace.
///
/// cu1h (DEC-050 #1, fail-open fix): the old version fell straight
/// through to the cwd/demo workspace for ANY tenant with no provisioned
/// row, so a session-authenticated tenant whose provisioning row was
/// (theoretically) missing would read/decide the demo workspace's held
/// actions — a cross-tenant read, contrary to fail-closed doctrine. The
/// cwd workspace is legitimate ONLY for the cwd tenant itself (the
/// anonymous/demo path, where the caller's tenant IS the cwd tenant), so
/// the fallback is now gated on a tenant-identity match. Provisioning
/// always writes a workspace row today (`tenant_provision.rs`), so this
/// is fail-closed defence in depth, not a live-bug fix.
pub(crate) async fn workspace_for_tenant(
    store: &Arc<SessionStore>,
    tenant_id: argo_types::TenantId,
) -> ArgoResult<Option<argo_types::WorkspaceId>> {
    if let Some(ws) = store.first_workspace_for_tenant(tenant_id).await? {
        return Ok(Some(ws));
    }
    // No provisioned row. Resolve the cwd workspace AND its owning tenant
    // in one `ensure_workspace` call; only hand it back when the caller is
    // that same cwd tenant. Any other (authenticated) tenant fails closed
    // to `None` — never the shared demo workspace.
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Io(format!("cwd: {e}")))?;
    let name = cwd
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("workspace");
    let (cwd_tenant, cwd_ws) = store
        .ensure_workspace(name, &cwd.to_string_lossy())
        .await
        .map_err(|e| ArgoError::Provider(format!("ensure_workspace: {e}")))?;
    if cwd_tenant == tenant_id {
        return Ok(Some(cwd_ws));
    }
    Ok(None)
}

/// Shared latest-wins fold over `(workspace, vertical_ids)` memory rows.
async fn latest_actions_in(
    memory: &MemoryStore,
    workspace: argo_types::WorkspaceId,
    vertical_ids: &[String],
) -> ArgoResult<Vec<ActionRecord>> {
    let mut latest: HashMap<String, ActionRecord> = HashMap::new();
    for vid in vertical_ids {
        let entries = memory
            .list_for_vertical(workspace, MemoryFile::Memory, vid)
            .await?;
        for e in entries {
            let Ok(rec) = serde_json::from_str::<ActionRecord>(&e.body) else {
                continue;
            };
            if rec.schema_key != ActionRecord::SCHEMA_KEY {
                continue;
            }
            match latest.get(&rec.action_id) {
                Some(prev) if prev.created_at >= rec.created_at => {}
                _ => {
                    latest.insert(rec.action_id.clone(), rec);
                }
            }
        }
    }
    Ok(latest.into_values().collect())
}

pub(crate) fn short(id: &str) -> String {
    if id.len() > 24 {
        format!("{}…", &id[..23])
    } else {
        id.to_string()
    }
}

/// Lookup `[actions.<type>.retry] max_attempts` across every
/// loaded vertical. Returns the first match.
async fn lookup_max_attempts(action_type: &str) -> Option<u32> {
    let dir = super::vertical::resolve_verticals_dir();
    let (registry, _errs) = argo_vertical::load_dir(&dir).ok()?;
    for (_id, loaded) in registry.list() {
        if let Some(decl) = loaded
            .manifest
            .actions
            .as_ref()
            .and_then(|a| a.actions.get(action_type))
        {
            return Some(decl.retry.clone().unwrap_or_default().max_attempts);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn migrated_store() -> Arc<SessionStore> {
        let store = SessionStore::open_unencrypted_for_tests(":memory:")
            .await
            .expect("open test store");
        store.migrate().await.expect("migrate");
        Arc::new(store)
    }

    /// cu1h (DEC-050 #1, fail-open fix): a session-authenticated tenant
    /// with NO provisioned workspace resolves to `None` — never the
    /// shared cwd/demo workspace. This is the cross-read the old fall-open
    /// allowed.
    #[tokio::test]
    async fn workspace_for_tenant_authenticated_no_workspace_is_none() {
        let store = migrated_store().await;
        let stranger = argo_types::TenantId::new();
        let ws = workspace_for_tenant(&store, stranger)
            .await
            .expect("resolve");
        assert!(
            ws.is_none(),
            "a tenant with no workspace must not borrow the cwd/demo workspace"
        );
    }

    /// A tenant that owns a provisioned workspace still resolves to its
    /// own — the fail-closed gate only suppresses the cwd fallback, it
    /// does not change the happy path.
    #[tokio::test]
    async fn workspace_for_tenant_returns_own_provisioned_workspace() {
        let store = migrated_store().await;
        // `ensure_workspace` mints a fresh tenant for a new root_path and
        // inserts the row; `first_workspace_for_tenant` then finds it.
        let (tenant, ws) = store
            .ensure_workspace("own", "tenant://cu1h/own")
            .await
            .expect("ensure workspace");
        let got = workspace_for_tenant(&store, tenant).await.expect("resolve");
        assert_eq!(got, Some(ws));
    }

    /// And the action collector returns an empty list (not the cwd
    /// workspace's actions) for the stranger tenant.
    #[tokio::test]
    async fn collect_latest_actions_for_stranger_tenant_is_empty() {
        let store = migrated_store().await;
        let stranger = argo_types::TenantId::new();
        let actions = collect_latest_actions_for_tenant(&store, stranger, None)
            .await
            .expect("collect");
        assert!(actions.is_empty());
    }
}
