//! GET /sessions/<uuid>/judge response builder split out of
//! `routes_post.rs` for the 500-LOC budget. (The parent file's
//! name is historical — this is a heavy query helper, not a
//! POST; the dispatcher in routes_session_get.rs is unchanged.)
//!
//! Public API stays stable: `routes_post.rs` re-exports
//! `session_judge_response` so existing call sites compile
//! unchanged.
use argo_store::SessionStore;
use argo_types::SessionId;
use std::sync::Arc;

use super::store_unavailable;

/// `GET /sessions/<id>/judge[?format=markdown]` — list
/// judge_record rows scoped to one session. Mirrors
/// `argo judge verdict <id>`. Returns 200 + a JSON array
/// (possibly empty) or markdown table; 400 on bad uuid.
pub(super) async fn session_judge_response(
    sid: &str,
    store: Option<&Arc<SessionStore>>,
    format: Option<&str>,
) -> (&'static str, String) {
    let Some(store) = store else {
        return store_unavailable();
    };
    let id = match uuid::Uuid::parse_str(sid) {
        Ok(u) => SessionId(u),
        Err(e) => {
            return (
                "400 Bad Request",
                format!("{{\"error\":\"invalid session id: {e}\"}}"),
            );
        }
    };
    match store.list_judge_records(id).await {
        Ok(rows) => {
            if matches!(format, Some("markdown")) {
                let n = rows.len();
                let mut out = format!(
                    "# Judge verdicts for session `{sid}` ({n} record{})\n\n",
                    if n == 1 { "" } else { "s" }
                );
                if rows.is_empty() {
                    out.push_str("_No judge records yet._\n");
                } else {
                    out.push_str("| subject | verdict | judge | reason |\n");
                    out.push_str("| --- | :---: | --- | --- |\n");
                    for r in &rows {
                        let reason = r
                            .reason
                            .as_deref()
                            .unwrap_or("—")
                            .replace('|', "\\|");
                        out.push_str(&format!(
                            "| `{}/{}` | `{}` | `{}/{}` | {reason} |\n",
                            r.subject_kind,
                            r.subject_ref,
                            r.verdict,
                            r.judge_provider,
                            r.judge_model,
                        ));
                    }
                }
                return ("200 OK", out);
            }
            (
                "200 OK",
                serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_string()),
            )
        }
        Err(e) => (
            "500 Internal Server Error",
            format!("{{\"error\":\"list_judge_records: {e}\"}}"),
        ),
    }
}
