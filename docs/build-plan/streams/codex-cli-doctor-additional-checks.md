# codex-cli-doctor-additional-checks — extend `argo doctor` with 4 more health probes (P2)

**Owner**: codex. **Sizing hint**: judgment + integration. **Depends on**: `mission-argo-doctor-network-check` MERGED (#1788).
**Authored against**: origin/main `c2d3417d` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-cli-doctor-additional-checks`.
**Source**: post-v1 polish — `argo doctor` is the operator's first port of call. #1788 added network checks; this brief adds 4 more probes that map to common production-incident triage questions.

## Mission alignment

- **Observability** (CLAUDE.md priority #2) — single command surfaces all common failure modes.
- **Reliability** (#1) — doctor is the operator's diagnostic surface.

## Scope

P2. Four new health probes:

### 1. Audit-chain integrity

Run `argo audit verify-chain --json` (already exists) against the most-recently-active session. Report:
- ✅ chain verifiable, head_seq=N
- ⚠️  chain has gaps (rare; can happen post-restore)
- ❌ chain break detected at seq=X — escalate

### 2. Channel reachability

For each configured channel in `argo-config::channels`:
- Slack: ping `https://slack.com/api/api.test` with operator's bot token; expect `ok: true`.
- Email (SendGrid/Postmark): GET `/v3/api_keys/{id}` or equivalent.
- Discord: GET `/api/v10/gateway`.
- WhatsApp/Meta: GET `/v18.0/me` with the configured access token.

Each channel: ✅ reachable / ⚠️  not configured (SKIP) / ❌ unreachable + reason.

### 3. Storage capacity

- Disk usage on `~/.argo/` (where DB + audit + memory live).
- Report: total bytes / free bytes / used %.
- Warn at >80% used; FAIL at >95%.
- `audit_record` table size + row count.
- `memory_row` table size + row count.
- Suggested operator action (`argo gdpr delete`, `argo audit gc`) if thresholds exceeded.

### 4. Stale-worker check

- Query `worker` table: count workers with `last_active_at < now - 24h` and `status = 'active'`.
- Report: ✅ 0 stale / ⚠️ N stale (suggests they haven't run in a day; check duty scheduler).

LOC: ~250.

## Files you may touch

- `crates/argo-cli/src/handlers/doctor.rs` — extend with 4 new sections. ≤80 modified LOC.
- `crates/argo-cli/src/handlers/doctor_audit_chain.rs` (NEW) — reuse `argo-audit::verify` API. ≤50 LOC.
- `crates/argo-cli/src/handlers/doctor_channels.rs` (NEW) — per-channel reachability. ≤120 LOC.
- `crates/argo-cli/src/handlers/doctor_storage.rs` (NEW) — disk + table size. ≤80 LOC.
- `crates/argo-cli/src/handlers/doctor_workers.rs` (NEW) — worker staleness query. ≤50 LOC.
- `crates/argo-cli/src/handlers/doctor_tests.rs` — extend with 4 new test cases (each new probe). ≤80 modified LOC.

## Files you must NOT touch

- `argo-audit::verify` — read-only consumer.
- Channel client crates — use existing `argo-channels` API surface (probe via existing method, not new HTTP).
- Worker / duty scheduler infrastructure — read-only query.
- Network-check module from #1788 — that lands; doctor handler calls it; this brief adds AFTER that.

## Acceptance criteria

- [ ] `argo doctor` shows 4 new sections (audit-chain / channels / storage / stale-workers) after the existing health checks.
- [ ] `argo doctor --json` includes 4 new fields: `audit_chain`, `channels`, `storage`, `workers`.
- [ ] `argo doctor --skip-network` skips channels + audit-chain integrity (offline operator OK).
- [ ] Each probe FAILs (exit 2) on hard error, WARNs (exit 1) on yellow, OKs on green.
- [ ] Tests stub real channel + audit + DB I/O (offline-friendly).
- [ ] `cargo test -p argo-cli doctor` passes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-cli-doctor-additional-checks`
- Title: `feat(doctor): audit-chain + channel + storage + worker-staleness probes (codex-cli-doctor-additional-checks)`
- Worker verification: crate-scoped + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Channel probes use the operator's configured channel.** If `discord` not configured → SKIP, not FAIL.
- **Audit-chain probe is heavy** for large databases — cap at the most-recently-active session, not full history.
- **Storage thresholds operator-configurable** via `[doctor.storage_warn_pct]` config block; defaults 80/95.
- **Stale-worker threshold** — 24h is sensible default; configurable via `[doctor.worker_stale_hours]`.
- **Don't emit OK on configured-but-untested.** A channel is OK only if the probe succeeds; configured-but-untested is YELLOW.
- **No real network in tests.** Stub `reqwest::Client` for the channel-probe tests.
