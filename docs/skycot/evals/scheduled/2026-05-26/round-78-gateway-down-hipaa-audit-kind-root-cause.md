# Round 78 — Live gateway DOWN — root cause traced to unregistered audit-kind label

- **Date**: 2026-05-26
- **Round**: 78 (scheduled-task hourly fire ~03:09Z)
- **Pillar / Claim**: Reliability (priority 1) + Observability (priority 2) — live gateway is the substrate every SPA / curl / e2e probe routes through
- **Verdict**: 🔴 **FAIL — P0**
- **Anti-theater stance**: this is a real probe against the live gateway, not a test against a stubbed mock

## Setup

```
curl -sf -m 5 http://127.0.0.1:7400/health    # exit 7 (failed to connect)
curl -sf -m 5 http://127.0.0.1:7400/readyz    # exit 7
bash scripts/check-gateway-freshness.sh        # STATUS: UNREACHABLE
lsof -i :7400                                  # empty (no listener)
pgrep -lf "target/release/argo|argo gateway"   # empty (no process)
ls /tmp/argo-*.pid /tmp/gateway*.pid           # not found
```

Last gateway log on disk: `~/.argo/logs/gateway-20260526T094927.log` — started 09:49 AEST on 2026-05-26 (= 23:49Z 2026-05-25 UTC), file mtime `May 26 12:39` AEST (= 2026-05-26T02:39Z UTC).

## Observed

`tail ~/.argo/logs/gateway-20260526T094927.log`:

```
ERROR audit::watchdog audit chain tamper alarm: consecutive failure threshold reached
  consecutive_failures=33 threshold=3 break_count=18
ERROR gateway::audit_watchdog audit chain break detected
  seq=1 kind=hipaa_compliance_check reason=unknown audit kind label
ERROR gateway::audit_watchdog audit chain break detected
  seq=3 kind=hipaa_compliance_check reason=unknown audit kind label
...
ERROR audit::watchdog audit chain tamper alarm: consecutive failure threshold reached
  consecutive_failures=34 threshold=3 break_count=18
```

Every `hipaa_compliance_check` row is flagged by the watchdog with `reason=unknown audit kind label`. With each gateway restart that audit row is re-emitted, watchdog count climbs past `threshold=3`, tamper alarm fires repeatedly, and the gateway eventually stops servicing requests.

### Root cause walked end-to-end

1. **Producer site** — `crates/argo-cli/src/handlers/gateway/hipaa_startup_check.rs:13`:

   ```rust
   const KIND: &str = "hipaa_compliance_check";
   ```

   Called from `emit_startup_check_best_effort()` at every gateway boot. Writes one audit row whose `kind` column = `"hipaa_compliance_check"`.

2. **Consumer site** — `crates/argo-audit/src/event.rs:97` (`AuditKind::from_label`). Grep `hipaa` against `crates/argo-audit/src/event.rs` returns **zero matches**. The enum has labels for `turn_start`, `tool_call`, `approval.granted`, etc. but no `hipaa_compliance_check`.

3. **Watchdog reject** — `crates/argo-audit/src/verify.rs:184-191`:

   ```rust
   if AuditKind::from_label(&row.kind).is_none() {
       report.ok = false;
       report.breaks.push(ChainBreak {
           seq: row.seq,
           kind: row.kind.clone(),
           reason: "unknown audit kind label".into(),
       });
   }
   ```

   Hands the `unknown audit kind label` ChainBreak back to the live `audit_watchdog`, which trips `consecutive_failure_threshold=3` and fires `tamper alarm` forever.

### Reconciliation against the conductor T03 tick

The conductor T03 tick at 02:58:00Z reported `gateway HTTP 200 schema v83=v83 FRESH drift=0`. Now (03:09:35Z) gateway is UNREACHABLE. Two possibilities:

- **Real interval death** — gateway died between 02:58Z and 03:09Z (~11min). Consistent with the watchdog hitting its alarm threshold mid-window.
- **Ghost tick** — T03 reported state from a cached or earlier probe without re-curling. This is the exact pattern `ops-conductor-ghost-stamped-tick-detection` (PR #2338) was authored to detect.

Either way, the next conductor tick needs to (a) actually probe `/health` and (b) restart if down.

### Existing brief misses the root cause

`docs/build-plan/streams/fix-audit-unknown-worker-and-hipaa-fixture-rows.md` (BLOCKED per `5d899a9ac`) describes the same `kind=hipaa_compliance_check` symptom but diagnoses it as "stray test fixtures" that "landed in the demo tenant". The brief is wrong about the source — these rows are **production startup-emits**, written by every gateway boot at `hipaa_startup_check.rs:13`. The brief's three real concerns (empty `worker_id`, raw verb in SPA, hipaa rows on marketing-intern tenant) are still valid but downstream of the registry-gap fix.

## Anti-theater check

- Reading the literal log file on disk (not a mocked tail).
- Grepping the production source code in `crates/argo-audit/src/event.rs` for `hipaa` (zero matches) and in `crates/argo-cli/src/handlers/gateway/hipaa_startup_check.rs` for the literal kind string (one match). Production code paths are open in this report.
- Re-ran `curl` to the gateway twice (initial probe + retry) — exit 7 both times.

## Gap / follow-up brief candidates

### P0 NEW — `fix-audit-kind-register-hipaa-compliance-check`

- **Owner**: claude or codex (~10-LOC, fast-mode candidate)
- **Scope**: `crates/argo-audit/src/event.rs` only — add `HipaaComplianceCheck` variant, add label arm in `from_label()`, add label arm in `as_label()`. Test pin: `AuditKind::from_label("hipaa_compliance_check").is_some()`. Re-emit and verify watchdog stays quiet.
- **Acceptance**: `cargo test -p argo-audit hipaa` PASS; live gateway restart followed by 5-minute observation shows zero `unknown audit kind label` log lines; `/health` returns 200.
- **Authorization fit**: falls under overnight ops-* `--skip-clippy` window (expires 22:00Z AEST 2026-05-26).

### P0 OPERATOR — restart gateway NOW so demos / SPA work

```
bash scripts/restart-live-gateway.sh --no-prompt --force
```

Does NOT fix root cause — watchdog will re-fire on next startup-check emit — but restores `/health` for the next 5-30 min until cumulative failures climb again. The kind-registration brief above is the durable fix.

### P1 UPDATE — `fix-audit-unknown-worker-and-hipaa-fixture-rows`

- **Action**: UNBLOCK after the kind-registration fix lands.
- **Re-scope**: drop the "stray test fixtures" framing; keep the three legitimate concerns (empty `worker_id` rendering as "UN" in `ActivityRow.tsx`, raw `hipaa_compliance_check` as verb in SPA, hipaa rows showing on marketing-intern tenant when the workflow is unrelated to healthcare).

### P3 OBSERVABILITY — re-validate the ghost-tick hypothesis

Watch the next conductor tick's gateway-freshness probe output. If T04 tick reports `gateway HTTP 200` while a parallel `curl` from the operator confirms `/health` 503, prioritize merging PR #2338 (`ops-conductor-ghost-stamped-tick-detection`) above other backlog.
