//! `GET /routes` — the route-catalog response. Lifted out of
//! `routes_meta.rs` so that file stays under the 500-LOC ceiling
//! as the catalog grows. Pure data + a serde_json render — no
//! IO, no store access.

/// Build the `GET /routes[?method=GET|POST]` response: every
/// dispatched route + a one-line description, returned as a
/// JSON array of `{method, path, description}` rows. With
/// `method=GET|POST` the result is filtered to that verb.
pub(crate) fn routes_response(method_filter: Option<&str>) -> (&'static str, String) {
    let rows = [
        ("GET", "/health", "liveness probe"),
        ("GET", "/version", "build version"),
        ("GET", "/sessions", "list sessions (?limit&since-minutes&older-than-minutes&status&provider&model&title-contains&spent-min&spent-max&format=markdown)"),
        ("GET", "/sessions/<id>", "one session header + counts (?format=markdown for a paste-able header doc)"),
        ("GET", "/sessions/count", "count of sessions (?status=open|ended|all&since-minutes=N&older-than-minutes=N&provider=…&model=…)"),
        ("GET", "/sessions/<id>/messages", "messages (?limit&tail&role&since-minutes&seq-min&seq-max&content-contains)"),
        ("GET", "/sessions/<id>/messages/count", "count of messages on the session (?role=…&since-minutes=N&seq-min=N&seq-max=N&content-contains=…&format=markdown)"),
        ("GET", "/sessions/<id>/export", "session messages as JSONL or markdown (?format=jsonl|markdown&include-audit=true)"),
        ("GET", "/sessions/<id>/messages/last", "the last message on the session (?format=markdown for a paste-able block)"),
        ("GET", "/sessions/<id>/messages/first", "the first message on the session (typically the kickoff prompt; ?format=markdown for a paste-able block)"),
        ("GET", "/sessions/<id>/spend", "per-session spend ledger (?format=markdown)"),
        ("GET", "/sessions/<id>/spend/total", "single-number total for one session (?format=markdown)"),
        ("GET", "/sessions/<id>/audit", "per-session audit chain (alias for /audit/sessions/<id>; ?kind&since-minutes&format=markdown)"),
        ("GET", "/sessions/<id>/audit/count", "count of audit rows for one session (?kind&since-minutes=N)"),
        ("GET", "/sessions/<id>/audit/verify", "verify a session's chain integrity (alias for /audit/sessions/<id>/verify; ?include-breaks=true&format=markdown)"),
        ("GET", "/sessions/<id>/stats", "per-session stats: role histogram + timing envelope + inter-message gap stats (?format=markdown)"),
        ("GET", "/skills", "active skills (?include-superseded&name-contains&lock&body-contains&limit&format=markdown)"),
        ("GET", "/skills/count", "count of active skills (?include-superseded=true)"),
        ("GET", "/skills/<name>", "one skill view (full body; ?format=markdown reconstructs the SKILL.md doc)"),
        ("GET", "/skills/<name>/versions", "supersede chain for one skill name (?format=markdown)"),
        ("POST", "/skills", "install a SKILL.md document"),
        ("POST", "/memory", "append a memory entry ({file, body})"),
        ("GET", "/memory", "memory entries (?file&since-minutes&lock&body-contains&limit&format=markdown)"),
        ("GET", "/memory/count", "count of active memory entries (?file=…&since-minutes=N)"),
        ("GET", "/memory/<id>", "one memory entry (full body; ?format=markdown for a paste-able doc)"),
        ("GET", "/memory/<id>/versions", "supersede chain for a memory entry (?format=markdown)"),
        ("GET", "/audit", "audit chain summary (?format=markdown)"),
        ("GET", "/audit/breaks", "actual ChainBreak rows (seq, kind, reason) — empty if chain is clean (?format=markdown)"),
        ("GET", "/audit/count", "count of audit rows (?kind=…&since-minutes=N&session=<uuid>)"),
        ("GET", "/audit/recent", "most recent N audit rows across all sessions (?limit=50&kind=turn_start|...&since-minutes=M&session=<uuid>&format=markdown)"),
        ("GET", "/audit/sessions/<id>", "per-session audit chain (?kind&since-minutes&format=markdown)"),
        ("GET", "/audit/sessions/<id>/verify", "verify a session's chain integrity (?include-breaks=true&format=markdown)"),
        ("GET", "/costs", "cost ceilings (?format=markdown)"),
        ("GET", "/costs/total", "actual spend roll-up across all tenants today/this_month/all_time (?format=markdown)"),
        ("GET", "/costs/by-model", "spend grouped by (provider, model) (?format=markdown)"),
        ("GET", "/costs/by-session", "spend grouped by session_id (?format=markdown)"),
        ("GET", "/providers", "workspace provider manifests (?family=<f>&format=markdown)"),
        ("GET", "/providers/count", "count of provider manifests (?family=<f>&format=markdown)"),
        ("GET", "/providers/<id>", "one provider manifest by basename (?format=markdown for a paste-able doc)"),
        ("GET", "/providers/<id>/models", "just the [[models]] array for one provider (?format=markdown)"),
        ("GET", "/tools", "registered in-core tools (?name-contains=<sub>&format=markdown)"),
        ("GET", "/tools/count", "count of registered in-core tools (?name-contains=<sub>&format=markdown)"),
        ("GET", "/tools/<name>", "one tool by name (?format=markdown for a paste-able doc)"),
        ("GET", "/search", "FTS5 search (?q=…&limit=N&session=<id>&role=user|assistant|tool|system&format=markdown)"),
        ("GET", "/search/count", "count of FTS5 matches (?q=…&session=<id>&role=…&format=markdown; capped at 10000)"),
        ("GET", "/workspace/info", "first workspace row (id, name, root_path, created_at; ?format=markdown)"),
        ("GET", "/workspace/artifacts", "recorded marketplace pushes (?limit=N&since-minutes=M&format=markdown)"),
        ("GET", "/forge/runs", "recorded forge runs (?limit&since-minutes&status&score-min&score-max&skill&has-candidate&iterations-min&iterations-max&format=markdown)"),
        ("GET", "/forge/runs/<id>", "one forge_run row (?format=markdown for a paste-able doc)"),
        ("GET", "/forge/runs/count", "count of forge runs (?status=running|...&since-minutes=N)"),
        ("GET", "/cron", "scheduled cron jobs (workspace cron/jobs/*.json; ?format=markdown for a 6-column table)"),
        ("GET", "/cron/<name>", "one cron job by name (?format=markdown for a paste-able doc)"),
        ("POST", "/cron", "create a new cron job ({name, schedule, prompt, platform, chat})"),
        ("POST", "/cron/<name>/delete", "remove the on-disk job file"),
        ("POST", "/workspace/init", "register a workspace row ({name, root_path})"),
        ("POST", "/judge/<record_id>/override", "override a judge verdict ({to})"),
        ("GET", "/doctor", "JSON snapshot of `argo doctor`"),
        ("GET", "/health/full", "deep health (chain + skill + memory)"),
        ("GET", "/routes", "this listing (?method=GET|POST&format=markdown)"),
        ("GET", "/metrics", "Prometheus-style text exposition"),
        ("POST", "/metrics/reset", "zero out the scrape counter; returns {previous, current}"),
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
        ("GET", "/sessions/<id>/skills", "skills attached to the session (?format=markdown)"),
        ("GET", "/sessions/<id>/judge", "judge_record rows for the session (?format=markdown)"),
        ("POST", "/sessions/<id>/skills/attach", "attach a skill ({skill: name})"),
        ("POST", "/sessions/<id>/skills/detach", "detach a skill ({skill: name})"),
        ("POST", "/sessions/<id>/skills/clear", "detach every attached skill"),
        ("POST", "/sessions/<id>/delete", "delete a session + dependent rows"),
        ("POST", "/sessions/<id>/title", "rename a session ({title}, empty clears)"),
        ("POST", "/sessions/<id>/end", "mark a session ended (stamps ended_at)"),
        ("POST", "/sessions/<id>/reopen", "clear ended_at — inverse of /end"),
        ("POST", "/sessions/<id>/clone", "duplicate a session into a fresh row, copying every message; returns {cloned_from, session_id, title, messages_copied}"),
        ("POST", "/sessions/<id>/branch", "fork a session at a specific message (?through-seq=N copies seq 1..=N; defaults to all); returns {branched_from, session_id, title, through_seq, messages_copied}"),
        ("POST", "/sessions/end-all", "mark open sessions ended (?older-than-minutes=N for stale-only)"),
        ("POST", "/forge/optimize", "kick off `argo forge optimize` (returns 202)"),
        ("POST", "/forge/optimize/stream", "chunked-NDJSON forge optimize event stream"),
        ("POST", "/forge/bench", "kick off `argo forge bench` (returns 202)"),
        ("POST", "/forge/runs/<id>/abort", "abort an in-flight forge run"),
        ("POST", "/forge/runs/abort-all", "abort every in-flight forge run"),
        ("POST", "/forge/runs/sweep", "flip stalled running rows older than ?older-than-minutes=60 to failed (gateway-crash recovery)"),
        ("POST", "/forge/runs/<id>/promote", "promote that run's candidate to active"),
        ("POST", "/skills/<name>/promote", "promote a skill version to active"),
        ("POST", "/skills/<name>/lock", "lock a skill (user-protected by default)"),
        ("POST", "/skills/<name>/unlock", "clear a skill lock"),
        ("POST", "/skills/<name>/refine", "supersede with hint-appended body ({hint, version?})"),
        ("POST", "/memory/<id>/lock", "lock a memory entry"),
        ("POST", "/memory/<id>/unlock", "clear a memory-entry lock"),
        ("POST", "/memory/<id>/forget", "revert (supersede) a memory entry"),
        ("POST", "/memory/<id>/edit", "supersede with a new body ({body})"),
    ];
    let body: Vec<serde_json::Value> = rows
        .iter()
        .filter(|(method, _, _)| match method_filter {
            Some(m) => m.eq_ignore_ascii_case(method),
            None => true,
        })
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

/// Markdown table render of the route catalog. Returned when
/// `GET /routes?format=markdown` is requested. Same `method`
/// filter semantics as the JSON variant. Pipes in descriptions
/// are escaped so unusual characters don't break the table.
pub(crate) fn routes_markdown(method_filter: Option<&str>) -> (&'static str, String) {
    let (_status, json_body) = routes_response(method_filter);
    let parsed: serde_json::Value = match serde_json::from_str(&json_body) {
        Ok(v) => v,
        Err(_) => return ("500 Internal Server Error", String::from("# error")),
    };
    let arr = parsed.as_array().cloned().unwrap_or_default();
    let mut out = format!("# Gateway routes ({} endpoints)\n\n", arr.len());
    if arr.is_empty() {
        return ("200 OK", out);
    }
    out.push_str("| method | path | description |\n");
    out.push_str("| :---: | --- | --- |\n");
    for row in &arr {
        let m = row["method"].as_str().unwrap_or("?");
        let p = row["path"].as_str().unwrap_or("?");
        let d = row["description"].as_str().unwrap_or("").replace('|', "\\|");
        out.push_str(&format!("| `{m}` | `{p}` | {d} |\n"));
    }
    ("200 OK", out)
}
