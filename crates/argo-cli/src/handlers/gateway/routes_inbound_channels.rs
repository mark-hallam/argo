//! `GET /api/v1/inbound-channels/{kind}/recent` — recent operator
//! replies that landed on the inbound webhook for one channel kind
//! (SMS / Discord / Voice DTMF).
//!
//! Backs the `/settings/inbound-channels` Scene-6 surface: lets the
//! operator confirm at a glance that Twilio / Discord webhooks are
//! actually reaching the gateway after they pasted the URL into the
//! upstream console.
//!
//! Implementation: every inbound reply path
//! (`routes_channels_sms`, `routes_channels_discord`,
//! `routes_channels_voice`) lands in `routes_channels::settle`, which
//! emits one `AuditKind::ApprovalReplied` row with `payload.channel`
//! set to `"sms" | "discord" | "voice"`. So "recent inbound activity"
//! is the most recent `approval.replied` rows whose payload channel
//! matches the requested kind. No new audit verb required.
//!
//! Tenant scope mirrors `routes_audit_feed_scope`: the workspace
//! tenant pulled from `X-Argo-Tenant-Id` (`request_tenant_id`) gates
//! visibility against the stamped `payload.tenant_id`.

use super::routes_audit_feed::RAW_FETCH_CAP;
use super::routes_query::parse_query_kv;
use super::{request_tenant_id, store_unavailable};
use argo_audit::{ExportedRow, TailFilters};
use argo_store::SessionStore;
use serde_json::{json, Value};
use std::sync::Arc;

const APPROVAL_REPLIED_LABEL: &str = "approval.replied";
const DEFAULT_LIMIT: usize = 20;
const MAX_LIMIT: usize = 100;

/// Route entry. Returns `None` when the request is not a recent-feed
/// match, so the caller can fall through to other dispatchers.
pub(super) async fn dispatch_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let kind = parse_inbound_kind(req)?;
    let Some(store) = store else {
        return Some(store_unavailable());
    };
    let tail = req.strip_prefix("GET ").unwrap_or(req);
    let limit = parse_limit(tail);
    let tenant_id = request_tenant_id(req);
    Some(recent_response(store, kind.label(), &tenant_id, limit).await)
}

/// Pure helper extracted for unit tests. Surfaces the channel id
/// parsed off the request path so the dispatcher can short-circuit
/// when the URL is not one of ours.
pub(super) fn parse_inbound_kind(req: &str) -> Option<InboundKind> {
    let rest = req.strip_prefix("GET /api/v1/inbound-channels/")?;
    let (path, _) = rest.split_once(' ')?;
    let (path, _query) = path.split_once('?').unwrap_or((path, ""));
    let mut segments = path.split('/');
    let kind = segments.next()?;
    let tail = segments.next()?;
    if segments.next().is_some() {
        return None;
    }
    if tail != "recent" {
        return None;
    }
    InboundKind::from_str(kind)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InboundKind {
    Sms,
    Discord,
    Voice,
}

impl InboundKind {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "sms" => Some(Self::Sms),
            "discord" => Some(Self::Discord),
            "voice" => Some(Self::Voice),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Sms => "sms",
            Self::Discord => "discord",
            Self::Voice => "voice",
        }
    }
}

fn parse_limit(tail: &str) -> usize {
    parse_query_kv(tail, "limit")
        .and_then(|v| v.parse::<usize>().ok())
        .map_or(DEFAULT_LIMIT, |n| n.clamp(1, MAX_LIMIT))
}

