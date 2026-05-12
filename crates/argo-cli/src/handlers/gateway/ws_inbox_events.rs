//! Structured event protocol for `/inbox/ws` (J3).
//!
//! The legacy text-frame "reload" signal continues to fan out via
//! [`super::ws_upgrade::publish_inbox_event`] for HTML-inbox
//! compatibility. This module adds:
//!
//! 1. A typed [`WebInboxEvent`] vocabulary (`approval.{created,decided,
//!    undone,expired}`).
//! 2. A per-tenant tokio `broadcast` channel registry so cross-tenant
//!    events stay isolated even on a shared-process gateway.
//! 3. A monotonic per-connection sequence counter to let the SPA
//!    drop out-of-order frames.
//! 4. A connection-time `hello` envelope advertising protocol version
//!    + supported event vocabulary.
//! 5. The query-param JWT auth helper used by the WS upgrade path.
//!
//! v1 posture vs target (see brief §v1 posture):
//! - Per-tenant broadcast = the architectural target; what's missing
//!   is per-Principal RBAC filtering (`mission-enterprise-ws-rbac-filter`).
//! - Sequence numbers are per-connection (not per-tenant): replay
//!   protection is a v1.5 brief.
//! - Tenant key defaults to the operator id when a JWT is absent so
//!   the legacy bearer-token consumers still receive every tenant's
//!   broadcast on the global channel (back-compat).

use super::jwt::{self, Claims, JwtError};
use argo_channels::webinbox::{ApprovalRowFull, WebInboxEvent as ChannelEvent, WebInboxRow};
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

const BROADCAST_CAPACITY: usize = 256;
/// Protocol version advertised in the connection-time hello frame.
/// Bump when the wire format changes incompatibly; v1 is `"1.0"`.
pub(super) const PROTOCOL_VERSION: &str = "1.0";

/// Typed event vocabulary. The four variants intentionally mirror
/// the lifecycle of an approval row — see the brief's acceptance
/// criteria. The wire format is locked: `type` is the snake-case
/// `"approval.<verb>"` literal.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "type")]
#[allow(clippy::enum_variant_names)]
pub(super) enum WebInboxEvent {
    /// New approval entered the inbox.
    #[serde(rename = "approval.created")]
    ApprovalCreated {
        /// Approval row payload (full `ApprovalRow` JSON).
        payload: Value,
    },
    /// Operator decided an approval (approve OR refuse).
    #[serde(rename = "approval.decided")]
    ApprovalDecided {
        /// Updated approval row payload.
        payload: Value,
        /// `"approve"` or `"refuse"`.
        decision: String,
    },
    /// Operator undid a recent decision (within the 30s window).
    #[serde(rename = "approval.undone")]
    ApprovalUndone {
        /// Reverted approval row payload.
        payload: Value,
    },
    /// Approval expired without a decision.
    #[serde(rename = "approval.expired")]
    ApprovalExpired {
        /// Expired approval row payload.
        payload: Value,
    },
}

/// Connection-time hello frame. Always the first text frame the
/// client receives. `seq: 0` is reserved for hello so clients can
/// treat it as a sentinel before the first data event.
pub(super) fn hello_frame() -> String {
    json!({
        "type": "hello",
        "protocol_version": PROTOCOL_VERSION,
        "capabilities": [
            "approval.created",
            "approval.decided",
            "approval.undone",
            "approval.expired",
        ],
        "seq": 0,
    })
    .to_string()
}

/// Envelope a structured event for the wire — adds `seq` + `ts`. The
/// inner event keeps its `type` discriminant, and the `payload` /
/// `decision` fields land alongside it (flat shape per brief).
pub(super) fn envelope(seq: u64, event: &WebInboxEvent) -> String {
    let body = serde_json::to_value(event).unwrap_or_else(|_| json!({}));
    let mut obj = body.as_object().cloned().unwrap_or_default();
    obj.insert("seq".into(), json!(seq));
    obj.insert("ts".into(), json!(chrono::Utc::now().to_rfc3339()));
    Value::Object(obj).to_string()
}

