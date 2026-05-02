//! `/forge/runs/<id>[/...]` GET dispatch lifted out of `routes.rs`
//! once the audit drill-down (#634) pushed the main dispatcher
//! over the 500-LOC budget. Returns `Some(resp)` on a match,
//! `None` otherwise so callers fall through.
use super::routes_query::parse_query_kv;
use argo_store::SessionStore;
use std::sync::Arc;

pub(super) async fn dispatch_forge_run_get(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> Option<(&'static str, String)> {
    let rest = req.strip_prefix("GET /forge/runs/")?;
    // /forge/runs/<id>/{payload,audit} — sub-paths take priority
    // over the bare /forge/runs/<id> show.
    if let Some((id, tail)) = rest.split_once('/') {
        if tail.starts_with("payload ") || tail.starts_with("payload?") {
            let format = parse_query_kv(tail, "format");
            return Some(
                super::routes_forge::forge_run_payload_response(
                    id,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
        if tail.starts_with("audit ") || tail.starts_with("audit?") {
            let format = parse_query_kv(tail, "format");
            return Some(
                super::routes_forge_rollups::forge_run_audit_response(
                    id,
                    store,
                    format.as_deref(),
                )
                .await,
            );
        }
    }
    if let Some((id_with_query, _)) = rest.split_once(' ') {
        let (id, query) = match id_with_query.split_once('?') {
            Some((i, q)) => (i, Some(q)),
            None => (id_with_query, None),
        };
        let format = query
            .and_then(|q| q.split('&').find_map(|kv| kv.strip_prefix("format=")))
            .map(str::to_string);
        return Some(
            super::routes_forge::forge_run_show_response_with_format(
                id,
                store,
                format.as_deref(),
            )
            .await,
        );
    }
    None
}