async fn recent_response(
    store: &Arc<SessionStore>,
    channel_label: &str,
    tenant_id: &str,
    limit: usize,
) -> (&'static str, String) {
    let filters = TailFilters {
        kind: Some(APPROVAL_REPLIED_LABEL),
        since_micros: None,
        session_id: None,
    };
    match argo_audit::tail_rows_filtered(store, RAW_FETCH_CAP, &filters).await {
        Ok(mut rows) => {
            // tail_rows_filtered returns ASC inside the LIMIT window —
            // flip to newest-first so the page's "last 20" is the last
            // 20 in time order, not the bottom 20 of an ASC slice.
            rows.sort_by_key(|r| std::cmp::Reverse(r.created_at_micros));
            let items: Vec<Value> = rows
                .into_iter()
                .filter(|row| row_in_channel(row, channel_label))
                .filter(|row| row_in_tenant_scope(row, tenant_id))
                .take(limit)
                .map(summarise_row)
                .collect();
            (
                "200 OK",
                serde_json::to_string(&json!({
                    "kind": channel_label,
                    "items": items,
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"inbound_channels_recent: {e}\"}}"),
        ),
    }
}

fn row_in_channel(row: &ExportedRow, channel_label: &str) -> bool {
    row.payload
        .get("channel")
        .and_then(Value::as_str)
        .is_some_and(|c| c == channel_label)
}

/// Mirror of `routes_audit_feed_scope::row_in_tenant_scope` minus the
/// cross-tenant lane: this surface is operator-facing per-workspace
/// only, no `as_tenant` override. Legacy (un-stamped) rows are
/// hidden by default so a fresh workspace doesn't see global noise.
fn row_in_tenant_scope(row: &ExportedRow, tenant_id: &str) -> bool {
    row.payload
        .get("tenant_id")
        .and_then(Value::as_str)
        .is_some_and(|t| t == tenant_id)
}

fn summarise_row(row: ExportedRow) -> Value {
    let action_id = row.payload.get("action_id").and_then(Value::as_str);
    let replier = row.payload.get("replier").and_then(Value::as_str);
    let reply_kind = row.payload.get("reply_kind").and_then(Value::as_str);
    let reply_body = row.payload.get("reply_body").and_then(Value::as_str);
    let replied_at = row.payload.get("replied_at").and_then(Value::as_str);
    json!({
        "seq": row.seq,
        "id": row.id,
        "action_id": action_id,
        "replier": replier,
        "decision": reply_kind,
        "reply_body": reply_body,
        "replied_at": replied_at,
        "created_at_micros": row.created_at_micros,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use argo_audit::{AuditKind, AuditSink};
    use argo_store::SessionStore;
    use std::sync::Arc;

    fn req(path: &str) -> String {
        format!(
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nX-Argo-Tenant-Id: tenant-a\r\n\r\n"
        )
    }

    #[test]
    fn parses_sms_recent_path() {
        let r = req("/api/v1/inbound-channels/sms/recent");
        assert_eq!(parse_inbound_kind(&r), Some(InboundKind::Sms));
    }

    #[test]
    fn parses_discord_with_query() {
        let r = req("/api/v1/inbound-channels/discord/recent?limit=10");
        assert_eq!(parse_inbound_kind(&r), Some(InboundKind::Discord));
    }

    #[test]
    fn parses_voice_recent_path() {
        let r = req("/api/v1/inbound-channels/voice/recent");
        assert_eq!(parse_inbound_kind(&r), Some(InboundKind::Voice));
    }

    #[test]
    fn rejects_unknown_kind() {
        let r = req("/api/v1/inbound-channels/email/recent");
        assert_eq!(parse_inbound_kind(&r), None);
    }

    #[test]
    fn rejects_wrong_suffix() {
        let r = req("/api/v1/inbound-channels/sms/list");
        assert_eq!(parse_inbound_kind(&r), None);
    }

    #[test]
    fn rejects_extra_segment() {
        let r = req("/api/v1/inbound-channels/sms/recent/extra");
        assert_eq!(parse_inbound_kind(&r), None);
    }

    #[test]
    fn limit_clamps_to_window() {
        assert_eq!(parse_limit("/api/v1/inbound-channels/sms/recent"), 20);
        assert_eq!(
            parse_limit("/api/v1/inbound-channels/sms/recent?limit=5"),
            5
        );
        assert_eq!(
            parse_limit("/api/v1/inbound-channels/sms/recent?limit=9999"),
            MAX_LIMIT
        );
        assert_eq!(
            parse_limit("/api/v1/inbound-channels/sms/recent?limit=0"),
            1
        );
    }

    #[tokio::test]
    async fn recent_returns_only_matching_channel_and_tenant() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let sink = AuditSink::new_with_tenant(Arc::clone(&store), "tenant-a".to_string());
        // 2 sms rows for tenant-a + 1 discord + 1 voice + 1 sms for tenant-b.
        for action in &["act-1", "act-2"] {
            sink.write(
                None,
                AuditKind::ApprovalReplied,
                json!({"action_id": action, "channel": "sms"}),
                None,
            )
            .await
            .unwrap();
        }
        sink.write(
            None,
            AuditKind::ApprovalReplied,
            json!({"action_id": "act-3", "channel": "discord"}),
            None,
        )
        .await
        .unwrap();
        sink.write(
            None,
            AuditKind::ApprovalReplied,
            json!({"action_id": "act-4", "channel": "voice"}),
            None,
        )
        .await
        .unwrap();
        let sink_b = AuditSink::new_with_tenant(Arc::clone(&store), "tenant-b".to_string());
        sink_b
            .write(
                None,
                AuditKind::ApprovalReplied,
                json!({"action_id": "act-5", "channel": "sms"}),
                None,
            )
            .await
            .unwrap();

        let (status, body) = recent_response(&store, "sms", "tenant-a", 20).await;
        assert_eq!(status, "200 OK");
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["kind"], "sms");
        let items = parsed["items"].as_array().expect("items array");
        assert_eq!(items.len(), 2, "got {items:?}");
        for item in items {
            assert!(["act-1", "act-2"].contains(&item["action_id"].as_str().unwrap()));
        }

        let (_, body) = recent_response(&store, "discord", "tenant-a", 20).await;
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(parsed["items"].as_array().unwrap().len(), 1);

        // Cross-tenant request: tenant-c sees nothing on a channel
        // that tenant-a has rows for.
        let (_, body) = recent_response(&store, "sms", "tenant-c", 20).await;
        let parsed: Value = serde_json::from_str(&body).unwrap();
        assert!(parsed["items"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn recent_orders_newest_first_and_respects_limit() {
        let store = Arc::new(SessionStore::open(":memory:").await.unwrap());
        store.migrate().await.unwrap();
        let sink = AuditSink::new_with_tenant(Arc::clone(&store), "tenant-a".to_string());
        for i in 0..5 {
            sink.write(
                None,
                AuditKind::ApprovalReplied,
                json!({"action_id": format!("act-{i}"), "channel": "sms"}),
                None,
            )
            .await
            .unwrap();
        }

        let (_, body) = recent_response(&store, "sms", "tenant-a", 3).await;
        let parsed: Value = serde_json::from_str(&body).unwrap();
        let items = parsed["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        // Newest first → most-recently-written action_id leads.
        assert_eq!(items[0]["action_id"], "act-4");
        assert_eq!(items[1]["action_id"], "act-3");
        assert_eq!(items[2]["action_id"], "act-2");
    }
}
