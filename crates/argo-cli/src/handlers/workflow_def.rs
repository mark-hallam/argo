//! `argo workflow def ...` handler suite (Stream
//! `mission-pm-workflow-rollback-cli`).
//!
//! Read-side surfaces (`list`, `show`, `versions`, `diff`) require
//! the `workflow.read` capability; mutating surfaces (`rollback`,
//! `archive`) require `workflow.edit`. Each handler calls
//! `argo_store::workflow_definition` directly — the CLI does not
//! round-trip through the gateway because `argo` ships as a single
//! binary (see CLAUDE.md core invariants).
//!
//! The `diff` subcommand uses a self-contained naive line-diff
//! (longest-common-subsequence based) so we do not add a new
//! workspace dependency on `similar`. The brief permitted this
//! fallback ("falls back to a naive line-diff if `similar` is
//! unavailable").
//!
//! Handlers are split into one-shot inner functions that take an
//! injected [`WorkflowDefContext`] so the unit tests can drive each
//! capability + audit + DB path without spawning the real binary
//! (see `workflow_def_tests`). The public [`run`] wrapper opens the
//! context once per CLI invocation.
use crate::cli::WorkflowDefCmd;
use crate::handlers::worker_create_store::open_workspace_store;
use crate::handlers::workflow_def_diff::{compute_line_diff, format_unified_diff};
use crate::handlers::workflow_def_io::{confirm_or_abort, print_json, print_text};
use crate::handlers::workflow_def_query::list_workflow_heads;
use crate::handlers::workflow_def_render::{
    definition_json, diff_endpoint_json, version_row_json,
};
use argo_audit::{AuditKind, AuditSink};
use argo_capability::{workflow_edit_capability, workflow_read_capability, CapabilityEngine};
use argo_store::workflow_definition::{
    self as wd, WorkflowDefinition, WorkflowDefinitionId, WorkflowLifecycleState,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, Capability, Principal, TenantId};
use serde_json::{json, Value};
use std::sync::Arc;

/// Default operator role for CLI invocations. Mirrors the
/// `worker_freeze` precedent: local operators bind to `owner` and
/// inherit the default capability presets.
const CLI_ROLE: &str = "owner";

/// Entry point dispatched from `handlers::workflow::run`.
pub async fn run(cmd: WorkflowDefCmd) -> ArgoResult<()> {
    let ctx = WorkflowDefContext::open().await?;
    dispatch(&ctx, cmd).await
}

async fn dispatch(ctx: &WorkflowDefContext, cmd: WorkflowDefCmd) -> ArgoResult<()> {
    match cmd {
        WorkflowDefCmd::List { vertical, json } => list(ctx, &vertical, json).await,
        WorkflowDefCmd::Show {
            vertical,
            name,
            version,
            json,
        } => show(ctx, &vertical, &name, version.as_deref(), json).await,
        WorkflowDefCmd::Versions {
            vertical,
            name,
            json,
        } => versions(ctx, &vertical, &name, json).await,
        WorkflowDefCmd::Rollback {
            vertical,
            name,
            to,
            yes,
            json,
        } => rollback(ctx, &vertical, &name, &to, yes, json).await,
        WorkflowDefCmd::Diff {
            vertical,
            name,
            version_a,
            version_b,
            json,
        } => diff(ctx, &vertical, &name, &version_a, &version_b, json).await,
        WorkflowDefCmd::Archive {
            vertical,
            name,
            version,
            yes,
            json,
        } => archive(ctx, &vertical, &name, &version, yes, json).await,
    }
}

// ── Read surfaces ─────────────────────────────────────────────────

pub(crate) async fn list(ctx: &WorkflowDefContext, vertical: &str, json: bool) -> ArgoResult<()> {
    ctx.require(&workflow_read_capability())?;
    let rows = list_workflow_heads(ctx.store.pool(), ctx.tenant, vertical).await?;
    if json {
        let arr: Vec<Value> = rows.iter().map(definition_json).collect();
        print_json(&json!({
            "vertical_id": vertical,
            "count": arr.len(),
            "items": arr,
        }))
    } else if rows.is_empty() {
        print_text(&format!(
            "(no workflow definitions for vertical '{vertical}')"
        ))
    } else {
        let mut out = format!(
            "{:<36}  {:<24}  {:>7}  {:<10}  created_at\n",
            "id", "name", "version", "state"
        );
        for def in &rows {
            out.push_str(&format!(
                "{:<36}  {:<24}  {:>7}  {:<10}  {}\n",
                def.id.as_str(),
                def.name,
                def.version,
                def.lifecycle_state.as_str(),
                def.created_at,
            ));
        }
        print_text(out.trim_end())
    }
}

