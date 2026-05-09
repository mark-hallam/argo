//! Named-const defaults for every workspace-level config field.
//!
//! Centralizes the values that previously lived as inline
//! `unwrap_or(...)` in dozens of handler callsites. Handlers should
//! reference these consts (e.g.
//! [`DEFAULT_HANDOFF_SWEEP_SECS`](self::DEFAULT_HANDOFF_SWEEP_SECS))
//! instead of inlining literals so that one edit here re-tunes the
//! whole workspace.

/// Project-local config filename (under cwd).
pub const PROJECT_LOCAL_CONFIG: &str = "argo.toml";

/// Forward-compatible Skycot-rebrand project-local config path
/// (preferred when present).
pub const PROJECT_LOCAL_SKYCOT_CONFIG: &str = ".skycot/argo.toml";

/// User-level config path under `$XDG_CONFIG_HOME` / `~/.config`.
pub const USER_CONFIG_RELATIVE: &str = "argo/argo.toml";

// --- [core] ----------------------------------------------------------

/// `core.home` default — `~/.argo`, expanded at resolve time.
pub const DEFAULT_HOME: &str = "~/.argo";

/// `core.workspace` default — current directory.
pub const DEFAULT_WORKSPACE: &str = ".";

/// `core.profile` default.
pub const DEFAULT_PROFILE: &str = "default";

// --- [tenant] --------------------------------------------------------

/// `tenant.mode` default.
pub const DEFAULT_TENANT_MODE: &str = "single";

/// `tenant.default_id` default — the synthetic single-tenant id.
pub const DEFAULT_TENANT_ID: &str = "default";

/// `tenant.auth_provider` default.
pub const DEFAULT_AUTH_PROVIDER: &str = "none";

// --- [telemetry] -----------------------------------------------------

/// `telemetry.enabled` default.
pub const DEFAULT_TELEMETRY_ENABLED: bool = true;

/// `telemetry.exporter` default.
pub const DEFAULT_TELEMETRY_EXPORTER: &str = "stdout";

// --- [audit] ---------------------------------------------------------

/// `audit.chain_algo` default.
pub const DEFAULT_AUDIT_CHAIN_ALGO: &str = "sha256";

/// `audit.export_format` default.
pub const DEFAULT_AUDIT_EXPORT_FORMAT: &str = "ndjson";

// --- [budget] --------------------------------------------------------

/// `budget.default_iterations` default.
pub const DEFAULT_BUDGET_ITERATIONS: u32 = 60;

/// `budget.default_spend_per_session_usd_micros` default ($1).
pub const DEFAULT_SPEND_PER_SESSION_USD_MICROS: u64 = 1_000_000;

/// `budget.default_spend_per_day_usd_micros` default ($10).
pub const DEFAULT_SPEND_PER_DAY_USD_MICROS: u64 = 10_000_000;

/// `budget.default_spend_per_month_usd_micros` default ($100).
pub const DEFAULT_SPEND_PER_MONTH_USD_MICROS: u64 = 100_000_000;

/// `budget.notify_at_pct` default thresholds.
pub const DEFAULT_NOTIFY_AT_PCT: &[u8] = &[80, 95];

/// `budget.on_max_iterations` default.
pub const DEFAULT_ON_MAX_ITERATIONS: &str = "prompt_user";

/// `budget.auto_continue_max_extensions` default.
pub const DEFAULT_AUTO_CONTINUE_MAX_EXTENSIONS: u32 = 3;

/// `budget.auto_continue_extension_size` default.
pub const DEFAULT_AUTO_CONTINUE_EXTENSION_SIZE: u32 = 30;

// --- [delegate] ------------------------------------------------------

/// `delegate.max_spawn_depth` default.
pub const DEFAULT_MAX_SPAWN_DEPTH: u32 = 1;

/// `delegate.max_concurrent_children` default.
pub const DEFAULT_MAX_CONCURRENT_CHILDREN: u32 = 4;

/// `delegate.child_default_iterations` default.
pub const DEFAULT_CHILD_ITERATIONS: u32 = 30;

// --- [parallel] ------------------------------------------------------

/// `parallel.max_parallel_tools` default.
pub const DEFAULT_MAX_PARALLEL_TOOLS: u32 = 8;

// --- [routing] -------------------------------------------------------

/// `routing.policy` default.
pub const DEFAULT_ROUTING_POLICY: &str = "fixed";

/// `routing.tiered.classify_threshold_ms` default.
pub const DEFAULT_CLASSIFY_THRESHOLD_MS: u32 = 80;

// --- [judge] ---------------------------------------------------------

/// `judge.enabled` default.
pub const DEFAULT_JUDGE_ENABLED: bool = true;

/// `judge.rubric` default.
pub const DEFAULT_JUDGE_RUBRIC: &str = "default";

/// `judge.verdict_on_error` default.
pub const DEFAULT_JUDGE_VERDICT_ON_ERROR: &str = "partial";

// --- [forge] ---------------------------------------------------------

/// `forge.default_examples_min` default.
pub const DEFAULT_FORGE_EXAMPLES_MIN: u32 = 3;

/// `forge.default_iterations` default.
pub const DEFAULT_FORGE_ITERATIONS: u32 = 12;

/// `forge.budget_per_run_usd_micros` default ($5).
pub const DEFAULT_FORGE_BUDGET_PER_RUN_USD_MICROS: u64 = 5_000_000;

/// `forge.auto_promote_threshold` default.
pub const DEFAULT_FORGE_AUTO_PROMOTE_THRESHOLD: f32 = 0.92;

// --- [gateway] -------------------------------------------------------

/// `gateway.host` default — bind address.
pub const DEFAULT_GATEWAY_HOST: &str = "127.0.0.1";

/// `gateway.port` default.
pub const DEFAULT_GATEWAY_PORT: u16 = 8787;

/// `gateway.url` default — used by clients (e.g. TUI) to talk to a
/// locally running gateway.
pub const DEFAULT_GATEWAY_URL: &str = "http://127.0.0.1:8787";

/// `gateway.duty_scheduler_secs` default — orchestrator duty
/// scheduler tick interval.
pub const DEFAULT_DUTY_SCHEDULER_SECS: u64 = 60;

/// `gateway.handoff_sweep_secs` default — handoff walker sweep
/// interval.
pub const DEFAULT_HANDOFF_SWEEP_SECS: u64 = 60;

// --- [loop] ----------------------------------------------------------

/// `loop.on_restart` default.
pub const DEFAULT_ON_RESTART: &str = "prompt_user";
