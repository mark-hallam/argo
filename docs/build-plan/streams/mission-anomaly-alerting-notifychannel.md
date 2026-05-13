# Mission load-bearing wire — Anomaly alerting via NotifyChannel

**Owner**: claude. **Depends on**: existing `argo-channels` dispatch surface (verify via `rg -l 'pub fn dispatch\|NotifyChannel\|ChannelKind' crates/argo-channels/src/`); existing `argo-telemetry` event surface for metrics.
**Authored against**: origin/main `694a50ee` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-anomaly-alerting-notifychannel`.

**DEEP-DRIFT — CONDUCTOR REWRITE NEEDED**: There is no `NotifyChannelDispatch` on current main; live `argo-channels::NotifyChannel` sends approval-shaped `Notification`s, and the only multi-channel dispatcher is CLI-private `worker_actions_notify::EnvChannelSender` / `NotifySender`, both outside may-touch. The tracing subscriber is built in `crates/argo-cli/src/tracing_setup.rs`, NOT `argo-telemetry::init_subscriber`, so `[alerting].enabled=false` cannot provide the promised zero-overhead registration path without widening may-touch. Conductor rewrite should either expose a reusable alert dispatch adapter in `argo-channels`, OR include `worker_actions_notify*` + `tracing_setup.rs` in scope.

> SOURCE: MISSION_BACKLOG.md Section 1 #35 ("Add real anomaly alerting via NotifyChannel (currently `tracing::warn!` only)"). **Tier M (observability).** Today when bad things happen (provider 5xx storm, budget-ceiling hit, audit-chain break, gateway boot fail) the runtime emits `tracing::warn!` / `tracing::error!`. Operators staring at logs see them; operators NOT staring at logs (most of the time) miss them. This brief wires the existing `argo-channels::NotifyChannel` dispatch into an `anomaly_alerter` so configured conditions page through Slack/PagerDuty/Email/Voice etc.

## Scope

NEW `crates/argo-runtime/src/anomaly_alerter.rs` (≤ 280 LOC) — or wherever the runtime's main async-coordination lives; verify via `ls crates/argo-runtime/src/`:

```rust
pub struct AnomalyAlerter {
    rules: Vec<AnomalyRule>,
    channels: Arc<dyn NotifyChannelDispatch>, // existing argo-channels surface
    state: Mutex<AlerterState>,
}

#[derive(Debug, Clone)]
pub struct AnomalyRule {
    pub name: String,                  // human-readable, included in alerts
    pub trigger: AnomalyTrigger,
    pub channels: Vec<ChannelKind>,    // route via these channels
    pub min_severity: Severity,        // Warn | Critical
    pub debounce: Duration,            // don't re-fire within this window
}

#[derive(Debug, Clone)]
pub enum AnomalyTrigger {
    /// >= count occurrences of `metric` in `window` minutes
    MetricThreshold { metric: String, count: u32, window: Duration },
    /// >= rate (fraction 0..1) of `metric_num`/`metric_denom`
    ErrorRate { numerator: String, denominator: String, rate: f64, window: Duration },
    /// any structured-tracing event matching `event_filter`
    EventMatches { event_filter: EventFilter }, // by target + level + field-eq pairs
}

pub enum Severity { Warn, Critical }

impl AnomalyAlerter {
    pub fn new(rules: Vec<AnomalyRule>, channels: Arc<dyn NotifyChannelDispatch>) -> Self;
    pub async fn ingest(&self, event: TracingEventSummary) -> ArgoResult<()>;
    pub async fn tick(&self, now: DateTime<Utc>) -> ArgoResult<Vec<AlertSent>>;
}
```

NEW tracing-subscriber Layer in `argo-telemetry::anomaly_capture` that captures every structured event w/ a `metric = "..."` field (existing convention used by gateway_rate_limit_throttle_count, audit_chain_tamper_detected, provider_call_total, etc.) and forwards to the AnomalyAlerter. The Layer is a true zero-overhead path when `[alerting].enabled = false`.

NEW `[alerting]` config block (`crates/argo-config/src/sections.rs`):
```toml
[alerting]
enabled = true
default_channels = ["slack", "email"]

