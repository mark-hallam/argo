//! `argo gateway start` boot-time route summary banner — printed once
//! when the gateway begins listening so an operator can see at a
//! glance what endpoints the process serves. Extracted from `mod.rs`
//! to keep that file under the 500-LOC ceiling. The behaviour is
//! identical to the inline eprintln chain it replaced.

pub(super) fn print_endpoint_summary() {
    eprintln!("  GET  /health   → {{\"status\":\"ok\"}}");
    eprintln!("  GET  /version  → {{\"version\":\"<crate>\"}}");
    eprintln!("  GET  /sessions[?since-minutes=N] → recent sessions (JSON array; carries spent_usd_micros)");
    eprintln!(
        "  GET  /sessions/<uuid> → one session header + message_count + tool_call_count + spend"
    );
    eprintln!("  GET  /sessions/<uuid>/messages[?limit=N&role=user&since-minutes=M] → messages on the session");
    eprintln!("  GET  /sessions/<uuid>/messages/last → just the last message");
    eprintln!("  GET  /sessions/<uuid>/spend → per-session spend ledger rows + total");
    eprintln!("  GET  /sessions/<uuid>/spend/total → single-number total for one session");
    eprintln!("  POST /sessions → create a session row ({{model, provider, title?}})");
    eprintln!("  POST /sessions/<uuid>/messages → append a message ({{role, content}})");
    eprintln!("  POST /sessions/<uuid>/runs → run one agent turn ({{user?, stream:false, max_tool_rounds?}})");
    eprintln!("  POST /sessions/<uuid>/runs/stream → chunked-NDJSON stream of audit events as the run progresses");
    eprintln!("  POST /sessions/<uuid>/steer → inject a system-role steer ({{text}})");
    eprintln!("  POST /sessions/<uuid>/replay → replay user prompts in a fresh session ({{model?, provider?}})");
    eprintln!("  GET  /sessions/<uuid>/skills → skills attached to the session");
    eprintln!("  GET  /sessions/<uuid>/judge  → judge_record rows for the session");
    eprintln!("  POST /sessions/<uuid>/skills/{{attach,detach}} → manage per-session skill set");
    eprintln!("  POST /sessions/<uuid>/delete → delete a session + dependent rows");
    eprintln!("  POST /sessions/<uuid>/title  → rename ({{title}}; empty clears)");
    eprintln!("  POST /sessions/<uuid>/end    → mark a session ended (stamps ended_at)");
    eprintln!("  POST /sessions/<uuid>/replay/stream → chunked-NDJSON stream of replay events");
    eprintln!("  POST /channels/email/webhook → Postmark inbound approval webhook");
    eprintln!("  GET  /skills   → active skills (?include-superseded=true for full history)");
    eprintln!("  GET  /skills/<name> → one skill view (full body + frontmatter)");
    eprintln!("  GET  /skills/<name>/versions → supersede chain for that name");
    eprintln!("  POST /skills   → install a SKILL.md document ({{raw}})");
    eprintln!("  POST /memory   → append a memory entry ({{file: memory|user|soul|agents, body}})");
    eprintln!("  GET  /memory[?file=memory|user|soul|agents] → active memory entries");
    eprintln!("  GET  /memory/<id> → one memory entry (full body)");
    eprintln!("  GET  /memory/<id>/versions → supersede chain for that entry");
    eprintln!("  GET  /audit    → audit chain summary (ok / rows / breaks / kinds)");
    eprintln!("  GET  /providers → workspace provider manifests + their model lists");
    eprintln!("  GET  /providers/<id> → one provider manifest by basename");
    eprintln!("  GET  /providers/<id>/models → just the [[models]] array");
    eprintln!("  GET  /tools    → registered in-core tools + JSON-Schema parameters");
    eprintln!("  GET  /tools/<name> → one tool's name + description + parameters");
    eprintln!("  GET  /search?q=…[&limit=N&session=<id>] → FTS5 hits across the message corpus");
    eprintln!("  GET  /workspace/info → first workspace row (id, name, root_path, created_at)");
    eprintln!("  GET  /workspace/artifacts[?limit=N] → recorded marketplace pushes (oci_ref + digest + pushed_at)");
    eprintln!("  GET  /forge/runs[?limit=N&since-minutes=M] → recorded forge runs");
    eprintln!("  GET  /forge/runs/<id>      → one forge_run row");
    eprintln!("  GET  /cron                 → scheduled cron jobs (workspace cron/jobs/)");
    eprintln!("  GET  /cron/<name>          → one cron job by name");
    eprintln!(
        "  POST /cron                 → create a job ({{name, schedule, prompt, platform, chat}})"
    );
    eprintln!("  POST /workspace/init       → register a workspace row ({{name, root_path}})");
    eprintln!("  POST /judge/<record_id>/override → override a judge verdict ({{to}})");
    eprintln!("  POST /cron/<name>/delete   → remove the on-disk job file");
    eprintln!("  POST /forge/optimize → kick off `argo forge optimize` ({{skill, examples, iterations?, budget_usd?}})");
    eprintln!("  POST /forge/optimize/stream → chunked-NDJSON live forge progress");
    eprintln!("  POST /forge/bench    → kick off `argo forge bench` ({{skill, examples, provider?, model?}})");
    eprintln!("  POST /forge/runs/<id>/abort → flip a run's status to 'aborted'");
    eprintln!("  POST /forge/runs/<id>/promote → promote that run's candidate to active");
    eprintln!("  POST /skills/<name>/promote → promote a version to active ({{version}})");
    eprintln!("  POST /skills/<name>/{{lock,unlock,refine}} → lock / unlock / append hint refine");
    eprintln!("  POST /memory/<id>/{{lock,unlock,forget,edit}} → admin ops on a memory entry");
    eprintln!("  GET  /metrics        → Prometheus-style text exposition (text/plain)");
    eprintln!("  GET  /routes  → discovery: every route + a one-line description");
    eprintln!("  GET  /doctor       → JSON snapshot of `argo doctor` (cwd, db, cron, creds, …)");
    eprintln!("  GET  /health/full  → deep health (audit chain + skill + memory provenance)");
    eprintln!("  GET  /audit/sessions/<uuid>[?kind=<kind>] → per-session audit chain (JSON array)");
    eprintln!("  GET  /audit/sessions/<uuid>/verify → verify a session's chain integrity");
    eprintln!("  GET  /costs    → cost ceilings (per_session/day/month + notify_at_pct)");
    eprintln!("  GET  /db/stats → per-table row counts (sessions, messages, audit, …)");
    eprintln!("  POST /db/vacuum → run VACUUM on the SQLite store");
    eprintln!("  POST /db/prune  → prune sessions older than N days ({{older_than_days?}})");
    eprintln!("  GET  /costs/by-model    → top spend buckets grouped by (provider, model)");
    eprintln!("  GET  /costs/by-session  → top spend buckets grouped by session_id");
    eprintln!("press Ctrl-C or `argo gateway stop` to terminate");
}
