//! `GET /sessions/<id>/...` sub-dispatcher. Lifted out of
//! `routes.rs` so the main dispatcher stays under the 500-LOC
//! budget — and so we have one place to add new per-session
//! GET endpoints (export, etc).
//!
//! The helper returns `Some(response)` when the request matches
//! one of the known per-session GETs, `None` otherwise — letting
//! `routes.rs` fall through to the 404 path or other dispatchers.
//!
//! Sibling: `routes_session_get_export.rs` owns the
//! `GET /sessions/<id>/export` body + render helpers.

use super::routes_audit::{
    audit_count_response, audit_session_response, audit_session_verify_response,
};
use super::routes_msgs::{
    count_messages_response, first_message_response, last_message_response,
    list_messages_response, message_at_seq_response,
};
use super::routes_post::session_judge_response;
use super::routes_query::{parse_limit, parse_query_kv, parse_role};
use super::routes_session::{session_spend_response, session_spend_total_response};
use super::routes_session_get_export::export_response;
use super::routes_session_skills::session_skills_list_response;
use super::routes_session_stats::{session_stats_response, session_turns_response};
use argo_store::SessionStore;
use std::sync::Arc;

/// Dispatch every `GET /sessions/<id>/...` (and bare
/// `GET /sessions/<id>`) request. Caller has already screened the
/// `GET /sessions/` prefix.
pub(super) async fn dispatch_session_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("GET /sessions/")?;
    if let Some((sid, tail)) = rest.split_once('/') {
        if tail.starts_with("spend/total ") || tail.starts_with("spend/total?") {
            let format = parse_query_kv(tail, "format");
            return Some(
                session_spend_total_response(sid, store, format.as_deref()).await,
            );
        }
        if tail.starts_with("spend ") || tail.starts_with("spend?") {
            let format = parse_query_kv(tail, "format");
            return Some(session_spend_response(sid, store, format.as_deref()).await);
        }
        if tail.starts_with("messages/last ") || tail.starts_with("messages/last?") {
            let format = parse_query_kv(tail, "format");
            return Some(last_message_response(sid, store, format.as_deref()).await);
        }
        if tail.starts_with("messages/first ") || tail.starts_with("messages/first?") {
            let format = parse_query_kv(tail, "format");
            return Some(first_message_response(sid, store, format.as_deref()).await);
        }
        if tail.starts_with("messages/by-role ") || tail.starts_with("messages/by-role?") {
            let since = parse_query_kv(tail, "since-minutes")
                .and_then(|v| v.parse::<i64>().ok());
            let format = parse_query_kv(tail, "format");
            return Some(
                super::routes_messages_rollups::session_messages_by_role_response(
                    sid,
                    since,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
        if tail.starts_with("messages/count ") || tail.starts_with("messages/count?") {
            let role = parse_role(tail);
            let since = parse_query_kv(tail, "since-minutes")
                .and_then(|v| v.parse::<i64>().ok());
            let seq_min = parse_query_kv(tail, "seq-min")
                .and_then(|v| v.parse::<i64>().ok());
            let seq_max = parse_query_kv(tail, "seq-max")
                .and_then(|v| v.parse::<i64>().ok());
            let content_contains =
                parse_query_kv(tail, "content-contains").map(|s| s.replace('+', " "));
            let format = parse_query_kv(tail, "format");
            return Some(
                count_messages_response(
                    sid,
                    store,
                    role,
                    since,
                    seq_min,
                    seq_max,
                    content_contains,
                    format.as_deref(),
                )
                .await,
            );
        }
        // /messages/<numeric_seq> — fetch one row by seq. Has to
        // come after the word-keyed branches (last / first /
        // count) and before the bare /messages list so the
        // numeric path segment is correctly disambiguated.
        if let Some(after) = tail.strip_prefix("messages/") {
            let seg = after
                .split([' ', '?'])
                .next()
                .unwrap_or("");
            if !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()) {
                if let Ok(seq) = seg.parse::<i64>() {
                    let format = parse_query_kv(tail, "format");
                    return Some(
                        message_at_seq_response(sid, seq, store, format.as_deref())
                            .await,
                    );
                }
            }
        }
        if tail.starts_with("messages ") || tail.starts_with("messages?") {
            let limit = parse_limit(tail);
            let tail_n = parse_query_kv(tail, "tail").and_then(|v| v.parse::<usize>().ok());
            let role = parse_role(tail);
            let since = parse_query_kv(tail, "since-minutes")
                .and_then(|v| v.parse::<i64>().ok());
            let seq_min = parse_query_kv(tail, "seq-min")
                .and_then(|v| v.parse::<i64>().ok());
            let seq_max = parse_query_kv(tail, "seq-max")
                .and_then(|v| v.parse::<i64>().ok());
            let content_contains =
                parse_query_kv(tail, "content-contains").map(|s| s.replace('+', " "));
            let format = parse_query_kv(tail, "format");
            return Some(
                list_messages_response(
                    sid,
                    store,
                    limit,
                    tail_n,
                    role,
                    since,
                    seq_min,
                    seq_max,
                    content_contains,
                    format.as_deref(),
                )
                .await,
            );
        }
        if tail.starts_with("skills ") || tail.starts_with("skills?") {
            let format = parse_query_kv(tail, "format");
            return Some(
                session_skills_list_response(sid, store, format.as_deref()).await,
            );
        }
        // #666 — list active human-approval grants for this
        // session. Operator visibility into "what's currently
        // allowed through the escalation gate".
        if tail.starts_with("grants ") || tail.starts_with("grants?") {
            return Some(
                super::routes_session_approval::session_grants_get_response(sid, store).await,
            );
        }
        if tail.starts_with("judge ") || tail.starts_with("judge?") {
            let format = parse_query_kv(tail, "format");
            return Some(session_judge_response(sid, store, format.as_deref()).await);
        }
        if tail.starts_with("export ") || tail.starts_with("export?") {
            return Some(export_response(sid, tail, store).await);
        }
        if tail.starts_with("stats ") || tail.starts_with("stats?") {
            let format = parse_query_kv(tail, "format");
            return Some(session_stats_response(sid, store, format.as_deref()).await);
        }
        if tail.starts_with("turns ") || tail.starts_with("turns?") {
            let format = parse_query_kv(tail, "format");
            return Some(session_turns_response(sid, store, format.as_deref()).await);
        }
        // /sessions/<id>/audit/{verify,count} are aliases for the
        // /audit/sessions/<id>/{verify,count} surface so callers
        // that already know the session id don't have to swap
        // path prefixes.
        if tail.starts_with("audit/verify ") || tail.starts_with("audit/verify?") {
            let include_breaks = parse_query_kv(tail, "include-breaks")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false);
            let format = parse_query_kv(tail, "format");
            return Some(
                audit_session_verify_response(
                    sid,
                    include_breaks,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
        if tail.starts_with("audit/count ") || tail.starts_with("audit/count?") {
            let kind = parse_query_kv(tail, "kind");
            let since = parse_query_kv(tail, "since-minutes")
                .and_then(|v| v.parse::<i64>().ok());
            return Some(
                audit_count_response(kind.as_deref(), since, Some(sid), store).await,
            );
        }
        if tail.starts_with("audit/by-kind ") || tail.starts_with("audit/by-kind?") {
            let since = parse_query_kv(tail, "since-minutes")
                .and_then(|v| v.parse::<i64>().ok());
            let format = parse_query_kv(tail, "format");
            return Some(
                super::routes_session_audit_rollups::session_audit_by_kind_response(
                    sid,
                    since,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
        if tail.starts_with("audit/by-tool ") || tail.starts_with("audit/by-tool?") {
            let since = parse_query_kv(tail, "since-minutes")
                .and_then(|v| v.parse::<i64>().ok());
            let format = parse_query_kv(tail, "format");
            return Some(
                super::routes_session_audit_rollups::session_audit_by_tool_response(
                    sid,
                    since,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
        if tail.starts_with("audit/tool-errors ")
            || tail.starts_with("audit/tool-errors?")
        {
            let since = parse_query_kv(tail, "since-minutes")
                .and_then(|v| v.parse::<i64>().ok());
            let format = parse_query_kv(tail, "format");
            return Some(
                super::routes_session_audit_rollups::session_audit_tool_errors_response(
                    sid,
                    since,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
        if tail.starts_with("audit ") || tail.starts_with("audit?") {
            let kind = parse_query_kv(tail, "kind");
            let since = parse_query_kv(tail, "since-minutes")
                .and_then(|v| v.parse::<i64>().ok());
            let format = parse_query_kv(tail, "format");
            return Some(
                audit_session_response(
                    sid,
                    store,
                    kind.as_deref(),
                    since,
                    format.as_deref(),
                )
                .await,
            );
        }
    }
    // Bare `GET /sessions/<uuid>[?format=markdown]` — strip
    // trailing " HTTP/..." → split off the query string.
    if let Some((sid_with_query, _)) = rest.split_once(' ') {
        let (sid, query) = match sid_with_query.split_once('?') {
            Some((s, q)) => (s, Some(q)),
            None => (sid_with_query, None),
        };
        let format = query
            .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
            .map(str::to_string);
        return Some(
            super::routes_session::session_show_response_with_format(
                sid,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    None
}