[[alerting.rules]]
name = "provider_5xx_storm"
trigger = { kind = "metric_threshold", metric = "provider_call_total", count = 50, window = "1m" }
min_severity = "warn"
debounce = "5m"
channels = ["slack"]

[[alerting.rules]]
name = "audit_chain_break"
trigger = { kind = "event_matches", target = "argo_audit", level = "error", field = { metric = "audit_chain_tamper_detected" } }
min_severity = "critical"
debounce = "1h"
channels = ["slack", "pagerduty", "voice"]

[[alerting.rules]]
name = "budget_ceiling_hit"
trigger = { kind = "event_matches", field = { metric = "budget_ceiling_exceeded" } }
min_severity = "warn"
debounce = "10m"
channels = ["slack", "email"]
```

NEW CLI surface (`crates/argo-cli/src/handlers/alerting.rs` ≤ 160 LOC):
- `argo alerting rules` — list configured rules
- `argo alerting test <rule-name> [--severity warn|critical]` — fire one synthetic alert through the configured channels (operator confidence-check)
- `argo alerting history [--since <when>] [--limit <n>] [--json]` — recent alerts sent (in-memory ring buffer in v1, 1000-entry cap)

This is NOT in scope: PagerDuty incidents API integration (separate brief once PagerDuty connector lands); per-tenant alerting rules (rules are tenant-global in v1); operator on-call rotation schedules (separate); alert-acknowledgement workflow / inhibitions / suppression windows; alert-correlation (multiple firing rules → one super-alert); metric-storage beyond the rolling-window in-memory tracker; the actual PagerDuty/Slack/Voice channel impls (already exist in `argo-channels`).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-runtime/src/anomaly_alerter.rs` (NEW, ≤ 280 LOC)
- `crates/argo-runtime/src/anomaly_alerter_tests.rs` (NEW, sibling, ≤ 300 LOC) — 10 unit tests:
  - metric-threshold-fires-after-count-in-window
  - metric-threshold-does-not-fire-before-count
  - metric-threshold-window-expires-resets-counter
  - error-rate-fires-when-num-div-denom-exceeds-rate
  - event-matches-by-target-level-field-eq
  - debounce-suppresses-repeat-fire-within-window
  - debounce-clears-after-window-elapse
  - critical-severity-fires-even-when-warn-rules-debounced
  - notifier-error-doesnt-crash-alerter
  - history-ring-buffer-caps-at-1000