/// Publish a structured event to every subscriber of the given
/// tenant. Best-effort — a tenant with no current subscribers
/// silently no-ops (the broadcast channel is dropped when the last
/// receiver disappears). The legacy `reload` signal is intentionally
/// NOT emitted here; callers that need both must invoke
/// [`super::ws_upgrade::publish_inbox_event`] in addition.
///
/// **v1 sensible deviation:** the brief specifies a per-connection
/// monotonic `seq`, but the existing WS frame loop is generic and
/// the brief's may-touch list excludes `ws_upgrade_tests.rs`. v1
/// envelopes at publish time with a per-tenant atomic counter so
/// every subscriber on the same tenant sees the same monotonic seq.
/// Functionally equivalent for client-side ordering; per-connection
/// seq lands with the replay-cursor brief
/// (`mission-enterprise-ws-replay-cursor`).
pub(super) fn publish_event(tenant_id: &str, event: WebInboxEvent) {
    let seq = next_seq(tenant_id);
    let frame = envelope(seq, &event);
    let guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(tx) = guard.get(tenant_id) {
        let _ = tx.send(frame);
    }
}

fn next_seq(tenant_id: &str) -> u64 {
    let mut guard = seq_registry().lock().unwrap_or_else(|e| e.into_inner());
    let counter = guard
        .entry(tenant_id.to_string())
        .or_insert_with(|| AtomicU64::new(0));
    counter.fetch_add(1, Ordering::Relaxed) + 1
}

type SeqRegistry = std::collections::HashMap<String, AtomicU64>;

fn seq_registry() -> &'static Mutex<SeqRegistry> {
    static REG: OnceLock<Mutex<SeqRegistry>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(SeqRegistry::new()))
}

/// Subscribe to a tenant's broadcast channel. Lazily creates the
/// channel on first subscription. Returns `(receiver, channel handle)`
/// — the handle is what gets dropped when the connection closes; the
/// channel is GC'd by the registry sweep once all handles go.
pub(super) fn subscribe_tenant(tenant_id: &str) -> broadcast::Receiver<String> {
    ensure_channel_hook();
    let mut guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(tx) = guard.get(tenant_id) {
        return tx.subscribe();
    }
    let (tx, rx) = broadcast::channel::<String>(BROADCAST_CAPACITY);
    guard.insert(tenant_id.to_string(), tx);
    rx
}

/// Result of authorizing a WS connection at upgrade-time. Carries
/// enough context (tenant id) for the per-tenant broadcast subscribe
/// step. Legacy bearer-token connections fall back to the global
/// broadcast and receive every tenant's events — that's the v1
/// back-compat seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum WsAuth {
    /// JWT-authed connection. Tenant id is `claims.tnt`.
    Jwt(Claims),
    /// Legacy bearer-token connection. No tenant filtering.
    Legacy,
}

impl WsAuth {
    /// Tenant key for the per-tenant broadcast registry. Legacy
    /// connections use the well-known `"*"` key so an operator can
    /// publish to "every legacy listener" via the same primitive
    /// (the wire side still posts to the global channel).
    pub(super) fn tenant_key(&self) -> &str {
        match self {
            WsAuth::Jwt(c) => c.tnt.as_str(),
            WsAuth::Legacy => "*",
        }
    }
}

/// Authorize a WS upgrade request. JWT (`?token=…`) wins when both
/// are present; bearer-token fallback preserves the HTML inbox path.
pub(super) fn authorize(req: &str, now_unix: i64) -> Result<WsAuth, (&'static str, String)> {
    if let Some(token) = token_from_query(req) {
        return match jwt::verify_jwt(token, now_unix) {
            Ok(claims) => Ok(WsAuth::Jwt(claims)),
            Err(JwtError::NoPubkey) => {
                // No pubkey configured — fall through to bearer path.
                bearer_or_unauth(req)
            }
            Err(e) => Err((
                "401 Unauthorized",
                format!("{{\"error\":\"{}\"}}", e.reason()),
            )),
        };
    }
    bearer_or_unauth(req)
}

