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
        ("GET", "/", "landing JSON: name + version + entry-point hints (replaces the bare 404)"),
        ("GET", "/health", "combined health probe (liveness + timestamp)"),
        ("GET", "/health/live", "K8s-style liveness probe — always 200 if the process responds"),
        ("GET", "/health/ready", "K8s-style readiness probe — 200 when SQLite store responds, 503 otherwise"),
        ("GET", "/verticals", "list loaded vertical templates + load errors (?format=markdown; #650)"),
        ("GET", "/verticals/<id>", "show one vertical's manifest summary (?format=markdown; #650; 404 if not loaded)"),
        ("POST", "/verticals/<id>/sessions", "create a session bound to this vertical; body {model, provider, title?} (#651)"),
        ("POST", "/verticals/<id>/run", "one-shot composite (#670): create session + attach skills + run the named workflow against the new session; body {workflow, title?, provider?, model?}; returns {session_id, vertical_id, skills_attached, workflow:{…}}"),
        ("GET",  "/verticals/<id>/lint", "best-practice hygiene checks for one vertical (#672); returns {ok, id, findings:[{severity, code, message}]}; 422 when any finding has severity=error"),
        ("GET",  "/verticals/<id>/doctor", "composite health check (#674): validate + eval + lint in one report; 422 when any check fails"),
        ("GET", "/version", "build version"),
        ("GET", "/sessions", "list sessions incl. vertical_id per row (#690; ?limit&since-minutes&older-than-minutes&status&provider&model&title-contains&spent-min&spent-max&format=markdown)"),
        ("GET", "/sessions/<id>", "one session header + counts incl. vertical_id (#689; ?format=markdown for a paste-able header doc)"),
        ("GET", "/sessions/count", "count of sessions (?status=open|ended|all&since-minutes=N&older-than-minutes=N&provider=…&model=…)"),
        ("GET", "/sessions/by-day", "sessions created per calendar day (?days=N&format=markdown; default 30, cap 365)"),
        ("GET", "/sessions/by-provider", "histogram of sessions by provider (?since-minutes=N&format=markdown)"),
        ("GET", "/sessions/by-model", "histogram of sessions by model (?since-minutes=N&format=markdown)"),
        ("GET", "/sessions/<id>/messages", "messages (?limit&tail&role&since-minutes&seq-min&seq-max&content-contains&format=markdown)"),
        ("GET", "/sessions/<id>/messages/count", "count of messages on the session (?role=…&since-minutes=N&seq-min=N&seq-max=N&content-contains=…&format=markdown)"),
        ("GET", "/sessions/<id>/export", "session messages as JSONL or markdown (?format=jsonl|markdown&include-audit=true)"),
        ("GET", "/sessions/<id>/messages/last", "the last message on the session (?format=markdown for a paste-able block)"),
        ("GET", "/sessions/<id>/messages/first", "the first message on the session (typically the kickoff prompt; ?format=markdown for a paste-able block)"),
        ("GET", "/sessions/<id>/messages/<seq>", "one message by its sequence number (?format=markdown for a paste-able block)"),
        ("GET", "/sessions/<id>/spend", "per-session spend ledger (?format=markdown)"),
        ("GET", "/sessions/<id>/spend/total", "single-number total for one session (?format=markdown)"),
        ("GET", "/sessions/<id>/audit", "per-session audit chain (alias for /audit/sessions/<id>; ?kind&since-minutes&format=markdown)"),
        ("GET", "/sessions/<id>/audit/count", "count of audit rows for one session (?kind&since-minutes=N)"),
        ("GET", "/sessions/<id>/audit/verify", "verify a session's chain integrity (alias for /audit/sessions/<id>/verify; ?include-breaks=true&format=markdown)"),
        ("GET", "/sessions/<id>/audit/by-kind", "per-session audit kind histogram (?since-minutes=N&format=markdown)"),
        ("GET", "/sessions/<id>/audit/by-tool", "per-session tool_call+tool_result histogram by tool name (?since-minutes=N&format=markdown)"),
        ("GET", "/sessions/<id>/messages/by-role", "per-session message-role histogram (?since-minutes=N&format=markdown)"),
        ("GET", "/sessions/<id>/audit/tool-errors", "per-session histogram of failed tool calls (?since-minutes=N&format=markdown)"),
        ("GET", "/sessions/<id>/stats", "per-session stats: role histogram + timing envelope + inter-message gap stats (?format=markdown)"),
        ("GET", "/sessions/<id>/turns", "flatten messages into user→assistant→tools turn tuples (?format=markdown)"),
        ("GET", "/skills", "active skills (?include-superseded&name-contains&lock&body-contains&limit&format=markdown)"),
        ("GET", "/skills/count", "count of active skills (?include-superseded=true)"),
        ("GET", "/skills/<name>", "one skill view (full body; ?format=markdown reconstructs the SKILL.md doc)"),
        ("GET", "/skills/<name>/versions", "supersede chain for one skill name (?format=markdown)"),
        ("POST", "/skills", "install a SKILL.md document"),
        ("POST", "/memory", "append a memory entry ({file, body, vertical_id?}); when vertical_id is set the row is tagged for vertical-namespaced reads (#687, Rule 7)"),
        ("GET", "/memory", "memory entries (?file&since-minutes&lock&body-contains&limit&format=markdown&vertical=<id>); ?vertical default-deny excludes legacy NULL rows (#687)"),
        ("GET", "/memory/count", "count of active memory entries (?file=…&since-minutes=N)"),
        ("GET", "/memory/<id>", "one memory entry (full body; ?format=markdown for a paste-able doc)"),
        ("GET", "/memory/<id>/versions", "supersede chain for a memory entry (?format=markdown)"),
        ("GET", "/audit", "audit chain summary (?format=markdown)"),
        ("GET", "/audit/breaks", "actual ChainBreak rows (seq, kind, reason) — empty if chain is clean (?format=markdown)"),
        ("GET", "/audit/count", "count of audit rows (?kind=…&since-minutes=N&session=<uuid>)"),
        ("GET", "/audit/by-kind", "histogram of audit rows by kind (?since-minutes=N&format=markdown)"),
        ("GET", "/audit/by-session", "top-N sessions by audit row count (?limit=N&since-minutes=M&format=markdown; default limit 50, cap 500)"),
        ("GET", "/audit/by-tool", "histogram of tool_call+tool_result rows grouped by tool name (?since-minutes=N&format=markdown)"),
        ("GET", "/audit/by-vertical", "per-vertical audit-row histogram (#688); rows from unbound sessions land in the `(none)` bucket (?since-minutes=N&format=markdown)"),
        ("GET", "/audit/tool-errors", "histogram of tool_result rows where is_error=true, grouped by tool (?since-minutes=N&format=markdown)"),
        ("GET", "/audit/by-day-by-kind", "2D rollup: audit rows bucketed by (day, kind) (?days=N&format=markdown; default 30, cap 365)"),
        ("GET", "/audit/by-day", "audit rows per calendar day (?days=N&format=markdown; default 30, cap 365)"),
        ("GET", "/messages/by-day", "messages created per calendar day (?days=N&format=markdown; default 30, cap 365)"),
        ("GET", "/messages/by-role", "histogram of messages by role (?since-minutes=N&format=markdown)"),
        ("GET", "/messages/by-session", "top-N sessions by message row count (?limit=N&since-minutes=M&format=markdown; default limit 50, cap 500)"),
        ("GET", "/messages/recent", "workspace-wide recent message tail (?limit=50&role=user&since-minutes=M&format=markdown; default limit 50, cap 500)"),
        ("GET", "/messages/by-day-by-role", "2D rollup: messages bucketed by (day, role) (?days=N&format=markdown; default 30, cap 365)"),
        ("GET", "/audit/recent", "most recent N audit rows incl. per-row vertical_id (#691; ?limit=50&kind=turn_start|...&since-minutes=M&session=<uuid>&format=markdown)"),
        ("GET", "/audit/sessions/<id>", "per-session audit chain incl. per-row vertical_id (#691; ?kind&since-minutes&format=markdown)"),
        ("GET", "/audit/sessions/<id>/verify", "verify a session's chain integrity (?include-breaks=true&format=markdown)"),
        ("GET", "/costs", "cost ceilings (?format=markdown)"),
        ("GET", "/costs/total", "actual spend roll-up across all tenants today/this_month/all_time (?format=markdown)"),
        ("GET", "/costs/by-model", "spend grouped by (provider, model) (?format=markdown)"),
        ("GET", "/costs/by-session", "spend grouped by session_id (?format=markdown)"),
        ("GET", "/costs/by-provider", "spend grouped by provider only (no model split; ?format=markdown)"),
        ("GET", "/costs/by-day", "spend rolled up per calendar day (?days=N&format=markdown; default 30, cap 365)"),
        ("GET", "/costs/recent", "workspace-wide recent spend ledger entries (?limit=50&since-minutes=M&format=markdown; default 50, cap 500)"),
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
        ("GET", "/forge/runs/<id>/payload", "queued payload JSON for the run; ?format=markdown wraps it in a paste-able fenced block; 404 when the row has no forge_queue entry (e.g. ran via ?in_process=true)"),
        ("GET", "/forge/runs/<id>/audit", "audit rows linked to one forge run via payload's $.run_id (?format=markdown)"),
        ("GET", "/forge/runs/count", "count of forge runs (?status=running|...&since-minutes=N)"),
        ("GET", "/forge/runs/by-day", "forge runs started per calendar day (?days=N&format=markdown; default 30, cap 365)"),
        ("GET", "/forge/runs/by-status", "histogram of forge_run rows by status (?since-minutes=N&format=markdown)"),
        ("GET", "/forge/runs/by-skill", "histogram of forge_run rows by skill_name (?since-minutes=N&format=markdown)"),
        ("GET", "/cron", "scheduled cron jobs (workspace cron/jobs/*.json; ?format=markdown for a 6-column table)"),
        ("GET", "/cron/<name>", "one cron job by name (?format=markdown for a paste-able doc)"),
        ("POST", "/cron", "create a new cron job ({name, schedule, prompt, platform, chat})"),
        ("POST", "/cron/<name>/delete", "remove the on-disk job file"),
        ("POST", "/workspace/init", "register a workspace row ({name, root_path})"),
        ("POST", "/judge/<record_id>/override", "override a judge verdict ({to})"),
        ("GET", "/doctor", "JSON snapshot of `argo doctor` (?format=markdown for a paste-able status doc)"),
        ("GET", "/health/full", "deep health (chain + skill + memory)"),
        ("GET", "/routes", "this listing (?method=GET|POST&format=markdown)"),
        ("GET", "/metrics", "Prometheus-style text exposition"),
        ("GET", "/metrics/json", "JSON variant of /metrics for clients that don't speak Prometheus text"),
        ("POST", "/metrics/reset", "zero out the scrape counter; returns {previous, current}"),
        ("GET", "/db/stats", "per-table row counts (sessions, messages, audit, …; ?format=markdown)"),
        ("GET", "/db/version", "current SQLite schema migration version (?format=markdown)"),
        ("GET", "/db/integrity", "PRAGMA integrity_check; returns {ok, check} (?format=markdown)"),
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
        ("GET", "/sessions/<id>/grants", "list active human-approval grants for this session (#666); returns {session_id, count, grants:[…]}"),
        ("POST", "/sessions/<id>/skills/attach", "attach a skill ({skill: name})"),
        ("POST", "/sessions/<id>/skills/detach", "detach a skill ({skill: name})"),
        ("POST", "/sessions/<id>/skills/clear", "detach every attached skill"),
        ("POST", "/sessions/<id>/delete", "delete a session + dependent rows"),
        ("POST", "/sessions/<id>/title", "rename a session ({title}, empty clears)"),
        ("POST", "/sessions/<id>/end", "mark a session ended (stamps ended_at)"),
        ("POST", "/sessions/<id>/reopen", "clear ended_at — inverse of /end"),
        ("POST", "/sessions/<id>/clone", "duplicate a session into a fresh row, copying every message; returns {cloned_from, session_id, title, messages_copied}"),
        ("POST", "/sessions/<id>/branch", "fork a session at a specific message (?through-seq=N copies seq 1..=N; defaults to all); returns {branched_from, session_id, title, through_seq, messages_copied}"),
        ("POST", "/sessions/<id>/approve", "grant the session permission to dispatch a tool the bound vertical's [escalation].human_approval_required would block; body {tool, note?} (#665)"),
        ("POST", "/sessions/<id>/revoke-approval", "revoke every grant for a (session, tool) pair; body {tool}; returns {removed} (#665)"),
        ("POST", "/sessions/end-all", "mark open sessions ended (?older-than-minutes=N for stale-only)"),
        ("POST", "/forge/optimize", "enqueue `argo forge optimize` for the daemon to run (returns 202; `?in_process=true` keeps the old in-gateway behaviour)"),
        ("POST", "/forge/optimize/stream", "chunked-NDJSON forge optimize event stream"),
        ("POST", "/forge/bench", "kick off `argo forge bench` (returns 202)"),
        ("POST", "/forge/runs/<id>/abort", "abort an in-flight forge run"),
        ("POST", "/forge/runs/abort-all", "abort every in-flight forge run"),
        ("POST", "/forge/runs/sweep", "flip stalled running rows older than ?older-than-minutes=60 to failed (gateway-crash recovery)"),
        ("POST", "/forge/runs/<id>/promote", "promote that run's candidate to active"),
        ("POST", "/forge/runs/<id>/requeue", "copy a terminal run's queued payload into a fresh queued run; returns 201 + {requeued, source_id, run_id}"),
        ("POST", "/skills/<name>/promote", "promote a skill version to active"),
        ("POST", "/skills/<name>/lock", "lock a skill (user-protected by default)"),
        ("POST", "/skills/<name>/unlock", "clear a skill lock"),
        ("POST", "/skills/<name>/refine", "supersede with hint-appended body ({hint, version?})"),
        ("POST", "/memory/<id>/lock", "lock a memory entry"),
        ("POST", "/memory/<id>/unlock", "clear a memory-entry lock"),
        ("POST", "/memory/<id>/forget", "revert (supersede) a memory entry"),
        ("POST", "/memory/<id>/edit", "supersede with a new body ({body})"),
        // ── /api/v1/* — versioned API surface consumed by the SPA. Keep
        // this block grouped so `scripts/check-routes-catalog-complete.sh`
        // (CI drift gate) can sanity-check coverage vs the dispatchers in
        // `routes_dispatch_get_api*.rs`, `routes_dispatch_post*.rs`,
        // `routes_dispatch_patch.rs`, and `routes_dispatch_delete.rs`.
        ("GET", "/api/v1/auth/csrf", "issue a CSRF token cookie + body for SPA flows"),
        ("GET", "/api/v1/meta/error-reporting", "client-side error-reporting metadata (sentry dsn, sample rate)"),
        ("GET", "/api/v1/workspace/members", "list workspace members (?role&status)"),
        ("POST", "/api/v1/workspace/members/invite", "invite a workspace member ({email, role})"),
        ("POST", "/api/v1/workspace/members/accept", "accept a workspace-member invite ({token})"),
        ("GET", "/api/v1/team", "deprecated alias of /api/v1/workers — legacy SPA shim"),
        ("GET", "/api/v1/workers", "list AI teammates (?status&cohort&vertical&format=markdown)"),
        ("POST", "/api/v1/workers", "hire a new teammate ({worker_draft, problem, answers})"),
        ("GET", "/api/v1/workers/<id>", "worker detail (?format=markdown for a paste-able profile)"),
        ("PATCH", "/api/v1/workers/<id>", "update worker settings: alias of /api/v1/workers/<id>/<channels|quiet-hours|budget|autonomy>"),
        ("DELETE", "/api/v1/workers/<id>", "hard-delete a worker + dependent rows (J11)"),
        ("GET", "/api/v1/workers/<id>/overview", "worker dashboard overview tile"),
        ("GET", "/api/v1/workers/<id>/activity", "worker activity feed (?since-minutes&limit)"),
        ("GET", "/api/v1/workers/<id>/evals", "worker evaluation history"),
        ("GET", "/api/v1/workers/<id>/memory", "worker memory entries (?file&limit)"),
        ("GET", "/api/v1/workers/<id>/settings", "worker settings: channels, quiet-hours, budget, autonomy"),
        ("GET", "/api/v1/workers/<id>/skills", "skills attached to this worker"),
        ("GET", "/api/v1/workers/<id>/skills/<sid>/history", "version history for one skill on one worker"),
        ("GET", "/api/v1/workers/<id>/policies", "worker policy rows (?action_type)"),
        ("GET", "/api/v1/workers/<id>/retros", "retros authored by this worker"),
        ("GET", "/api/v1/workers/<id>/trust-grade", "worker trust-grade + last recompute"),
        ("GET", "/api/v1/workers/<id>/trust-streaks", "worker trust-streak history"),
        ("POST", "/api/v1/workers/<id>/pause", "pause a worker"),
        ("POST", "/api/v1/workers/<id>/resume", "resume a paused worker"),
        ("POST", "/api/v1/workers/<id>/archive", "archive a worker (soft-delete; preserves history)"),
        ("POST", "/api/v1/workers/<id>/freeze", "freeze a worker (halts new actions; trust-incident response)"),
        ("POST", "/api/v1/workers/<id>/unfreeze", "unfreeze a frozen worker"),
        ("POST", "/api/v1/workers/<id>/trust-grade/recompute", "on-demand trust-grade recompute"),
        ("PATCH", "/api/v1/workers/<id>/channels", "update worker channels ({channels:[...]})"),
        ("PATCH", "/api/v1/workers/<id>/quiet-hours", "update worker quiet-hours window"),
        ("PATCH", "/api/v1/workers/<id>/budget", "update worker budget caps"),
        ("PATCH", "/api/v1/workers/<id>/autonomy", "update worker autonomy band"),
        ("PATCH", "/api/v1/workers/<id>/policies/<action_type>", "update one policy row for this worker"),
        ("GET", "/api/v1/cohorts", "list cohorts across departments"),
        ("GET", "/api/v1/cohorts/<id>", "one cohort detail (?format=markdown)"),
        ("POST", "/api/v1/cohorts/<id>/bulk-assign-skill", "attach a skill to every worker in a cohort"),
        ("GET", "/api/v1/skills/marketplace", "marketplace listing — installable skill packs"),
        ("GET", "/api/v1/skills/<name>/ir", "skill IR (deterministic representation)"),
        ("POST", "/api/v1/skills/<name>/ir/import", "import a skill IR ({body})"),
        ("GET", "/api/v1/skills/<name>/ir/export", "export a skill IR (paste-able)"),
        ("GET", "/api/v1/skills/<name>/usage", "skill usage histogram (?since-minutes)"),
        ("POST", "/api/v1/skills/draft-from-prompt", "draft a SKILL.md from a plain-English prompt ({prompt})"),
        ("GET", "/api/v1/audit", "audit feed (?actor&kind&since-minutes&limit)"),
        ("GET", "/api/v1/audit/<id>", "one audit row detail (?format=markdown)"),
        ("GET", "/api/v1/audit/screenshots/<id>", "screenshot bytes for an audit row"),
        ("GET", "/api/v1/audit/elevated-view-grants", "list active elevated-view grants"),
        ("POST", "/api/v1/audit/elevated-view-request", "request elevated view of audit content ({audit_id, reason})"),
        ("DELETE", "/api/v1/audit/elevated-view-grants/<id>", "revoke an elevated-view grant"),
        ("GET", "/api/v1/approvals", "approvals inbox feed (?status&worker&since-minutes)"),
        ("POST", "/api/v1/approvals/<id>/decide", "decide one approval ({decision:'approve'|'deny', note?})"),
        ("GET", "/api/v1/approvals/bundles", "list approval bundles (?status&since-minutes)"),
        ("POST", "/api/v1/approvals/bundles", "create an approval bundle ({approvals:[...]})"),
        ("POST", "/api/v1/approvals/bundles/<id>/decide", "decide one approval bundle ({decision, note?})"),
        ("GET", "/api/v1/notifications", "notifications inbox (?since-minutes&limit)"),
        ("GET", "/api/v1/notifications/unread-count", "unread notification count"),
        ("POST", "/api/v1/notifications/<id>/read", "mark one notification read"),
        ("POST", "/api/v1/notifications/<id>/dismiss", "dismiss one notification"),
        ("POST", "/api/v1/notifications/dismiss-all", "dismiss every notification in the inbox"),
        ("GET", "/api/v1/inbound-channels/<kind>/recent", "recent operator-visible inbound webhook events for one channel kind (SMS, Discord, voice, email, Slack inbox); backs /settings/inbound-channels Scene-6 surface"),
        ("GET", "/api/v1/orchestration/runs", "orchestration run history (?status&since-minutes)"),
        ("POST", "/api/v1/orchestration/team-runs", "run a Supervisor team plan — lead fans delegations out in parallel with depends_on dependency layers, persists each as an orchestration run"),
        ("POST", "/api/v1/orchestration/runs/<id>/cancel", "cancel an in-flight orchestration run"),
        ("POST", "/api/v1/orchestration/runs/<id>/force-escalate", "force-escalate a running orchestration"),
        ("POST", "/api/v1/orchestration/runs/<id>/reset-retry", "reset retry counter for a stuck run"),
        ("GET", "/api/v1/retros", "retro feed (?q&since-minutes)"),
        ("GET", "/api/v1/dashboards/error-rate", "error-rate dashboard JSON (?window=…)"),
        ("GET", "/api/v1/dashboards/sla", "SLA dashboard JSON (?window=…)"),
        ("GET", "/api/v1/support/tickets", "support ticket list (?status&assignee)"),
        ("GET", "/api/v1/support/tickets/<id>", "one support ticket detail"),
        ("POST", "/api/v1/support/tickets", "open a new support ticket ({title, body, severity})"),
        ("PATCH", "/api/v1/support/tickets/<id>", "update support ticket fields"),
        ("POST", "/api/v1/support/tickets/<id>/reply", "post a reply to a support ticket"),
        ("GET", "/api/v1/verticals", "list verticals (api-prefixed alias of /verticals)"),
        ("GET", "/api/v1/verticals/<id>", "vertical detail (api-prefixed alias)"),
        ("GET", "/api/v1/tenant/usage", "tenant usage roll-up ($ spend, action counts, quota state)"),
        ("GET", "/api/v1/wizard", "first-run wizard state (?step=…)"),
        ("POST", "/api/v1/wizard/answer", "submit one wizard answer ({step, value})"),
        ("POST", "/api/v1/wizard/reset", "reset wizard state to step 1"),
        ("POST", "/api/v1/onboarding/first-hire", "onboarding first-hire commit"),
        ("POST", "/api/v1/onboarding/install-vertical", "install a vertical pack during onboarding"),
        ("POST", "/api/v1/onboarding/provider-key", "store a provider API key during onboarding"),
        ("POST", "/api/v1/onboarding/provision", "provision the workspace during onboarding"),
        ("POST", "/api/v1/auth/signup", "magic-link signup ({email})"),
        ("POST", "/api/v1/auth/verify", "verify a magic-link token ({token})"),
        ("POST", "/api/v1/billing/checkout", "create a Stripe checkout session"),
        ("POST", "/api/v1/billing/portal", "create a Stripe customer-portal session"),
        ("POST", "/api/v1/billing/webhook", "Stripe webhook receiver (signed)"),
        ("POST", "/api/v1/demo/mint-principal", "mint a demo principal cookie for the v1 demo"),
        ("POST", "/api/v1/genesis/classify", "classify a plain-English role description for Genesis"),
        ("POST", "/api/v1/genesis/draft-worker", "draft a worker_draft from a Genesis classification"),
        ("GET", "/api/v1/me/export/<id>", "fetch a personal-data export by id"),
        ("POST", "/api/v1/me/export", "request a personal-data export"),
        ("POST", "/api/v1/audit/<id>/replay", "replay an audit-row's tool dispatch in a fresh session"),
        ("GET", "/api/v1/packs", "installed marketplace packs"),
        ("GET", "/api/v1/knowledge", "workplace-knowledge doc index (?category=…)"),
        ("GET", "/api/v1/knowledge/<slug>", "one workplace-knowledge doc"),
        ("POST", "/api/v1/knowledge", "create a workplace-knowledge doc ({slug, title, body, category?})"),
        ("PUT", "/api/v1/knowledge/<slug>", "update a workplace-knowledge doc ({title?, body?, category?})"),
        ("DELETE", "/api/v1/knowledge/<slug>", "delete a workplace-knowledge doc (owner only)"),
        ("GET", "/api/v1/knowledge/<slug>/revisions", "revision history for one knowledge doc"),
        ("GET", "/api/v1/workflows", "top-level workflows index across verticals (?vertical&format=markdown)"),
        ("GET", "/api/v1/workflows/list", "raw workflows list for the SPA workflows page"),
        ("POST", "/api/v1/verticals/<v>/workflows/<n>/execute", "execute one workflow against a vertical ({inputs})"),
        ("GET", "/api/v1/verticals/<v>/workflows/<n>/runs", "history of runs for one workflow"),
        ("GET", "/api/v1/skill-library/list", "skill-library catalog: shared + tenant-private skills"),
        ("POST", "/api/v1/auth/logout", "clear the SSO session cookie"),
        // ── settings: SKYCOT service tokens + BYOK provider keys
        ("GET", "/api/v1/api-keys", "list SKYCOT service tokens for the tenant (metadata only, never the secret value)"),
        ("POST", "/api/v1/api-keys", "mint a named, scoped service token ({name, scopes?}); returns the secret once"),
        ("DELETE", "/api/v1/api-keys/<id>", "revoke one service token (tenant-verified)"),
        ("GET", "/api/v1/providers/keys", "BYOK upstream-provider key status (configured/missing per family; metadata only)"),
        ("GET", "/api/v1/providers/policy", "provider routing-policy panel for the settings page"),
        ("PATCH", "/api/v1/providers/policy", "update the provider routing-policy panel"),
        // ── governance readiness snapshot
        ("GET", "/api/v1/governance/readiness", "governance readiness snapshot (controls + gaps)"),
        // ── GTM + marketing agent drafting
        ("POST", "/api/v1/gtm/draft-followup", "draft a go-to-market follow-up message from a prompt"),
        ("POST", "/api/v1/marketing/draft-response", "draft a marketing response from a prompt"),
        // ── pitch agent (v1 demo headline) — truth-bounded Q&A
        ("POST", "/api/v1/pitch/ask", "ask the pitch agent a question (truth-bounded Q&A)"),
        // ── voice I/O (pitch + coach + live transcripts)
        ("GET", "/api/v1/voice/sessions", "list voice sessions (?mode=coach|pitch)"),
        ("GET", "/api/v1/voice/settings", "voice settings panel (provider, voice id, model)"),
        ("PATCH", "/api/v1/voice/settings", "update the voice settings panel"),
        ("POST", "/api/v1/voice/tts", "text-to-speech synthesis for a voice turn"),
        ("POST", "/api/v1/voice/stt", "speech-to-text transcription of an audio clip"),
        ("POST", "/api/v1/voice/pitch-turn", "run one pitch voice turn (STT → pitch answer → TTS)"),
        ("POST", "/api/v1/voice/realtime-session", "mint a realtime voice session token"),
        ("POST", "/api/v1/voice/transcript", "append a voice transcript row (hash-chained audit)"),
        // ── public early-access waitlist capture (WS6)
        ("POST", "/api/v1/waitlist", "capture a public early-access waitlist signup ({email})"),
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
        let d = row["description"]
            .as_str()
            .unwrap_or("")
            .replace('|', "\\|");
        out.push_str(&format!("| `{m}` | `{p}` | {d} |\n"));
    }
    ("200 OK", out)
}