- `crates/argo-runtime/src/lib.rs` — narrowly scoped: register module + re-exports
- `crates/argo-telemetry/src/anomaly_capture.rs` (NEW, ≤ 200 LOC) — `AnomalyCaptureLayer` tracing-subscriber layer
- `crates/argo-telemetry/src/anomaly_capture_tests.rs` (NEW, sibling, ≤ 150 LOC) — 5 unit tests
- `crates/argo-telemetry/src/lib.rs` — narrowly scoped: register module + install in subscriber when enabled
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[alerting]` block + Vec<AlertingRule> typed parse (≤ 80 LOC)
- `crates/argo-config/src/defaults.rs` — narrowly scoped
- `crates/argo-config/src/alerting_tests.rs` (NEW, sibling, ≤ 120 LOC) — 4 unit tests on TOML roundtrip + default-when-section-missing
- `crates/argo-cli/src/handlers/alerting.rs` (NEW, ≤ 160 LOC)
- `crates/argo-cli/src/cli_subcommands.rs` — narrowly scoped: ADD `Alerting` arm w/ `Rules` + `Test` + `History` (≤ 25 LOC delta)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: register
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: instantiate `AnomalyAlerter` from config + register `AnomalyCaptureLayer` when `[alerting].enabled = true` (≤ 25 LOC delta)
- `crates/argo-cli/tests/anomaly_alerting_smoke.rs` (NEW, ≤ 200 LOC) — integration smoke: configure 1 rule (event_matches on a test metric), boot gateway, emit 1 matching event (via test-only endpoint or direct tracing call), assert mock-channel received exactly 1 alert

## Files you must NOT touch

- `crates/argo-channels/src/{slack,email,sms,voice,whatsapp,discord,pagerduty}.rs` — channel-specific impls; this brief dispatches THROUGH the existing surface, doesn't change it
- `crates/argo-store/migrations/**` — no schema (in-memory history)
- Vertical content / frontend

## Acceptance criteria

- [ ] `argo alerting rules` lists the rules configured in `[alerting]`
- [ ] `argo alerting test <rule>` fires one synthetic alert through the configured channels; mock channel verifies receipt
- [ ] Provider-5xx-storm rule: emit 50 `provider_call_total` events with `status=500` within 1m → exactly 1 alert sent; emit 49 → 0 alerts
- [ ] Audit-chain-break rule: emit `audit_chain_tamper_detected` once → exactly 1 alert sent (severity=critical, all 3 channels reached)
- [ ] Debounce works: same rule firing twice within `debounce` window → 1 alert sent; outside window → 2 alerts
- [ ] Notifier error (e.g. slack 500) does NOT crash the alerter loop — logs `tracing::error!` and continues; verified via unit test
- [ ] `[alerting].enabled = false` → Layer not installed, true zero-overhead path
- [ ] 10 unit tests on alerter + 5 on capture layer + 4 on config + 1 integration smoke
- [ ] `cargo clippy -p argo-runtime -p argo-telemetry -p argo-config -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-runtime -p argo-telemetry -p argo-config -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-anomaly-alerting-notifychannel`
- PR title: `runtime: anomaly alerter dispatching configured rules through argo-channels (mission-anomaly-alerting-notifychannel)`

## Notes for the implementer

- First verify the existing `NotifyChannel` trait shape via `rg -nA 20 'pub trait NotifyChannel' crates/argo-channels/src/`. The brief assumes a `dispatch(&self, channel: ChannelKind, msg: &AlertMessage) -> ArgoResult<()>` shape — match the actual signature exactly.
- The tracing capture layer pattern is the same as the just-authored `mission-structured-logging-tenant-worker-session` brief (`ContextInjectorLayer`). Mirror that structure: register below the formatter, walk fields of every event, forward those with `metric = "..."` to the alerter.
- Rolling-window counters: use a `VecDeque<(DateTime<Utc>, ())>` per metric; on `ingest`, push + truncate-front-by-time. Don't use a fancier RingBuffer crate — the cardinality is bounded by ≤ ~20 rules × ~5 metrics each.
- The 3 example rules in the brief are LOAD-BEARING — they should ship in the default `argo.toml` so a fresh install gets reasonable signal-on-bad-things without operator configuration. Document this as "default-on for production, default-off via `enabled = false` for dev".
- `argo alerting test <rule>` is intentionally for OPERATOR CONFIDENCE — they configure a rule, hit `argo alerting test`, see the alert land in Slack, sleep better. The synthetic alert body should explicitly say `[TEST]` so the on-call doesn't panic.
- The history ring buffer is intentionally tiny (1000 entries) — it's a CLI convenience for "what did we page about today". For real long-term alert auditing, ship to Loki/Datadog via the existing log shipping path (separate brief).
- Critical-severity-fires-even-when-warn-debounced: there's a subtle priority pattern here. If `warn` rules are debounced AND a `critical` rule fires for the same underlying condition, the critical one MUST go through (different debounce key — `(rule_name, severity)` not just `rule_name`). Test this case explicitly.
- DO NOT add a metric storage layer. The alerter is reactive (rolling windows in memory). Long-term storage is the metrics-exporter brief (`mission-prometheus-exporter-default-registry`, in flight).
