//! Dormant, default-OFF per-action execution-graduation resolve seam
//! (UF Phase-B · B3, DEC-037).
//!
//! A hired teammate is draft-only at birth: every generated action is
//! `executor = "human"`, so [`resolve_executor`] hands it to a human.
//! DEC-037 greenlit the deliberate post-hire grant — a **proven**
//! teammate may have one action graduated onto a real executor (MCP
//! today). The grant lives in [`argo_store::executor_promotion`]; this
//! module is the read side that turns a stored grant into the executor
//! the dispatch path runs.
//!
//! # Dormancy (the load-bearing safety property)
//!
//! Honouring a promotion is gated behind the `ARGO_EXECUTION_GRADUATION`
//! environment flag, which is **unset by default**. With the flag unset
//! — the shipped default — [`resolve_executor_with_promotion`] is
//! byte-for-byte identical to [`resolve_executor`]: the promotion table
//! is never consulted and the manifest's declared executor (normally
//! `human`) is returned. The live flip is operator-gated and lands in a
//! follow-up brief only after the adversarial blast-radius review
//! (DEC-037). Approval STAYS on sensitive verbs either way — promotion
//! changes *who* executes after approval, never that approval is
//! required.

// DEC-041 piece 2: this execution-graduation seam is now WIRED into the
// live dispatch callsite (`worker_actions_dispatch::invoke_executor_for`),
// gated behind `ARGO_EXECUTION_GRADUATION` (default-OFF). With the flag
// unset the dispatch callsite calls `resolve_executor` directly, so this
// module's promotion path stays dormant; flag-on, a graduated action
// resolves to its real executor here. The blast-radius review must be
// re-run against this wired path before the operator flips the flag live.

use super::worker_actions_executor::{resolve_executor, Executor};
use super::worker_actions_executor_stubs::McpExecutor;
use super::worker_actions_executor_timing::TimedExecutor;
use argo_store::executor_promotion::{
    self, PromotionRecord, PROMOTABLE_KIND_CU_MACOS, PROMOTABLE_KIND_DOCUMENT,
    PROMOTABLE_KIND_MCP,
};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use argo_vertical::{
    ActionDeclaration, ExecutorDeclaration, ExecutorTagged, McpServerDeclaration, VerticalManifest,
};
use std::collections::HashMap;

/// Environment flag that arms per-action execution graduation. Unset
/// (the shipped default) keeps every promotion dormant.
pub(crate) const EXECUTION_GRADUATION_ENV: &str = "ARGO_EXECUTION_GRADUATION";

/// True only when `ARGO_EXECUTION_GRADUATION` is explicitly truthy.
pub(crate) fn execution_graduation_enabled() -> bool {
    graduation_flag(std::env::var(EXECUTION_GRADUATION_ENV).ok().as_deref())
}

/// Pure flag parse, factored out so the gate can be unit-tested without
/// mutating process environment (which races across parallel tests).
///
/// Mirrors `boot_spawns::parse_autofire_flag` (the `ARGO_DUTY_AUTOFIRE`
/// parser): case-insensitive, whitespace-trimmed, accepting
/// `1`/`true`/`yes`/`on`. Aligning the two highest-risk Phase-B flags
/// removes a footgun — an operator setting `ON` or ` true ` at the
/// live flip would otherwise leave graduation silently dormant. Still
/// default-OFF: `None` (unset) and any non-truthy value → `false`.
fn graduation_flag(value: Option<&str>) -> bool {
    value
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false)
}

/// Resolve the executor for `action_type`, honouring a per-action
/// promotion **only** when execution graduation is enabled. See the
/// module docs: with the flag unset this is byte-for-byte
/// [`resolve_executor`].
///
/// This is the operator-gated seam. It is wired into the live dispatch
/// path by the follow-up live-flip brief (after the DEC-037
/// blast-radius review); until then it is exercised only by this
/// module's tests, so the production resolve path stays untouched.
pub(crate) async fn resolve_executor_with_promotion(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: &str,
    vertical: &VerticalManifest,
    action_type: &str,
) -> ArgoResult<Box<dyn Executor>> {
    resolve_with_graduation(
        store,
        tenant,
        worker_id,
        vertical,
        action_type,
        execution_graduation_enabled(),
    )
    .await
}

/// Resolve for an OPERATOR-APPROVED action (the cu5 "Approve & run"
/// path): promotions are honoured regardless of
/// `ARGO_EXECUTION_GRADUATION`, because the operator's explicit,
/// authenticated approval IS the authorization — the same principle
/// #2818 merged for executing the manifest executor. The DEC-051
/// risk-tiered floor still gates committing computer-use operations
/// inside the run loop, so this never grants unattended host control.
pub(crate) async fn resolve_executor_for_approved(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: &str,
    vertical: &VerticalManifest,
    action_type: &str,
) -> ArgoResult<Box<dyn Executor>> {
    resolve_with_graduation(store, tenant, worker_id, vertical, action_type, true).await
}