pub(crate) async fn show(
    ctx: &WorkflowDefContext,
    vertical: &str,
    name: &str,
    version_id: Option<&str>,
    json: bool,
) -> ArgoResult<()> {
    ctx.require(&workflow_read_capability())?;
    let def = match version_id {
        Some(vid) => load_version_strict(ctx, vertical, name, vid).await?,
        None => wd::get_head(ctx.store.pool(), ctx.tenant, vertical, name)
            .await?
            .ok_or_else(|| {
                ArgoError::NotFound(format!(
                    "no head workflow definition for ({vertical}, {name})"
                ))
            })?,
    };
    if json {
        print_json(&definition_json(&def))
    } else {
        print_text(def.definition_toml.trim_end())
    }
}

pub(crate) async fn versions(
    ctx: &WorkflowDefContext,
    vertical: &str,
    name: &str,
    json: bool,
) -> ArgoResult<()> {
    ctx.require(&workflow_read_capability())?;
    let rows = wd::list_versions(ctx.store.pool(), ctx.tenant, vertical, name).await?;
    if json {
        let arr: Vec<Value> = rows.iter().map(version_row_json).collect();
        print_json(&json!({
            "vertical_id": vertical,
            "name": name,
            "count": arr.len(),
            "items": arr,
        }))
    } else if rows.is_empty() {
        print_text(&format!(
            "(no workflow definition versions for ({vertical}, {name}))"
        ))
    } else {
        let mut out = format!(
            "{:>7}  {:<36}  {:<10}  {:<28}  modified_by\n",
            "version", "id", "state", "created_at"
        );
        for def in &rows {
            out.push_str(&format!(
                "{:>7}  {:<36}  {:<10}  {:<28}  {}\n",
                def.version,
                def.id.as_str(),
                def.lifecycle_state.as_str(),
                def.created_at,
                def.modified_by,
            ));
        }
        print_text(out.trim_end())
    }
}

// ── Mutate surfaces ──────────────────────────────────────────────

pub(crate) async fn rollback(
    ctx: &WorkflowDefContext,
    vertical: &str,
    name: &str,
    to: &str,
    yes: bool,
    json: bool,
) -> ArgoResult<()> {
    ctx.require(&workflow_edit_capability())?;
    let target = load_version_strict(ctx, vertical, name, to).await?;
    let head_before = wd::get_head(ctx.store.pool(), ctx.tenant, vertical, name)
        .await?
        .map(|d| d.id.0);
    let prompt = format!(
        "Set head of ({vertical}, {name}) to version {} ({})? [y/N] ",
        target.version,
        target.id.as_str()
    );
    if !yes && !confirm_or_abort(&prompt)? {
        return print_aborted(json, "rollback aborted");
    }
    wd::set_head(ctx.store.pool(), ctx.tenant, vertical, name, &target.id).await?;
    ctx.emit_audit(
        AuditKind::WorkflowHeadChanged,
        vertical,
        name,
        target.id.as_str(),
    )
    .await;
    if json {
        print_json(&json!({
            "ok": true,
            "tenant_id": ctx.tenant.to_string(),
            "vertical_id": vertical,
            "name": name,
            "head_version_id": target.id.0,
            "head_version": target.version,
            "previous_head_version_id": head_before,
        }))
    } else {
        print_text(&format!(
            "rollback OK: head of ({vertical}, {name}) is now version {} ({})",
            target.version,
            target.id.as_str()
        ))
    }
}

pub(crate) async fn archive(
    ctx: &WorkflowDefContext,
    vertical: &str,
    name: &str,
    version_id: &str,
    yes: bool,
    json: bool,
) -> ArgoResult<()> {
    ctx.require(&workflow_edit_capability())?;
    let target = load_version_strict(ctx, vertical, name, version_id).await?;
    let prompt = format!(
        "Archive version {} ({}) of ({vertical}, {name})? [y/N] ",
        target.version,
        target.id.as_str()
    );
    if !yes && !confirm_or_abort(&prompt)? {
        return print_aborted(json, "archive aborted");
    }
    wd::archive(ctx.store.pool(), &target.id).await?;
    ctx.emit_audit(
        AuditKind::WorkflowVersionArchived,
        vertical,
        name,
        target.id.as_str(),
    )
    .await;
    if json {
        print_json(&json!({
            "ok": true,
            "tenant_id": ctx.tenant.to_string(),
            "vertical_id": vertical,
            "name": name,
            "version_id": target.id.0,
            "version": target.version,
            "lifecycle_state": WorkflowLifecycleState::Archived.as_str(),
        }))
    } else {
        print_text(&format!(
            "archive OK: version {} ({}) of ({vertical}, {name}) is now archived",
            target.version,
            target.id.as_str()
        ))
    }
}

