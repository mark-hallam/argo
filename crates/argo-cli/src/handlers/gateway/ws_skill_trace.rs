//! Live skill-trace WebSocket handler — Track L Phase 1 L5.
//!
//! Surfaces per-node IR trace events emitted by the runtime
//! ([`argo_agent_runner::instrumentation::ir_trace`]) over a
//! `/skill-trace/<skill_id>/<run_id>` WebSocket so the canvas
//! live-trace overlay can pulse the currently-executing node,
//! mark completed nodes with a green check, and mark failed nodes
//! with a red X.
//!
//! ## Wire shape
//!
//! - Route: `GET /skill-trace/<skill_id>/<run_id>?token=<JWT>` upgraded
//!   to WebSocket via the existing `ws_upgrade` accept loop.
//! - Auth: JWT only (the legacy global-bearer fallback the inbox WS
//!   path supports is rejected here — every subscriber needs a tenant
//!   claim so the per-`(tenant, skill, run)` broadcast key holds).
//! - Hello frame (always first, `seq: 0`):
//!   `{type:"hello", protocol_version:"1.0",
//!     topic:"skill-trace.<skill>.<run>", capabilities:[...], seq:0}`.
//! - Data frames carry the brief vocabulary verbatim — `node.entered`,
//!   `node.completed`, `node.failed` — plus a monotonic per-topic
//!   `seq` and an RFC-3339 `ts`.
//!
//! ## Architecture
//!
//! A per-`(tenant_id, skill_id, run_id)` `tokio::broadcast` channel
//! registry holds one channel per active topic. The runtime side
//! emits [`TraceEvent`]s; this module's lazily-installed hook
//! ([`ensure_trace_hook`]) fans events to the matching channel.
//!
//! v1 sensible deviations vs the L0 brief:
//!
//! - **Per-tenant key included**: the brief spec says "topic
//!   `skill-trace.<skill_id>.<run_id>`". Internally we key the
//!   broadcast registry by `(tenant_id, skill_id, run_id)` so two
//!   tenants can never share state on the same `(skill, run)` pair.
//!   The wire-side topic string still surfaces unchanged in the hello
//!   frame.
//! - **JWT-only auth**: the brief doesn't pin auth mode; we mirror
//!   L4-gw's tenant-scope guarantee. A future operator-token path can
//!   land additively if needed.
//! - **Per-topic seq**: same shape as `ws_inbox_events`' per-tenant
//!   seq, scoped to one (tenant, skill, run) so disconnect/reconnect
//!   restarts at 1 without ambiguity.

use super::jwt::{self, Claims, JwtError};
use argo_agent_runner::instrumentation::ir_trace::{self, TraceEvent};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;

const BROADCAST_CAPACITY: usize = 256;
/// Protocol version advertised in the connection-time hello frame.
pub(super) const PROTOCOL_VERSION: &str = "1.0";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TopicKey {
    tenant_id: String,
    skill_id: String,
    run_id: String,
}

impl TopicKey {
    fn from_event(evt: &TraceEvent) -> Self {
        Self {
            tenant_id: evt.tenant_id.clone(),
            skill_id: evt.skill_id.clone(),
            run_id: evt.run_id.clone(),
        }
    }
}

type Registry = HashMap<TopicKey, broadcast::Sender<String>>;

fn registry() -> &'static Mutex<Registry> {
    static REG: OnceLock<Mutex<Registry>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(Registry::new()))
}

type SeqMap = HashMap<TopicKey, AtomicU64>;

fn seq_registry() -> &'static Mutex<SeqMap> {
    static REG: OnceLock<Mutex<SeqMap>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(SeqMap::new()))
}

fn next_seq(key: &TopicKey) -> u64 {
    let mut guard = seq_registry().lock().unwrap_or_else(|e| e.into_inner());
    let counter = guard.entry(key.clone()).or_insert_with(|| AtomicU64::new(0));
    counter.fetch_add(1, Ordering::Relaxed) + 1
}

/// Connection-time hello frame the gateway writes immediately after
/// the WebSocket handshake. `seq: 0` is reserved for hello so clients
/// can treat it as the sentinel before the first data frame.
pub(super) fn hello_frame(skill_id: &str, run_id: &str) -> String {
    json!({
        "type": "hello",
        "protocol_version": PROTOCOL_VERSION,
        "topic": format!("skill-trace.{skill_id}.{run_id}"),
        "capabilities": ["node.entered", "node.completed", "node.failed"],
        "seq": 0,
    })
    .to_string()
}

