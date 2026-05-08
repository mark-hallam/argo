//! `argo gateway start` boot-time route summary banner — printed once
//! when the gateway begins listening so an operator can see at a
//! glance what endpoints the process serves. Extracted from `mod.rs`
//! to keep that file under the 500-LOC ceiling.
//!
//! Configurable via `start_banner_config::load_layered`. Default
//! output (no env vars / TOML / `--quiet`) is byte-identical to the
//! pre-config behaviour: the 73-line route catalog + the trailing
//! "press Ctrl-C…" footer, written to stderr.

/// Canonical default banner body. Each line ends with `\n`. Embedded
/// in the `BannerMode::Default` / `BannerMode::Custom` render paths
/// so the route catalog stays a single source of truth.
pub const DEFAULT_BANNER_BODY: &str = concat!(
    "  GET  /health   → {\"status\":\"ok\"}\n",
    "  GET  /version  → {\"version\":\"<crate>\"}\n",
    "  GET  /sessions[?since-minutes=N] → recent sessions (JSON array; carries spent_usd_micros)\n",
    "  GET  /sessions/<uuid> → one session header + message_count + tool_call_count + spend\n",
    "  GET  /sessions/<uuid>/messages[?limit=N&role=user&since-minutes=M] → messages on the session\n",
    "  GET  /sessions/<uuid>/messages/last → just the last message\n",
    "  GET  /sessions/<uuid>/spend → per-session spend ledger rows + total\n",
    "  GET  /sessions/<uuid>/spend/total → single-number total for one session\n",
    "  POST /sessions → create a session row ({model, provider, title?})\n",
    "  POST /sessions/<uuid>/messages → append a message ({role, content})\n",
    "  POST /sessions/<uuid>/runs → run one agent turn ({user?, stream:false, max_tool_rounds?})\n",
    "  POST /sessions/<uuid>/runs/stream → chunked-NDJSON stream of audit events as the run progresses\n",
    "  POST /sessions/<uuid>/steer → inject a system-role steer ({text})\n",
    "  POST /sessions/<uuid>/replay → replay user prompts in a fresh session ({model?, provider?})\n",
    "  GET  /sessions/<uuid>/skills → skills attached to the session\n",
    "  GET  /sessions/<uuid>/judge  → judge_record rows for the session\n",
    "  POST /sessions/<uuid>/skills/{attach,detach} → manage per-session skill set\n",
    "  POST /sessions/<uuid>/delete → delete a session + dependent rows\n",
    "  POST /sessions/<uuid>/title  → rename ({title}; empty clears)\n",
    "  POST /sessions/<uuid>/end    → mark a session ended (stamps ended_at)\n",
    "  POST /sessions/<uuid>/replay/stream → chunked-NDJSON stream of replay events\n",
    "  POST /channels/email/webhook → Postmark inbound approval webhook\n",
    "  GET  /skills   → active skills (?include-superseded=true for full history)\n",
    "  GET  /skills/<name> → one skill view (full body + frontmatter)\n",
    "  GET  /skills/<name>/versions → supersede chain for that name\n",
    "  POST /skills   → install a SKILL.md document ({raw})\n",
    "  POST /memory   → append a memory entry ({file: memory|user|soul|agents, body})\n",
    "  GET  /memory[?file=memory|user|soul|agents] → active memory entries\n",
    "  GET  /memory/<id> → one memory entry (full body)\n",
    "  GET  /memory/<id>/versions → supersede chain for that entry\n",
    "  GET  /audit    → audit chain summary (ok / rows / breaks / kinds)\n",
    "  GET  /providers → workspace provider manifests + their model lists\n",
    "  GET  /providers/<id> → one provider manifest by basename\n",
    "  GET  /providers/<id>/models → just the [[models]] array\n",
    "  GET  /tools    → registered in-core tools + JSON-Schema parameters\n",
    "  GET  /tools/<name> → one tool's name + description + parameters\n",
    "  GET  /search?q=…[&limit=N&session=<id>] → FTS5 hits across the message corpus\n",
    "  GET  /workspace/info → first workspace row (id, name, root_path, created_at)\n",
    "  GET  /workspace/artifacts[?limit=N] → recorded marketplace pushes (oci_ref + digest + pushed_at)\n",
    "  GET  /forge/runs[?limit=N&since-minutes=M] → recorded forge runs\n",
    "  GET  /forge/runs/<id>      → one forge_run row\n",
    "  GET  /cron                 → scheduled cron jobs (workspace cron/jobs/)\n",
    "  GET  /cron/<name>          → one cron job by name\n",
    "  POST /cron                 → create a job ({name, schedule, prompt, platform, chat})\n",
    "  POST /workspace/init       → register a workspace row ({name, root_path})\n",
    "  POST /judge/<record_id>/override → override a judge verdict ({to})\n",
    "  POST /cron/<name>/delete   → remove the on-disk job file\n",
    "  POST /forge/optimize → kick off `argo forge optimize` ({skill, examples, iterations?, budget_usd?})\n",
    "  POST /forge/optimize/stream → chunked-NDJSON live forge progress\n",
    "  POST /forge/bench    → kick off `argo forge bench` ({skill, examples, provider?, model?})\n",
    "  POST /forge/runs/<id>/abort → flip a run's status to 'aborted'\n",
    "  POST /forge/runs/<id>/promote → promote that run's candidate to active\n",
    "  POST /skills/<name>/promote → promote a version to active ({version})\n",
    "  POST /skills/<name>/{lock,unlock,refine} → lock / unlock / append hint refine\n",
    "  POST /memory/<id>/{lock,unlock,forget,edit} → admin ops on a memory entry\n",
    "  GET  /metrics        → Prometheus-style text exposition (text/plain)\n",
    "  GET  /routes  → discovery: every route + a one-line description\n",
    "  GET  /doctor       → JSON snapshot of `argo doctor` (cwd, db, cron, creds, …)\n",
    "  GET  /health/full  → deep health (audit chain + skill + memory provenance)\n",
    "  GET  /audit/sessions/<uuid>[?kind=<kind>] → per-session audit chain (JSON array)\n",
    "  GET  /audit/sessions/<uuid>/verify → verify a session's chain integrity\n",
    "  GET  /costs    → cost ceilings (per_session/day/month + notify_at_pct)\n",
    "  GET  /db/stats → per-table row counts (sessions, messages, audit, …)\n",
    "  POST /db/vacuum → run VACUUM on the SQLite store\n",
    "  POST /db/prune  → prune sessions older than N days ({older_than_days?})\n",
    "  GET  /costs/by-model    → top spend buckets grouped by (provider, model)\n",
    "  GET  /costs/by-session  → top spend buckets grouped by session_id\n",
    "press Ctrl-C or `argo gateway stop` to terminate\n",
);

