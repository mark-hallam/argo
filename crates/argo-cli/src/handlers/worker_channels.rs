//! Stream `claude-worker-channels-cli`.
//!
//! `argo worker channels add/remove/list <id>` mutates the per-worker
//! `Vec<NotifyChannelRef>` lazily — channels live as JSON inside the
//! worker row, so this is a read-mutate-write pattern over
//! [`IdentityStore::update_channels`]. v1 ships three sub-commands:
//!
//! - `add <worker-id> <kind:identifier>` — appends, idempotent on
//!   exact dup, capped at [`MAX_CHANNELS_PER_WORKER`]
//! - `remove <worker-id> <kind:identifier>` — removes by spec
//! - `list <worker-id>` — markdown by default, `--json` for tooling

use crate::home::{db_path, ensure_dir, resolve_home};
use argo_identity::{IdentityStore, NotifyChannelRef, WorkerId};
use argo_store::SessionStore;
use argo_types::{ArgoError, ArgoResult, TenantId};
use std::path::PathBuf;
use std::str::FromStr;
use uuid::Uuid;

/// Maximum channels stored on a single worker row.
pub const MAX_CHANNELS_PER_WORKER: usize = 16;

/// Channel kinds the worker can be tagged with. Most have a matching
/// arm in `worker_actions_notify::EnvChannelSender::send_via`;
/// `whatsapp` is permitted as metadata even though the dispatcher does
/// not yet route to it (a `NotifyChannel` impl lives in `argo-channels`
/// for when it gets wired). This brief is metadata-only — we do not
/// *send* — so allowing the kind here matches the brief's own example
/// (`whatsapp:+15551234567`).
const SUPPORTED_KINDS: &[&str] = &[
    "push", "webinbox", "slack", "email", "sms", "discord", "chat", "sse", "whatsapp",
];

/// `argo worker channels add <worker-id> <kind:identifier> --tenant <t>`.
pub async fn add(tenant: &str, worker_id: &str, spec: &str, json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let worker = parse_worker(worker_id)?;
    let new_channel = parse_channel_spec(spec)?;
    let owned = open_identity_store().await?;
    let mut current = load_channels(&owned, tenant_id, worker, worker_id, tenant).await?;
    let outcome = add_to(&mut current, new_channel)?;
    if outcome.changed {
        owned.update_channels(tenant_id, worker, &current).await?;
    }
    print_add_outcome(worker_id, spec, &outcome, json);
    Ok(())
}

/// `argo worker channels remove <worker-id> <kind:identifier> --tenant <t>`.
pub async fn remove(
    tenant: &str,
    worker_id: &str,
    channel_id: &str,
    json: bool,
) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let worker = parse_worker(worker_id)?;
    let target = parse_channel_spec(channel_id)?;
    let owned = open_identity_store().await?;
    let mut current = load_channels(&owned, tenant_id, worker, worker_id, tenant).await?;
    let before = current.len();
    current.retain(|ch| !channel_matches(ch, &target));
    if current.len() == before {
        return Err(ArgoError::Provider(format!(
            "no channel matches '{channel_id}' on worker {worker_id}"
        )));
    }
    owned.update_channels(tenant_id, worker, &current).await?;
    print_remove_outcome(worker_id, channel_id, current.len(), json);
    Ok(())
}

/// `argo worker channels list <worker-id> --tenant <t> [--json]`.
pub async fn list(tenant: &str, worker_id: &str, json: bool) -> ArgoResult<()> {
    let tenant_id = parse_tenant(tenant)?;
    let worker = parse_worker(worker_id)?;
    let owned = open_identity_store().await?;
    let current = load_channels(&owned, tenant_id, worker, worker_id, tenant).await?;
    print_list(worker_id, &current, json);
    Ok(())
}

/// Outcome record for [`add_to`].
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct AddOutcome {
    pub changed: bool,
    pub total: usize,
    pub duplicate: bool,
}

/// Append `new_channel` to `current`, enforcing the per-worker cap and
/// idempotency. Pure function — pulled out for direct unit testing.
pub(crate) fn add_to(
    current: &mut Vec<NotifyChannelRef>,
    new_channel: NotifyChannelRef,
) -> ArgoResult<AddOutcome> {
    if current.iter().any(|ch| channel_matches(ch, &new_channel)) {
        return Ok(AddOutcome {
            changed: false,
            total: current.len(),
            duplicate: true,
        });
    }
    if current.len() >= MAX_CHANNELS_PER_WORKER {
        return Err(ArgoError::Schema(format!(
            "worker has {MAX_CHANNELS_PER_WORKER} channels (cap reached); remove one first"
        )));
    }
    current.push(new_channel);
    Ok(AddOutcome {
        changed: true,
        total: current.len(),
        duplicate: false,
    })
}

/// Two channels match for add-dedupe + remove-target purposes when the
/// `(channel, target)` pair is identical. `risk_level` is a presentation
/// filter, not part of identity.
pub(crate) fn channel_matches(left: &NotifyChannelRef, right: &NotifyChannelRef) -> bool {
    left.channel == right.channel && left.target == right.target
}

/// Parse `kind:identifier` into a [`NotifyChannelRef`], rejecting
/// unknown kinds and empty / malformed identifiers per kind.
pub(crate) fn parse_channel_spec(spec: &str) -> ArgoResult<NotifyChannelRef> {
    let (kind_raw, target) = spec.split_once(':').ok_or_else(|| {
        ArgoError::Schema(format!(
            "channel spec '{spec}' must be '<kind>:<identifier>'"
        ))
    })?;
    let kind = kind_raw.trim();
    let target = target.trim();
    if kind.is_empty() || target.is_empty() {
        return Err(ArgoError::Schema(format!(
            "channel spec '{spec}' must be '<kind>:<identifier>' with both halves set"
        )));
    }
    if !SUPPORTED_KINDS.contains(&kind) {
        return Err(ArgoError::Schema(format!(
            "channel kind '{kind}' is not supported (one of: {})",
            SUPPORTED_KINDS.join(", ")
        )));
    }
    validate_target(kind, target)?;
    Ok(NotifyChannelRef::new(kind, target))
}