pub(crate) async fn diff(
    ctx: &WorkflowDefContext,
    vertical: &str,
    name: &str,
    version_a: &str,
    version_b: &str,
    json: bool,
) -> ArgoResult<()> {
    ctx.require(&workflow_read_capability())?;
    let a = load_version_strict(ctx, vertical, name, version_a).await?;
    let b = load_version_strict(ctx, vertical, name, version_b).await?;
    let summary = compute_line_diff(&a.definition_toml, &b.definition_toml);
    if json {
        print_json(&json!({
            "vertical_id": vertical,
            "name": name,
            "version_a": diff_endpoint_json(&a),
            "version_b": diff_endpoint_json(&b),
            "added": summary.added,
            "removed": summary.removed,
            "changed": summary.changed,
        }))
    } else {
        let header = format!(
            "--- ({vertical}, {name}) v{} {}\n+++ ({vertical}, {name}) v{} {}\n",
            a.version,
            a.id.as_str(),
            b.version,
            b.id.as_str(),
        );
        let body = format_unified_diff(&a.definition_toml, &b.definition_toml);
        print_text(&format!("{header}{body}"))
    }
}

// ── Shared helpers ───────────────────────────────────────────────

/// Per-invocation context: open store, resolved tenant, role-bound
/// principal, and an audit sink stamped with the workspace tenant.
pub(crate) struct WorkflowDefContext {
    pub(crate) store: Arc<SessionStore>,
    pub(crate) tenant: TenantId,
    pub(crate) principal: Principal,
    pub(crate) engine: CapabilityEngine,
    pub(crate) actor: String,
}

impl WorkflowDefContext {
    async fn open() -> ArgoResult<Self> {
        let (store, tenant, cwd) = open_workspace_store().await?;
        let principal = local_principal();
        let engine = CapabilityEngine::with_workspace_defaults(&cwd);
        engine.bind(principal.clone(), CLI_ROLE.into())?;
        let actor = principal_label(&principal);
        Ok(Self {
            store: Arc::new(store),
            tenant,
            principal,
            engine,
            actor,
        })
    }

    fn require(&self, cap: &Capability) -> ArgoResult<()> {
        self.engine.check(&self.principal, cap)
    }

    pub(crate) async fn emit_audit(
        &self,
        kind: AuditKind,
        vertical: &str,
        name: &str,
        version_id: &str,
    ) {
        let payload = json!({
            "actor": self.actor,
            "tenant_id": self.tenant.to_string(),
            "vertical_id": vertical,
            "name": name,
            "version_id": version_id,
        });
        let sink = AuditSink::new_with_tenant(Arc::clone(&self.store), self.tenant.to_string());
        if let Err(e) = sink
            .write(None, kind, payload, Some(self.actor.clone()))
            .await
        {
            tracing::warn!(
                target: "argo-cli::workflow_def",
                error = %e,
                kind = %kind.label(),
                vertical_id = %vertical,
                name = %name,
                "workflow audit write failed",
            );
            return;
        }
        let _ = sink.flush_pending().await;
    }
}

async fn load_version_strict(
    ctx: &WorkflowDefContext,
    vertical: &str,
    name: &str,
    version_id: &str,
) -> ArgoResult<WorkflowDefinition> {
    let id = WorkflowDefinitionId(version_id.to_string());
    let def = wd::get(ctx.store.pool(), &id).await?.ok_or_else(|| {
        ArgoError::NotFound(format!(
            "no workflow definition version '{version_id}' for ({vertical}, {name})"
        ))
    })?;
    if def.tenant_id != ctx.tenant || def.vertical_id != vertical || def.name != name {
        return Err(ArgoError::NotFound(format!(
            "no workflow definition version '{version_id}' for ({vertical}, {name})"
        )));
    }
    Ok(def)
}

fn local_principal() -> Principal {
    let os_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "operator".into());
    Principal::Local { os_user }
}

fn principal_label(principal: &Principal) -> String {
    match principal {
        Principal::Local { os_user } => os_user.clone(),
        Principal::Gateway { adapter, identity } => format!("{adapter}:{identity}"),
        Principal::Acp { harness, identity } => format!("{harness}:{identity}"),
        Principal::Forge => "forge".into(),
    }
}

fn print_aborted(json: bool, msg: &str) -> ArgoResult<()> {
    if json {
        print_json(&json!({"ok": false, "aborted": true, "reason": msg}))
    } else {
        print_text(msg)
    }
}

#[cfg(test)]
#[path = "workflow_def_tests.rs"]
mod tests;