/// What kind of banner to render. `Default` mirrors the pre-config
/// route catalog. `Custom` keeps the catalog and wraps it with the
/// configured header / footer / extra fields. `Quiet` suppresses
/// output entirely.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BannerMode {
    /// Pre-config behaviour — emit the 73-line route catalog verbatim.
    #[default]
    Default,
    /// Render the catalog wrapped in operator-supplied header / footer.
    Custom,
    /// Suppress all banner output.
    Quiet,
}

/// Operator-supplied banner overrides. `Default::default()` reproduces
/// the pre-config behaviour byte-for-byte. Construct directly or via
/// `start_banner_config::load_layered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BannerConfig {
    /// Master enable. `false` short-circuits to empty output even if
    /// `mode == Custom`. Defaults to `true`.
    pub enabled: bool,
    /// Optional line printed above the route catalog (Custom mode).
    pub header: Option<String>,
    /// Optional line printed below the catalog + extra fields (Custom).
    pub footer: Option<String>,
    /// Free-form `(label, value)` rows printed between the catalog and
    /// the footer (Custom). Empty by default.
    pub extra_fields: Vec<(String, String)>,
    /// Render mode — see [`BannerMode`].
    pub mode: BannerMode,
}

impl Default for BannerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            header: None,
            footer: None,
            extra_fields: Vec::new(),
            mode: BannerMode::Default,
        }
    }
}