/// Inner resolve with the graduation flag passed explicitly. Keeps the
/// honour/dormant decision deterministically testable without touching
/// `std::env`.
async fn resolve_with_graduation(
    store: &SessionStore,
    tenant: TenantId,
    worker_id: &str,
    vertical: &VerticalManifest,
    action_type: &str,
    enabled: bool,
) -> ArgoResult<Box<dyn Executor>> {
    // Dormant default-OFF: do not even consult the promotion table.
    if !enabled {
        return resolve_executor(vertical, action_type);
    }
    let promo = executor_promotion::get(
        store.pool(),
        &tenant.to_string(),
        worker_id,
        action_type,
    )
    .await?;
    match promo {
        // Only MCP grants are honoured in v1; anything else (or a
        // grant for a different tenant, which `get` never returns)
        // falls back to the manifest default.
        Some(p) if p.executor_kind == PROMOTABLE_KIND_MCP => {
            let base = lookup_action_decl(vertical, action_type)?;
            let servers = manifest_servers(vertical);
            Ok(promoted_mcp_executor(&p, base, servers))
        }
        // DEC-041 A: graduate to the side-effect-free internal document
        // executor — the teammate produces an internal artifact and the
        // action self-completes (no external blast radius). Wrapped in
        // `TimedExecutor` so dispatch records latency like every other path.
        Some(p) if p.executor_kind == PROMOTABLE_KIND_DOCUMENT => Ok(Box::new(
            TimedExecutor::new(Box::new(super::worker_actions_executor_document::DocumentExecutor::new())),
        )),
        // DEC-050/DEC-051: graduate onto the native-macOS computer-use
        // executor driving the granted bundle id. Safe-by-construction:
        // the CU run loop's risk-tiered floor (#2822) holds every
        // committing operation (click / key / launch) for operator
        // approval regardless of this grant, and dispatch only reaches
        // here per-action after the operator recorded the promotion.
        Some(p) if p.executor_kind == PROMOTABLE_KIND_CU_MACOS => {
            let base = lookup_action_decl(vertical, action_type)?;
            Ok(promoted_cu_macos_executor(&p, base))
        }
        _ => resolve_executor(vertical, action_type),
    }
}

/// Build the MCP executor a promoted action dispatches through. Clones
/// the manifest's base declaration (preserving its description and
/// instruction templates) and overrides **only** the `executor` field
/// with the tagged MCP form recorded in the grant. Wrapped in
/// [`TimedExecutor`] so dispatch records latency exactly like
/// [`resolve_executor`] does for a manifest-declared MCP action.
fn promoted_mcp_executor(
    promo: &PromotionRecord,
    base: &ActionDeclaration,
    servers: HashMap<String, McpServerDeclaration>,
) -> Box<dyn Executor> {
    let mut decl = base.clone();
    decl.executor = ExecutorDeclaration::Tagged(ExecutorTagged {
        kind: PROMOTABLE_KIND_MCP.to_string(),
        server: promo.mcp_server.clone(),
        tool: promo.mcp_tool.clone(),
        ..Default::default()
    });
    Box::new(TimedExecutor::new(Box::new(McpExecutor::with_manifest(
        decl, servers,
    ))))
}

/// Build the native-macOS computer-use executor a promoted action
/// dispatches through. Clones the manifest's base declaration and
/// overrides only the `executor` field with the cu-macos tagged form +
/// the granted `target_app`, exactly mirroring what a manifest-declared
/// `computer_use_native_macos` action resolves to (cu3 #2816).
fn promoted_cu_macos_executor(promo: &PromotionRecord, base: &ActionDeclaration) -> Box<dyn Executor> {
    let mut decl = base.clone();
    decl.executor = ExecutorDeclaration::Tagged(ExecutorTagged {
        kind: PROMOTABLE_KIND_CU_MACOS.to_string(),
        target_app: promo.target_app.clone(),
        ..Default::default()
    });
    Box::new(TimedExecutor::new(Box::new(
        super::worker_actions_executor_cu_macos::ComputerUseMacOSExecutor::with_declaration(decl),
    )))
}

fn lookup_action_decl<'a>(
    vertical: &'a VerticalManifest,
    action_type: &str,
) -> ArgoResult<&'a ActionDeclaration> {
    vertical
        .actions
        .as_ref()
        .and_then(|a| a.actions.get(action_type))
        .ok_or_else(|| {
            ArgoError::Provider(format!(
                "execution-graduation: vertical '{}' does not declare action_type \
                 '{action_type}' — cannot resolve the promoted executor",
                vertical.vertical.id
            ))
        })
}

fn manifest_servers(vertical: &VerticalManifest) -> HashMap<String, McpServerDeclaration> {
    vertical
        .mcp_servers
        .as_ref()
        .map(|s| s.servers.clone().into_iter().collect())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "worker_actions_executor_promotion_tests.rs"]
mod tests;