/// Per-kind shape check on the identifier half.
fn validate_target(kind: &str, target: &str) -> ArgoResult<()> {
    match kind {
        "email" => check_email(target),
        "whatsapp" | "sms" => check_e164(kind, target),
        // Slack channel ids, webinbox tokens, push device ids, discord
        // webhook ids, chat URLs, sse session ids — too varied to
        // shape-check beyond non-empty (already enforced upstream).
        _ => Ok(()),
    }
}

fn check_email(target: &str) -> ArgoResult<()> {
    let (local, domain) = target.split_once('@').ok_or_else(|| {
        ArgoError::Schema(format!(
            "email target '{target}' is not a valid address (missing '@')"
        ))
    })?;
    if local.is_empty() || !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.')
    {
        return Err(ArgoError::Schema(format!(
            "email target '{target}' is not a valid address"
        )));
    }
    Ok(())
}

fn check_e164(kind: &str, target: &str) -> ArgoResult<()> {
    let stripped = target.strip_prefix('+').ok_or_else(|| {
        ArgoError::Schema(format!(
            "{kind} target '{target}' must be E.164 format (e.g. +15551234567)"
        ))
    })?;
    let len = stripped.len();
    if !(7..=15).contains(&len) || !stripped.chars().all(|c| c.is_ascii_digit()) {
        return Err(ArgoError::Schema(format!(
            "{kind} target '{target}' must be E.164 format (e.g. +15551234567)"
        )));
    }
    Ok(())
}

async fn load_channels(
    owned: &OwnedIdentityStore,
    tenant_id: TenantId,
    worker: WorkerId,
    worker_id_raw: &str,
    tenant_raw: &str,
) -> ArgoResult<Vec<NotifyChannelRef>> {
    let row = owned.get_worker(tenant_id, worker).await?.ok_or_else(|| {
        ArgoError::Provider(format!(
            "worker {worker_id_raw} not found in tenant {tenant_raw}"
        ))
    })?;
    Ok(row.channels)
}

fn parse_tenant(tenant: &str) -> ArgoResult<TenantId> {
    Uuid::parse_str(tenant)
        .map(TenantId)
        .map_err(|e| ArgoError::Schema(format!("invalid tenant id '{tenant}': {e}")))
}

fn parse_worker(worker_id: &str) -> ArgoResult<WorkerId> {
    WorkerId::from_str(worker_id)
        .map_err(|e| ArgoError::Schema(format!("invalid worker id '{worker_id}': {e}")))
}

async fn open_identity_store() -> ArgoResult<OwnedIdentityStore> {
    let cwd = std::env::current_dir().map_err(|e| ArgoError::Provider(format!("cwd: {e}")))?;
    let home = resolve_home(&cwd)?;
    ensure_dir(&home)?;
    let store = SessionStore::open(db_path(&home)).await?;
    store.migrate().await?;
    Ok(OwnedIdentityStore { store, _home: home })
}

/// Owns the underlying [`SessionStore`] so the borrow that
/// [`IdentityStore`] holds stays valid for the whole CLI invocation
/// (mirrors the pattern from [`crate::handlers::worker_charter`]).
pub(crate) struct OwnedIdentityStore {
    store: SessionStore,
    _home: PathBuf,
}

impl OwnedIdentityStore {
    pub(crate) async fn get_worker(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
    ) -> ArgoResult<Option<argo_identity::Worker>> {
        IdentityStore::new(&self.store)
            .get_worker(tenant_id, worker_id)
            .await
    }

    pub(crate) async fn update_channels(
        &self,
        tenant_id: TenantId,
        worker_id: WorkerId,
        channels: &[NotifyChannelRef],
    ) -> ArgoResult<()> {
        IdentityStore::new(&self.store)
            .update_channels(tenant_id, worker_id, channels)
            .await
    }
}

fn print_add_outcome(worker_id: &str, spec: &str, outcome: &AddOutcome, json: bool) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": worker_id,
            "spec": spec,
            "added": outcome.changed,
            "duplicate": outcome.duplicate,
            "total": outcome.total,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else if outcome.duplicate {
        eprintln!("worker {worker_id}: channel '{spec}' already present (no-op)");
    } else {
        println!(
            "worker {worker_id}: added channel '{spec}' (now {} of {MAX_CHANNELS_PER_WORKER})",
            outcome.total
        );
    }
}

fn print_remove_outcome(worker_id: &str, channel_id: &str, remaining: usize, json: bool) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": worker_id,
            "removed": channel_id,
            "remaining": remaining,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("worker {worker_id}: removed channel '{channel_id}' ({remaining} remaining)");
    }
}

fn print_list(worker_id: &str, channels: &[NotifyChannelRef], json: bool) {
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "worker_id": worker_id,
            "channels": channels,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".into())
        );
        return;
    }
    if channels.is_empty() {
        println!("worker {worker_id}: (no channels)");
        return;
    }
    println!("worker {worker_id} channels ({} of {MAX_CHANNELS_PER_WORKER}):", channels.len());
    for ch in channels {
        let risk = ch.risk_level.as_deref().map(|r| format!(" [risk:{r}]")).unwrap_or_default();
        println!("  - {}:{}{risk}", ch.channel, ch.target);
    }
}

#[cfg(test)]
#[path = "worker_channels_tests.rs"]
mod tests;