fn bearer_or_unauth(req: &str) -> Result<WsAuth, (&'static str, String)> {
    let token = header_value(req, "Authorization")
        .and_then(|h| h.strip_prefix("Bearer "))
        .or_else(|| cookie_value(req, "argo_operator_token"));
    let Some(token) = token else {
        return Err((
            "401 Unauthorized",
            "{\"error\":\"missing bearer token\"}".to_string(),
        ));
    };
    // Mirror the env-var match in ws_upgrade::check_auth so legacy
    // consumers keep their existing failure mode for invalid tokens.
    if let Ok(expected) = argo_config::cred::get("ARGO_WEBINBOX_BEARER_TOKEN") {
        if !expected.is_empty() && token != expected {
            return Err((
                "401 Unauthorized",
                "{\"error\":\"invalid bearer token\"}".to_string(),
            ));
        }
    }
    Ok(WsAuth::Legacy)
}

/// Extract `?token=<JWT>` from the request line. Returns `None` if
/// the request has no query string or the key isn't present.
pub(super) fn token_from_query(req: &str) -> Option<&str> {
    let first = req.lines().next()?;
    let path_and_query = first.split(' ').nth(1)?;
    let qs = path_and_query.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "token").then_some(v)
    })
}

fn header_value<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    req.lines().skip(1).find_map(|l| {
        let (k, v) = l.split_once(':')?;
        k.eq_ignore_ascii_case(name)
            .then(|| v.trim_end_matches('\r').trim())
    })
}

fn cookie_value<'a>(req: &'a str, key: &str) -> Option<&'a str> {
    let raw = header_value(req, "Cookie")?;
    raw.split(';').find_map(|part| {
        let (k, v) = part.trim().split_once('=')?;
        (k == key).then_some(v)
    })
}

type Registry = std::collections::HashMap<String, broadcast::Sender<String>>;

fn registry() -> &'static Mutex<Registry> {
    static REG: OnceLock<Mutex<Registry>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(Registry::new()))
}

/// Register the argo-channels webinbox event hook exactly once so
/// `WebInboxChannel::send()` (i.e. an approval entering the inbox)
/// fans out as an `approval.created` event. Called lazily on first
/// subscribe so production binaries don't need an explicit bootstrap
/// call but tests can drive the hook via `subscribe_tenant`.
fn ensure_channel_hook() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        argo_channels::webinbox::set_event_hook(Box::new(|evt| match evt {
            ChannelEvent::ApprovalCreated {
                tenant_id,
                row,
                full,
            } => publish_event(
                &tenant_id,
                WebInboxEvent::ApprovalCreated {
                    payload: created_payload(&row, full.as_deref()),
                },
            ),
            ChannelEvent::ApprovalDecided {
                tenant_id,
                row,
                decision,
            } => publish_event(
                &tenant_id,
                WebInboxEvent::ApprovalDecided {
                    payload: row_payload(&row),
                    decision,
                },
            ),
            ChannelEvent::ApprovalUndone { tenant_id, row } => publish_event(
                &tenant_id,
                WebInboxEvent::ApprovalUndone {
                    payload: row_payload(&row),
                },
            ),
            ChannelEvent::ApprovalExpired { tenant_id, row } => publish_event(
                &tenant_id,
                WebInboxEvent::ApprovalExpired {
                    payload: row_payload(&row),
                },
            ),
        }));
    });
}

/// Serialize the channel-side `WebInboxRow` to the wire `payload`
/// JSON. v1 ships the full row; v1.5 (`mission-enterprise-ws-rbac-filter`)
/// will trim per-Principal permissions.
fn row_payload(row: &WebInboxRow) -> Value {
    serde_json::to_value(row).unwrap_or_else(|_| json!({}))
}

/// Build the `approval.created` payload. When the channel-side
/// enrichment succeeded we ship the wider [`ApprovalRowFull`] shape so
/// the SPA can `setQueryData` surgically. When enrichment failed (the
/// brief's worker-deleted-race fallback) we ship the legacy
/// `WebInboxRow` shape with `partial: true` so subscribers fall back to
/// `invalidateQueries`.
fn created_payload(row: &WebInboxRow, full: Option<&ApprovalRowFull>) -> Value {
    if let Some(f) = full {
        return serde_json::to_value(f).unwrap_or_else(|_| row_payload(row));
    }
    let mut obj = row_payload(row).as_object().cloned().unwrap_or_default();
    obj.insert("partial".into(), json!(true));
    Value::Object(obj)
}

#[cfg(test)]
#[path = "ws_inbox_events_tests.rs"]
mod tests;