#[cfg(test)]
mod tests {
    use super::routes_response;
    use serde_json::Value;

    fn parse_paths() -> Vec<(String, String)> {
        let (_status, body) = routes_response(None);
        let parsed: Value = serde_json::from_str(&body).expect("catalog must be valid JSON");
        parsed
            .as_array()
            .expect("catalog must be a JSON array")
            .iter()
            .map(|row| {
                (
                    row["method"].as_str().unwrap_or("").to_string(),
                    row["path"].as_str().unwrap_or("").to_string(),
                )
            })
            .collect()
    }

    /// F8 + F28 (ux-e2e-audit-T2309MEL): `/api/v1/*` was entirely
    /// absent from the catalog. Floor at 15 so future drift surfaces
    /// here rather than via a fresh-agent debugging session.
    #[test]
    fn routes_catalog_includes_v1_namespace() {
        let paths = parse_paths();
        let v1: Vec<_> = paths
            .iter()
            .filter(|(_, p)| p.starts_with("/api/v1"))
            .collect();
        assert!(
            v1.len() >= 15,
            "expected >= 15 /api/v1/* paths in catalog, got {}",
            v1.len()
        );
    }

    /// Spot-check the load-bearing SPA surfaces called out in the
    /// brief's acceptance criteria. Each `(method, path)` is a
    /// dispatcher-registered route the SPA fetches; the catalog must
    /// list at least one entry with the exact path.
    #[test]
    fn routes_catalog_covers_key_v1_paths() {
        let paths = parse_paths();
        let must_have = [
            ("GET", "/api/v1/workers"),
            ("POST", "/api/v1/workers"),
            ("GET", "/api/v1/workers/<id>"),
            ("PATCH", "/api/v1/workers/<id>"),
            ("DELETE", "/api/v1/workers/<id>"),
            ("GET", "/api/v1/team"),
            ("GET", "/api/v1/audit"),
            ("GET", "/api/v1/approvals"),
            ("GET", "/api/v1/skills/marketplace"),
            ("GET", "/api/v1/cohorts"),
            ("GET", "/api/v1/notifications"),
            ("GET", "/api/v1/orchestration/runs"),
            ("POST", "/api/v1/genesis/classify"),
            ("GET", "/api/v1/verticals"),
        ];
        for (method, path) in must_have {
            assert!(
                paths
                    .iter()
                    .any(|(m, p)| m == method && p == path),
                "catalog missing required route: {method} {path}"
            );
        }
    }

    /// `/routes?method=GET` filtering must not drop the v1 namespace.
    #[test]
    fn routes_catalog_get_filter_keeps_v1() {
        let (_status, body) = routes_response(Some("GET"));
        let parsed: Value = serde_json::from_str(&body).unwrap();
        let arr = parsed.as_array().unwrap();
        let v1_get = arr
            .iter()
            .filter(|row| {
                row["method"].as_str() == Some("GET")
                    && row["path"]
                        .as_str()
                        .map(|p| p.starts_with("/api/v1"))
                        .unwrap_or(false)
            })
            .count();
        assert!(
            v1_get >= 10,
            "GET-filtered catalog must keep >= 10 /api/v1 paths, got {v1_get}"
        );
    }
}
