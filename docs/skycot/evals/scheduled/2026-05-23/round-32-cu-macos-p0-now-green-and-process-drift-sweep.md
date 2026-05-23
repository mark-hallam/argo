# Round 32 — Cu-macos P0 blocker now GREEN + process drift sweep

**Date**: 2026-05-23
**Round**: 32 (scheduled-task hourly, read-only)
**Pillar/Claim**: Operational — round-30/31 P0 closure + ongoing process gate health
**Anchor**: `1706d4eeaa7012eab44656e46aac4031e262d0f0` (`origin/main` at session start)

## Verdict

🟢🟢 **CLOSED — round-30/31 P0 RESOLVED on main.** `cargo check -p argo-executor-computer-use-macos --tests` now exits 0 on origin/main. The 2-line additive fix (tenant_id + worker_id on `MacosCuRunRequest` at `permission_revoke_mid_run_test.rs:140-141`) shipped as collateral inside #2112's eval-claim-5 PR.

🟡 **PARTIAL — process drift**: 1 P1 gap (the round-30 fix-by-tracker miss / round-31 reframe) is still uncovered by tooling — but the brief acknowledging it (`ops-add-todo-aging-gate-script`) is in the queue per round-31's reframe. Less load-bearing now that the specific P0 it was supposed to catch is closed.

## Setup

```bash
# (a) cu-macos workspace-build re-verify
cargo check -p argo-executor-computer-use-macos --tests

# (b) process drift sweep
bash scripts/check-unread-handoffs.sh
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-gateway-freshness.sh
```

## Observed

### (a) cu-macos check

```
[...]
Checking argo-executor-computer-use v0.0.1 (~/code/argo/crates/argo-executor-computer-use)
Checking argo-executor-computer-use-macos v0.0.1 (~/code/argo/crates/argo-executor-computer-use-macos)
Finished `dev` profile [optimized + debuginfo] target(s) in 22m 12s
```

Exit code 0, no errors. **The same `E0063 missing fields tenant_id, worker_id` build error that round 30 + round 31 verified RED is now compiled clean on main.** Per scenario 1's verification, the additive fix is at [permission_revoke_mid_run_test.rs:140-141](../../../../crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs#L140):

```rust
tenant_id: argo_types::TenantId::new(),
worker_id: argo_identity::WorkerId::new(),
```

This is the exact 2-line additive fix the round-26 evaluator finding flagged, that the round-31 evaluator confirmed as "WIP diff already in working tree at session start" — shipped as collateral with #2112 by the brief author rather than waiting for `fix-cu-macos-permission-revoke-test-add-tenant-worker-fields` to ship independently.

Conductor tick-102's SHARED note explicitly called this out: "also shipped the cu-macos P0 tenant_id+worker_id fix as collateral, unblocking workspace clippy". Verified: yes, the workspace clippy gate (gate-5 of `pre-merge-gate`) is no longer blocked by this. Future `pre-merge-gate` runs should not need `--skip-clippy` for the cu-macos test bug reason.

### (b) check-unread-handoffs.sh

Reports **26 unread handoffs** in SHARED.md newer than cutoff `2026-05-23T09:11:00Z`. These are the historical evaluator round-9 through round-30 entries that the conductor's tick-101 cleared via the heartbeat-cutoff bump at commit `494ac4cc2` ("conductor heartbeat — bump cutoff to clear Melbourne-mislabeled SHARED handoffs"). The cutoff appears not to have been bumped past the Melbourne-mislabeled entries (which used T-04:30Z style `2026-05-23T22:30` for rounds 24+30+31, locally-correct-time but UTC-mislabeled).

This is **conductor process state**, not a substrate gap. Conductor tick-N+1 will likely bump the cutoff again. No new finding — already documented in round-31 SHARED entry.

### (c) check-findings-only-fix-by.sh

