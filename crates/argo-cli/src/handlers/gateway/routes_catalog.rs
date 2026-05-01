//! `GET /routes` — the route-catalog response. Lifted out of
//! `routes_meta.rs` so that file stays under the 500-LOC ceiling
//! as the catalog grows. Pure data + a serde_json render — no
//! IO, no store access.

/// Build the `GET /routes` response: every dispatched route +
/// a one-line description, returned as a JSON array of
/// `{method, path, description}` rows.
pub(super) fn routes_response() -> (&'static str, String) {
    let rows = [
        ("GET", "/health", "liveness probe"),
        ("GET", "/version", "build version"),
        ("GET", "/sessions", "list sessions (?since-minutes=N)"),
        ("GET", "/sessions/<id>", "one session header + counts"),
        ("GET", "/sessions/<id>/messages", "messages (?limit=N&role=<r>)"),
        ("GET", "/sessions/<id>/messages/last", "the last message on the session"),
        ("GET", "/sessions/<id>/spend", "per-session spend ledger"),
        ("GET", "/skills", "active skills"),
        ("GET", "/skills/<name>", "one skill view (full body)"),
        ("GET", "/skills/<name>/versions", "supersede chain for one skill name"),
        ("POST", "/skills", "install a SKILL.md document"),
        ("POST", "/memory", "append a memory entry ({file, body})"),
        ("GET", "/memory", "memory entries"),
        ("GET", "/memory/<id>", "one memory entry (full body)"),
        ("GET", "/audit", "audit chain summary"),
        ("GET", "/audit/sessions/<id>", "per-session audit chain"),
        ("GET", "/audit/sessions/<id>/verify", "verify a session's chain integrity"),
        ("GET", "/costs", "cost ceilings"),
        ("GET", "/costs/by-model", "spend grouped by (provider, model)"),
        ("GET", "/costs/by-session", "spend grouped by session_id"),
        ("GET", "/providers", "workspace provider manifests"),
        ("GET", "/tools", "registered in-core tools"),
        ("GET", "/search", "FTS5 search (?q=…&limit=N&session=<id>)"),
        ("GET", "/workspace/info", "first workspace row (id, name, root_path, created_at)"),
        ("GET", "/workspace/artifacts", "recorded marketplace pushes"),
        ("GET", "/forge/runs", "recorded forge optimization runs (?limit=N&since-minutes=M)"),
        ("GET", "/forge/runs/<id>", "one forge_run row"),
        ("GET", "/cron", "scheduled cron jobs (workspace cron/jobs/*.json)"),
        ("GET", "/cron/<name>", "one cron job by name"),
        ("POST", "/cron", "create a new cron job ({name, schedule, prompt, platform, chat})"),
        ("POST", "/cron/<name>/delete", "remove the on-disk job file"),
        ("POST", "/workspace/init", "register a workspace row ({name, root_path})"),
        ("POST", "/judge/<record_id>/override", "override a judge verdict ({to})"),
        ("GET", "/doctor", "JSON snapshot of `argo doctor`"),
        ("GET", "/health/full", "deep health (chain + skill + memory)"),
        ("GET", "/routes", "this listing"),
        ("GET", "/metrics", "Prometheus-style text exposition"),
        ("GET", "/db/stats", "per-table row counts (sessions, messages, audit, …)"),
        ("POST", "/db/vacuum", "run VACUUM on the SQLite store"),
        ("POST", "/db/prune", "delete sessions older than N days ({older_than_days?})"),
        ("POST", "/sessions", "create a session row"),
        ("POST", "/sessions/<id>/messages", "append a message"),
        ("POST", "/sessions/<id>/runs", "run one agent turn"),
        ("POST", "/sessions/<id>/runs/stream", "chunked-NDJSON live event stream"),
        ("POST", "/sessions/<id>/steer", "inject a system-role steer"),
        ("POST", "/sessions/<id>/replay", "replay user prompts in a fresh session"),
        ("POST", "/sessions/<id>/replay/stream", "chunked-NDJSON replay event stream"),
        ("GET", "/sessions/<id>/skills", "skills attached to the session"),
        ("GET", "/sessions/<id>/judge", "judge_record rows for the session"),
        ("POST", "/sessions/<id>/skills/attach", "attach a skill ({skill: name})"),
        ("POST", "/sessions/<id>/skills/detach", "detach a skill ({skill: name})"),
        ("POST", "/sessions/<id>/delete", "delete a session + dependent rows"),
        ("POST", "/sessions/<id>/title", "rename a session ({title}, empty clears)"),
        ("POST", "/sessions/<id>/end", "mark a session ended (stamps ended_at)"),
        ("POST", "/skills", "install a SKILL.md document"),
        ("POST", "/memory", "append a memory entry"),
        ("POST", "/forge/optimize", "kick off `argo forge optimize` (returns 202)"),
        ("POST", "/forge/optimize/stream", "chunked-NDJSON forge optimize event stream"),
        ("POST", "/forge/bench", "kick off `argo forge bench` (returns 202)"),
        ("POST", "/forge/runs/<id>/abort", "abort an in-flight forge run"),
        ("POST", "/skills/<name>/promote", "promote a skill version to active"),
        ("POST", "/skills/<name>/lock", "lock a skill (user-protected by default)"),
        ("POST", "/skills/<name>/unlock", "clear a skill lock"),
        ("POST", "/memory/<id>/lock", "lock a memory entry"),
        ("POST", "/memory/<id>/unlock", "clear a memory-entry lock"),
        ("POST", "/memory/<id>/forget", "revert (supersede) a memory entry"),
        ("POST", "/memory/<id>/edit", "supersede with a new body ({body})"),
    ];
    let body: Vec<serde_json::Value> = rows
        .iter()
        .map(|(method, path, description)| {
            serde_json::json!({
                "method": method,
                "path": path,
                "description": description,
            })
        })
        .collect();
    (
        "200 OK",
        serde_json::to_string(&body).unwrap_or_else(|_| "[]".to_string()),
    )
}
