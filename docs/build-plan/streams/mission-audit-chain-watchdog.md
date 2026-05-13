# Mission load-bearing wire — Audit-chain tamper watchdog

**Owner**: claude. **Depends on**: existing `argo-audit::chain` merkle/hash-chain primitive (verify via `rg -l 'verify_chain\|chain_hash\|prev_hash' crates/argo-audit/src/`), existing notification channels (`argo-channels`).
**Authored against**: origin/main `bd7fd450` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-chain-watchdog`.

**DEEP-DRIFT — CONDUCTOR REWRITE NEEDED**: `crates/argo-audit/src/watchdog.rs` ALREADY EXISTS (222 LOC, MVP audit-tamper-watchdog from `mvp-audit-tamper-watchdog` MERGED). It exposes `spawn_watchdog(store, WatchdogConfig, on_break)`, `verify_all` integration, consecutive-failure counter, structured `tracing::error!` on streak. This brief is **superseded by what already shipped**. Confirm: `rg "spawn_watchdog\|WatchdogConfig" crates/argo-audit/src/`. Open follow-ups (if needed) are scoped extensions, not a NEW module:  (a) channel-fan-out `TamperNotifier` impl (still missing), (b) `argo audit verify [--full] [--from] [--limit]` CLI (still missing), (c) persistent `audit_watchdog_cursor` table (still missing — currently in-memory). Conductor should rewrite this brief as 3 narrow follow-ups against the existing `watchdog.rs` rather than a NEW module.

> SOURCE: MISSION_BACKLOG.md Section 1 #37 ("Add audit-chain watchdog that periodically verifies and pages on tamper"). **Tier M (compliance / governance).** The audit chain is a hash-linked sequence: every row's `chain_hash = sha256(prev_chain_hash || row_canonical_bytes)`. If a row gets edited or deleted, the chain breaks at that row — every downstream `chain_hash` is now wrong. Today nobody checks. This brief adds a background watchdog that walks the chain periodically + on-demand, pages on break, and emits a structured tamper event.

## Scope

NEW module `crates/argo-audit/src/watchdog.rs` (≤ 280 LOC):

```rust
pub struct WatchdogConfig {
    pub enabled: bool,
    pub interval: Duration,           // default 1h
    pub batch_size: usize,            // verify N rows per tick (default 1000)
    pub start_from_last_verified: bool, // resume from cursor instead of full re-scan
}

pub struct VerifyReport {
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub rows_checked: u64,
    pub first_break_seq: Option<u64>,     // None = chain intact
    pub first_break_expected_hash: Option<[u8; 32]>,
    pub first_break_observed_hash: Option<[u8; 32]>,
}

pub trait TamperNotifier: Send + Sync {
    async fn notify(&self, report: &VerifyReport) -> ArgoResult<()>;
}