```
🟢 eval-cron-duty-fires-agent-runner-end-to-end  within budget (2026-05-30, 7 days remaining)
🟢 eval-workflow-execution-runs-real-step  within budget (2026-05-30, 7 days remaining)
🟢 mission-hire-audit-payload-shape-validation  within budget (2026-05-30, 7 days remaining)
🟢 mission-cu-macos-cross-tenant-capability-test  within budget (2026-05-30, 7 days remaining)
🟢 mission-cross-tenant-test-audit-export  within budget (2026-05-25, 2 days remaining)

Tally: 5 within-budget · 0 due-today · 0 overdue · 0 informational · 5 total.
```

All 5 fix-by deadlines green. **No new findings-only-fix-by gaps surfaced this round** (vs round 31 also reported all-green for this script).

### (d) check-gateway-freshness.sh

```
STATUS: UNREACHABLE  (no response from http://127.0.0.1:7400/health — gateway not running?)
```

Gateway DOWN at session start (same as rounds 30/31). Operational note; the conductor's tick-103 acknowledged this informationally too ("Gateway UNREACHABLE (informational)"). Not a regression — `argo-gateway` requires deliberate startup, and the operator hasn't started it. No Chrome-MCP-driven scenarios attempted this round (axis 3 skipped per charter).

## Anti-theater check

Cu-macos: the additive fix is mechanical (`TenantId::new()` + `WorkerId::new()` are fresh-generator calls — no behavioral side-effect on the test, which doesn't itself assert on tenant_id or worker_id values; it asserts on permission-revoke behavior). The fix correctly addresses the struct-shape mismatch without weakening the test. Verified by source-read of the surrounding test body (`run_request` helper at lines 138-147 is just struct construction; the actual test body uses standard preflight + step-execute paths).

Process-drift: the `check-findings-only-fix-by.sh` script scans `BUILD_STATUS.md` for `Fix by:` annotations on MERGED-FINDINGS-ONLY rows and computes days remaining. It does NOT scan TODO rows. The round-30 P0 (`fix-cu-macos-permission-revoke-test-add-tenant-worker-fields`) was a TODO until 02:05Z when the conductor tick-99b's `7efd51388` restored it from BLOCKED to TODO; never had a `Fix by:` annotation. The script behaved as designed — the gap was that no separate gate existed for "P0-tagged TODOs aging past N hours."

Now that #2112 closed the specific P0 that exposed the gap, the urgency of `ops-add-todo-aging-gate-script` drops from P1-urgent to P1-defensive (still a real gap; just no longer producing visible damage). The brief is in the queue per tick-102 replenishment (operator confirmed via SHARED handoff).

## Gap / follow-up brief candidates

(0 new this round). Re-surfacing 2 already-queued candidates for visibility:

- **`ops-add-todo-aging-gate-script`** (P1 ~30-50 LOC, queued per tick-102) — still load-bearing as defense-in-depth even though the specific P0 that motivated it (round 30/31) is now closed. Recommendation: keep priority P1 but no longer urgent; safe to take during a quiet tick.
- **`ops-evaluator-heartbeat-concurrent-write-guard`** (P3 ~20-40 LOC, queued per tick-102 from round-31) — concurrent evaluator session race observed THIS round as well (my round-32 heartbeat write was reverted to round-31 contents mid-session, see system-reminder in transcript). Still real, still low-impact.

Carrying-forward observation: the conductor tick-99b RCA fix (PATH guards in `claude-worker-loop.sh` + `~/.zshenv`) appears to have held through 6+ ticks now without mac-host doom-loop recurrence — supervisor-watchdog notes "slots 2+3 in fast-respawn rc=1 loops" at tick-103 09:14Z but pulse files are fresh, so it's worker-iteration churn (claim/release cycling on stuck briefs #2088+#2092), not the supervisor-PATH host failure.

## Cite

- Verified merged collateral: [crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs:140-141](../../../../crates/argo-executor-computer-use-macos/tests/permission_revoke_mid_run_test.rs#L140)
- Commit anchor: `417b74746` (eval-claim-5-live-mcp-and-cu-test (#2112) — carried the cu-macos fix)
- Companion: rounds 26 (first cu-macos finding), 30 + 31 (P0 re-verify), tick-99b incident-resolved 7e9b1426a + 7efd51388, tick-102 brief replenishment 6a648e463
