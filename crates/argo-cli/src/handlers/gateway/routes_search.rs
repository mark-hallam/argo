//! `GET /search` + `GET /search/count` — FTS5 search across the
//! message corpus, lifted out of `routes_meta.rs` so that file
//! stays under the 500-LOC budget. Both handlers honour the
//! same `q` / `session` / `role` / `format` filter family;
//! `/search` also accepts `limit`, `/search/count` does not.
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

/// `GET /search?q=<query>[&limit=N][&session=<uuid>][&role=<r>]`
/// — FTS5 search across the message corpus. Mirrors
/// `argo session search <query>` so external clients (web UI,
/// scripts) can reuse the same query syntax.
///
/// Each row is `{session_id, seq, role, snippet}`. With
/// `?role=user|assistant|tool|system` the result is filtered
/// to that role. Returns 503 when the store is unavailable,
/// 400 when `q` is missing, `session` is not a valid UUID, or
/// `role` is not one of the four canonical labels.
#[allow(clippy::too_many_arguments)]
pub(super) async fn search_response(
    query: Option<&str>,
    limit: u32,
    session: Option<&str>,
    role: Option<&str>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let Some(q) = query else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing q parameter\"}".to_string(),
        );
    };
    if q.is_empty() {
        return (
            "400 Bad Request",
            "{\"error\":\"empty q parameter\"}".to_string(),
        );
    }
    if let Some(want) = role {
        if !matches!(want, "user" | "assistant" | "tool" | "system") {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown role '{want}' — use user|assistant|tool|system\"}}"
                ),
            );
        }
    }
    let result = match session {
        None => store.search_messages(q, limit).await,
        Some(s) => match uuid::Uuid::parse_str(s) {
            Ok(u) => store.search_messages_in_session(SessionId(u), q, limit).await,
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"invalid session id: {e}\"}}"),
                );
            }
        },
    };
    match result {
        Ok(hits) => {
            let filtered: Vec<argo_store::MessageSearchHit> = hits
                .into_iter()
                .filter(|h| match role {
                    Some(want) => h.role == want,
                    None => true,
                })
                .collect();
            if matches!(format, Some("markdown")) {
                let n = filtered.len();
                let mut out = format!(
                    "# Search results for `{q}` ({n} match{})\n\n",
                    if n == 1 { "" } else { "es" }
                );
                if !filtered.is_empty() {
                    out.push_str("| session_id | seq | role | snippet |\n");
                    out.push_str("| --- | ---: | --- | --- |\n");
                    for h in &filtered {
                        let snippet = h.snippet.replace('|', "\\|");
                        out.push_str(&format!(
                            "| `{}` | {} | `{}` | {snippet} |\n",
                            h.session_id.0, h.seq, h.role
                        ));
                    }
                }
                return ("200 OK", out);
            }
            let rows: Vec<serde_json::Value> = filtered
                .into_iter()
                .map(|h| {
                    serde_json::json!({
                        "session_id": h.session_id.0.to_string(),
                        "seq": h.seq,
                        "role": h.role,
                        "snippet": h.snippet,
                    })
                })
                .collect();
            (
                "200 OK",
                serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"search: {e}\"}}"),
        ),
    }
}

/// `GET /search/count?q=…[&session=<uuid>&role=<r>&format=markdown]`
/// — count of FTS5 matches for `q` after the same role +
/// session filters that `search_response` honours. Useful for
/// paginated UIs that want a "found N matches" header without
/// fetching the row bodies. Caps the underlying fetch at
/// 10,000; anything above that returns `{count: 10000,
/// truncated: true}`.
pub(super) async fn search_count_response(
    query: Option<&str>,
    session: Option<&str>,
    role: Option<&str>,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return super::store_unavailable();
    };
    let Some(q) = query else {
        return (
            "400 Bad Request",
            "{\"error\":\"missing q parameter\"}".to_string(),
        );
    };
    if q.is_empty() {
        return (
            "400 Bad Request",
            "{\"error\":\"empty q parameter\"}".to_string(),
        );
    }
    if let Some(want) = role {
        if !matches!(want, "user" | "assistant" | "tool" | "system") {
            return (
                "400 Bad Request",
                format!(
                    "{{\"error\":\"unknown role '{want}' — use user|assistant|tool|system\"}}"
                ),
            );
        }
    }
    const HARD_CAP: u32 = 10_000;
    let result = match session {
        None => store.search_messages(q, HARD_CAP).await,
        Some(s) => match uuid::Uuid::parse_str(s) {
            Ok(u) => store.search_messages_in_session(SessionId(u), q, HARD_CAP).await,
            Err(e) => {
                return (
                    "400 Bad Request",
                    format!("{{\"error\":\"invalid session id: {e}\"}}"),
                );
            }
        },
    };
    match result {
        Ok(hits) => {
            let count = hits
                .into_iter()
                .filter(|h| match role {
                    Some(want) => h.role == want,
                    None => true,
                })
                .count();
            let truncated = count as u32 >= HARD_CAP;
            if matches!(format, Some("markdown")) {
                let mut out = format!(
                    "# Search match count for `{q}` ({count}{})\n\n",
                    if truncated { "+" } else { "" }
                );
                if let Some(s) = session {
                    out.push_str(&format!("- **session**: `{s}`\n"));
                }
                if let Some(r) = role {
                    out.push_str(&format!("- **role**: `{r}`\n"));
                }
                if truncated {
                    out.push_str("- **truncated**: hard cap of 10000 reached\n");
                }
                return ("200 OK", out);
            }
            (
                "200 OK",
                serde_json::to_string(&serde_json::json!({
                    "count": count,
                    "truncated": truncated,
                }))
                .unwrap_or_else(|_| "{}".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"search: {e}\"}}"),
        ),
    }
}