/// Wrap the wire-format payload in an envelope with `seq` + `ts`.
/// Adjacent fields (`type`, `node_id`, optional `duration_ms` /
/// `error`) land flat so the SPA can switch on `type` directly
/// without unwrapping a nested payload.
fn envelope(seq: u64, payload: &Value) -> String {
    let mut obj = payload.as_object().cloned().unwrap_or_default();
    obj.insert("seq".into(), json!(seq));
    obj.insert("ts".into(), json!(chrono::Utc::now().to_rfc3339()));
    Value::Object(obj).to_string()
}

/// Subscribe to events for one `(tenant_id, skill_id, run_id)` topic.
/// Lazily creates the broadcast channel on first subscribe and
/// guarantees the trace hook is installed.
pub(super) fn subscribe_topic(
    tenant_id: &str,
    skill_id: &str,
    run_id: &str,
) -> broadcast::Receiver<String> {
    ensure_trace_hook();
    let key = TopicKey {
        tenant_id: tenant_id.to_string(),
        skill_id: skill_id.to_string(),
        run_id: run_id.to_string(),
    };
    let mut guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(tx) = guard.get(&key) {
        return tx.subscribe();
    }
    let (tx, rx) = broadcast::channel::<String>(BROADCAST_CAPACITY);
    guard.insert(key, tx);
    rx
}

/// Publish one [`TraceEvent`] to every subscriber of its topic.
/// Best-effort — a topic with no current subscribers silently
/// no-ops. Public to this module so the lazily-installed trace hook
/// and the tests can drive it directly.
pub(super) fn publish_event(event: &TraceEvent) {
    let key = TopicKey::from_event(event);
    let seq = next_seq(&key);
    let frame = envelope(seq, &event.to_wire_json());
    let guard = registry().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(tx) = guard.get(&key) {
        let _ = tx.send(frame);
    }
}

fn ensure_trace_hook() {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    REGISTERED.get_or_init(|| {
        ir_trace::set_trace_hook(Box::new(|evt| publish_event(&evt)));
    });
}

/// Authorize a skill-trace WS upgrade. JWT-only (no legacy bearer
/// fallback) so the tenant claim is always available for the
/// broadcast key. The returned [`Claims`] carries the tenant id via
/// `claims.tnt`.
pub(super) fn authorize(req: &str, now_unix: i64) -> Result<Claims, (&'static str, String)> {
    let token = token_from_query(req).ok_or_else(|| {
        (
            "401 Unauthorized",
            "{\"error\":\"missing JWT token\"}".to_string(),
        )
    })?;
    jwt::verify_jwt(token, now_unix).map_err(|e| match e {
        JwtError::NoPubkey => (
            "401 Unauthorized",
            "{\"error\":\"jwt_pubkey_not_configured\"}".to_string(),
        ),
        other => (
            "401 Unauthorized",
            format!("{{\"error\":\"{}\"}}", other.reason()),
        ),
    })
}

/// Extract `?token=<JWT>` from the request line. Returns `None` when
/// the query string is absent or the key isn't present.
pub(super) fn token_from_query(req: &str) -> Option<&str> {
    let first = req.lines().next()?;
    let path_and_query = first.split(' ').nth(1)?;
    let qs = path_and_query.split_once('?').map(|(_, q)| q)?;
    qs.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == "token").then_some(v)
    })
}

/// Parse `GET /skill-trace/<skill_id>/<run_id>[?...] HTTP/1.1` into
/// the topic coordinates. Returns `None` when the path doesn't match
/// or either component is empty.
pub(super) fn parse_topic_path(req: &str) -> Option<(String, String)> {
    let first = req.lines().next()?;
    let path_and_query = first.split(' ').nth(1)?;
    let path = path_and_query.split('?').next()?;
    let rest = path.strip_prefix("/skill-trace/")?;
    let mut parts = rest.split('/');
    let skill = parts.next()?;
    let run = parts.next()?;
    if skill.is_empty() || run.is_empty() || parts.next().is_some() {
        return None;
    }
    Some((skill.to_string(), run.to_string()))
}

/// True when the request is a skill-trace WS upgrade. The
/// `is_ws_request` umbrella in `ws_upgrade` calls this alongside the
/// existing inbox/audit matchers.
pub(super) fn matches_request(req: &str) -> bool {
    parse_topic_path(req).is_some()
}

#[cfg(test)]
#[path = "ws_skill_trace_tests.rs"]
mod tests;