/// Render the banner to a string. Returns `""` when the banner is
/// disabled or `mode == Quiet`. The newline structure of the catalog
/// body is preserved exactly so a caller emitting the result via
/// `eprint!` produces byte-identical default output.
#[must_use]
pub fn render(config: &BannerConfig) -> String {
    if !config.enabled || matches!(config.mode, BannerMode::Quiet) {
        return String::new();
    }
    match config.mode {
        BannerMode::Default => DEFAULT_BANNER_BODY.to_string(),
        BannerMode::Custom => render_custom(config),
        BannerMode::Quiet => String::new(),
    }
}

fn render_custom(config: &BannerConfig) -> String {
    let mut out = String::with_capacity(DEFAULT_BANNER_BODY.len() + 256);
    if let Some(h) = config.header.as_deref() {
        out.push_str(h);
        if !h.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str(DEFAULT_BANNER_BODY);
    for (k, v) in &config.extra_fields {
        out.push_str("  ");
        out.push_str(k);
        out.push('=');
        out.push_str(v);
        out.push('\n');
    }
    if let Some(f) = config.footer.as_deref() {
        out.push_str(f);
        if !f.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

/// Emit the configured banner to stderr. With a default config this
/// preserves the pre-config byte sequence exactly: the 73-line route
/// catalog written line-by-line.
pub(super) fn print_banner(config: &BannerConfig) {
    let text = render(config);
    if !text.is_empty() {
        eprint!("{text}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_render_byte_identical_to_canonical_body() {
        // Acceptance #1: default `argo gateway start` output is
        // byte-for-byte the pre-config route catalog. Pre-config
        // print_endpoint_summary() emitted 68 distinct eprintln lines.
        let body = render(&BannerConfig::default());
        assert_eq!(body, DEFAULT_BANNER_BODY);
        assert!(body.starts_with("  GET  /health"));
        assert!(body.ends_with("press Ctrl-C or `argo gateway stop` to terminate\n"));
        assert_eq!(body.lines().count(), 68, "pre-config banner had 68 lines");
    }

    #[test]
    fn quiet_mode_renders_empty() {
        let cfg = BannerConfig {
            mode: BannerMode::Quiet,
            ..BannerConfig::default()
        };
        assert!(render(&cfg).is_empty());
    }

    #[test]
    fn disabled_short_circuits_even_in_custom_mode() {
        let cfg = BannerConfig {
            enabled: false,
            mode: BannerMode::Custom,
            header: Some("ignored".into()),
            ..BannerConfig::default()
        };
        assert!(render(&cfg).is_empty());
    }

    #[test]
    fn custom_mode_wraps_catalog_with_header_and_footer() {
        let cfg = BannerConfig {
            mode: BannerMode::Custom,
            header: Some("=== prod / us-east-1 ===".into()),
            footer: Some("=== build deadbeef ===".into()),
            extra_fields: vec![("region".into(), "us-east-1".into())],
            ..BannerConfig::default()
        };
        let body = render(&cfg);
        assert!(body.starts_with("=== prod / us-east-1 ===\n"));
        assert!(body.contains(DEFAULT_BANNER_BODY));
        assert!(body.contains("\n  region=us-east-1\n"));
        assert!(body.ends_with("=== build deadbeef ===\n"));
    }

    #[test]
    fn print_banner_does_not_panic_for_quiet_or_default() {
        // Smoke: the public emitter handles both branches.
        print_banner(&BannerConfig::default());
        let quiet = BannerConfig {
            mode: BannerMode::Quiet,
            ..BannerConfig::default()
        };
        print_banner(&quiet);
    }
}