pub struct AuditChainWatchdog<N: TamperNotifier> {
    chain: Arc<dyn AuditChain>,        // existing trait that exposes scan(from, n)
    notifier: Arc<N>,
    config: WatchdogConfig,
    cursor: AtomicU64,                 // last-verified seq
}
impl<N: TamperNotifier> AuditChainWatchdog<N> {
    pub fn new(chain: Arc<dyn AuditChain>, notifier: Arc<N>, config: WatchdogConfig) -> Self;
    pub async fn run_once(&self) -> ArgoResult<VerifyReport>;  // single tick
    pub async fn run_forever(self) -> !;                       // periodic loop
}
```

Wired with one concrete `TamperNotifier` impl (`crates/argo-audit/src/tamper_notifier_channels.rs`, ≤ 150 LOC) that fans out via the just-merged operator-preferences-style approval channel set: emit a `tracing::error!` event tagged `metric = "audit_chain_tamper_detected"` AND send an URGENT-priority message through every channel the operator configured (mirrors the existing `mission-mcp-connector-pagerduty` shape — verify exact channel-dispatch surface).

Storage cursor primitive `crates/argo-store/src/audit_watchdog.rs` (≤ 120 LOC):

```rust
pub struct AuditWatchdogCursor { pub last_verified_seq: u64, pub last_run_at: DateTime<Utc> }
pub async fn load_cursor(pool: &SqlitePool) -> ArgoResult<AuditWatchdogCursor>;
pub async fn save_cursor(pool: &SqlitePool, cursor: &AuditWatchdogCursor) -> ArgoResult<()>;
```

NEW migration vNN.rs (verify next-free slot — as of 2026-05-12 the live tail is v58 / SQL 62 after v57 routing-skillmatch (#1356) and v58 operator-risk-overrides (#1361) shipped; pick v59+ / SQL slot 63+, coordinating w/ parallel `mission-gateway-request-audit-log` brief via ordering) adding `audit_watchdog_cursor` single-row table.

NEW CLI surface (`crates/argo-cli/src/handlers/audit_watchdog.rs` ≤ 160 LOC):
- `argo audit verify [--full] [--from <seq>] [--limit <n>] [--json]` — manual one-shot verify; `--full` resets cursor and walks from seq=0
- `argo audit watchdog status [--json]` — last cursor + last report

Boot hook: when `[audit.watchdog].enabled = true`, the gateway boot path spawns the watchdog as a `tokio::spawn(run_forever())` task.

This is NOT in scope: actual chain-repair tooling (separate forensics brief — tamper response is HUMAN, not automated); per-tenant chain isolation (the chain is global today; per-tenant is a separate redesign); WORM storage backend (separate); off-host hash-attestation (separate).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-audit/src/watchdog.rs` (NEW, ≤ 280 LOC)
- `crates/argo-audit/src/watchdog_tests.rs` (NEW, sibling, ≤ 280 LOC) — 9 unit tests (intact-chain / break-at-mid / break-at-last / empty-chain / batch-size-cap / cursor-resume / notifier-error-doesn't-crash-loop / tamper-event-shape / VerifyReport-roundtrip-through-JSON)
- `crates/argo-audit/src/tamper_notifier_channels.rs` (NEW, ≤ 150 LOC) — concrete TamperNotifier hitting `argo-channels`
- `crates/argo-audit/src/lib.rs` — narrowly scoped: register modules
- `crates/argo-audit/src/sink.rs` (EXTEND, ≤ 30 LOC delta if needed) — add `pub async fn scan(from: u64, n: usize) -> Vec<AuditRow>` if it doesn't exist yet (low risk — pure read). NOTE: there is no `crates/argo-audit/src/chain.rs` on origin/main 2026-05-12; chain-hash logic lives in `sink.rs::compute_chain_hash` + `verify.rs::verify_session`/`verify_all`.
- `crates/argo-store/src/audit_watchdog.rs` (NEW, ≤ 120 LOC)
- `crates/argo-store/src/audit_watchdog_tests.rs` (NEW, sibling, ≤ 100 LOC) — 4 unit tests
- `crates/argo-store/src/lib.rs` — narrowly scoped: register module
- `crates/argo-store/src/migrations/vNN.rs` (NEW) — verify next-free slot
- `crates/argo-store/migrations/<NN>__audit_watchdog_cursor.sql` (NEW, ≤ 20 LOC)
- `crates/argo-store/src/migrations/mod.rs` — narrowly scoped: register
- `crates/argo-config/src/sections.rs` — narrowly scoped: ADD `[audit.watchdog]` section
- `crates/argo-config/src/defaults.rs` — narrowly scoped
- `crates/argo-cli/src/handlers/audit_watchdog.rs` (NEW, ≤ 160 LOC)
- `crates/argo-cli/src/cli_subcommands_audit.rs` — narrowly scoped: ADD `Verify` + `Watchdog::Status` arms to `AuditCmd`
- `crates/argo-cli/src/handlers/mod.rs` — narrowly scoped: register handler
- `crates/argo-cli/src/handlers/gateway/boot.rs` — narrowly scoped: ADD watchdog spawn-on-boot hook (≤ 15 LOC delta) gated on `[audit.watchdog].enabled = true`
- `crates/argo-cli/tests/audit_watchdog_smoke.rs` (NEW, ≤ 200 LOC) — boot gateway, `argo audit verify --full` → intact, tamper one row directly via SQL, re-run, assert break detected + notifier fired

## Files you must NOT touch

- Existing audit row schema (chain_hash / prev_hash columns) — this brief is read-only against the chain
- `crates/argo-channels/src/{slack,email,sms,voice,whatsapp,discord}.rs` — the TamperNotifier dispatches through the existing channel surface, doesn't change it
- Any other crate not listed above

## Acceptance criteria

- [ ] `argo audit verify --full --json` returns `{"rows_checked": N, "first_break_seq": null}` on intact chain
- [ ] After SQL-tampering with one row, `argo audit verify --full --json` reports `first_break_seq = <seq>` + correct expected/observed hashes
- [ ] When tamper detected, exactly ONE `tracing::error!` event tagged `metric = "audit_chain_tamper_detected"` fires, AND the configured channels each receive one URGENT-priority message
- [ ] `[audit.watchdog].enabled = false` → no background loop spawn, no notifier overhead
- [ ] Cursor persists across boot — second boot resumes from `last_verified_seq` instead of full re-scan
- [ ] Notifier error (e.g. Slack 500) does NOT crash the watchdog loop — it logs and continues; verified by a unit test
- [ ] `argo audit verify` exits 0 on intact, exit code 1 on break (operator-runnable from cron / k8s liveness)
- [ ] 9 unit tests on watchdog + 4 on store + 1 integration smoke
- [ ] `cargo clippy -p argo-audit -p argo-store -p argo-config -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-audit -p argo-store -p argo-config -p argo-cli` passes
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-audit-chain-watchdog`
- PR title: `audit: chain-tamper watchdog + verify CLI + channel notification (mission-audit-chain-watchdog)`

## Notes for the implementer

- First read `crates/argo-audit/src/sink.rs` (specifically `compute_chain_hash` near line 196) AND `crates/argo-audit/src/verify.rs` to confirm the chain primitive shape. There is NO `chain.rs` file — earlier draft of this brief had a typo. The brief assumes `chain_hash = sha256(prev_chain_hash || row_canonical_bytes)` — if the actual encoding is different (e.g. blake3, or different canonicalization), match the existing scheme exactly. Don't invent a new format.
- `run_forever()` is a `loop { run_once().await; tokio::time::sleep(interval).await; }` — interruptible by drop. Don't add custom shutdown signaling.
- Batch size: verify rows in slices of `config.batch_size` (default 1000) so a watchdog tick on a 10M-row chain doesn't lock the read pool for minutes. Cursor advances per batch.
- Notifier-error-doesn't-crash: wrap the notify call in `if let Err(e) = self.notifier.notify(&report).await { tracing::error!(error=?e, "tamper notifier failed"); }`. The loop continues regardless. The `tracing::error!` event is the fallback signal — even if all channels are broken, the operator still has the structured log.
- Cursor write happens BEFORE notifier dispatch — if notification fails, the cursor still moves forward, otherwise a stuck notifier would prevent progress.
- Tamper-test approach: write a row, then `UPDATE audit_rows SET row_canonical = 'X' WHERE seq = ?` to corrupt one row; the watchdog should detect that `sha256(prev_hash || 'X')` != stored `chain_hash`.
- Migration slot: as of 2026-05-12 the live tail is v58 / raw slot 62 (v57 routing-skillmatch + v58 operator-risk-overrides both shipped). Coordinate with the parallel `mission-gateway-request-audit-log` brief — pick whichever of v59/v60 (and SQL 63/64) the other doesn't claim. Verify with `ls crates/argo-store/migrations/ | sort -V | tail -5` at claim time and pick the next-free slot if those numbers have shifted further.
- Operator escalation contract: this watchdog is the FIRST signal of audit tampering. The PR body should document explicitly: "When this fires, treat as P0 — the audit chain is the security-of-record source. Do not auto-repair. Forensics first."
