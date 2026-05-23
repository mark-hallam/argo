# Round 42 — Process drift sweep + round-41 RED finding recheck

- **Date**: 2026-05-23T19:43Z (Melbourne local Sun 2026-05-24 05:43 AEST)
- **Round**: 42
- **Target commit**: `3df52142a` (origin/main HEAD)

## Verdict

🟡 **PARTIAL — P2** (process drift clean; round-41 P1 finding still LIVE-RED, both surfaced briefs still TODO; one process-gate amplifier surfaced)

## Setup

```bash
bash scripts/check-findings-only-fix-by.sh
bash scripts/check-unread-handoffs.sh
bash scripts/check-gateway-freshness.sh
SKYCOT_DB_KEY_BASE64=$(openssl rand -base64 32) cargo test -p argo-skill --lib store_worker_overlay_tests::
grep -A1 "fix-skill-worker-overlay-blob-text-join-mismatch\|ops-pre-merge-gate-fix-test-failures-not-just-clippy" docs/build-plan/BUILD_STATUS.md
```

## Observed

### Fix-by tracker — 🟢 5/5 within budget

| Brief | Fix-by | Days left |
|---|---|---|
| `mission-cross-tenant-test-audit-export` | 2026-05-25 | 2 days — **escalate if no PR claim by tick-126** |
| `eval-cron-duty-fires-agent-runner-end-to-end` | 2026-05-30 | 7 days |
| `eval-workflow-execution-runs-real-step` | 2026-05-30 | 7 days |
| `mission-hire-audit-payload-shape-validation` | 2026-05-30 | 7 days |
| `mission-cu-macos-cross-tenant-capability-test` | 2026-05-30 | 7 days |

### Handoffs gate — 10 unread above conductor cutoff `2026-05-23T19:14:25Z`

All 10 are my own evaluator entries with timestamps that arrange ABOVE the 19:14Z conductor cutoff (entries from rounds 25-38 with Melbourne-local-shaped timestamps). This is the same residue noted in round 41 (11 unread). PR #2150 watchdog defends the READ-side; the WRITE-side requires operator action on `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` (outside the repo, operator-actionable). **11th-consecutive-round observation**; not a code defect.

### Gateway freshness — UNREACHABLE (informational)

No gateway running on this dev box. Browser scenarios skipped per charter.

### Round-41 RED finding RECHECK — STILL LIVE on `origin/main`

```
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 351 filtered out; finished in 0.60s
failures:
    store_worker_overlay_tests::list_for_worker_filters_other_workers
    store_worker_overlay_tests::list_for_worker_returns_attached_active_overlays
```

Identical RED pattern as round 41 (commit `bc78952a7` → today's HEAD `3df52142a`). Schema divergence (`skill.id BLOB` at v1.rs:108 vs `worker_skill.skill_id TEXT` at 101__worker_skill.sql:28) preserved. Both surfaced briefs still TODO on the ledger:

- `fix-skill-worker-overlay-blob-text-join-mismatch` (P1 — tick-121, 21h+ unclaimed)
- `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (P0 — tick-123, 1h+ unclaimed)

### Drift-pattern compound — pre-merge gate ergonomics has TWO masking holes

This round's PR #2164 verification surfaced a SECOND gate-masking pattern, sibling to the already-surfaced `--skip-clippy` masking:

| # | Masking pattern | Bug it hides | Status |
|---|---|---|---|
| 1 | `--skip-clippy` shorts the only cross-crate sanity check | RED `argo-skill --lib` tests (BLOB↔TEXT bug, 2/5 failing for 6 PRs) | brief `ops-pre-merge-gate-fix-test-failures-not-just-clippy` TODO |
| 2 | Gate doesn't export `SKYCOT_DB_KEY_BASE64` → gateway-spawning tests hang at `await_listener_signal` deadline | Broken column-name typo `payload` vs `payload_json` in `demo_mode_mint_principal_smoke.rs:79` (now LIVE-RED on origin/main when key is set) | new brief candidate this round (see PR #2164 report) |

Both masking patterns produce identical net behavior: a real test failure is silenced by environment, the PR ships with a green pre-merge gate, the failure is invisible until an evaluator independently exercises the code path.

## Anti-theater check

Round-41 finding re-verified by running the exact same test command on a fresh cold-built target (`SKYCOT_DB_KEY_BASE64=$(...) cargo test -p argo-skill --lib store_worker_overlay_tests::` — note the SQLCipher key is now exported per #2165 runbook §5.1, which round 41 didn't have available). 2/5 RED is consistent regardless of `SKYCOT_DB_KEY_BASE64` value (the test exercises bare-store JOIN behavior, doesn't need a gateway bind).

The 10 unread handoffs are not stale CODE state — they're a known timestamp-shape residue in my own past entries above the 19:14Z conductor cutoff. The conductor's check-unread-handoffs.sh detects them (correct behavior); the underlying entries were already actioned in earlier ticks (rounds 25-38 recommendations are all on the ledger as briefs or dedup-confirmed).

## Gap / follow-up brief candidates

**No new briefs from drift-sweep alone.** Round 41's 2 surfaced briefs remain the load-bearing actions; PR #2164 verification this round adds 2 net-new brief candidates (see that report).

Re-surfaced still-open:

- `fix-skill-worker-overlay-blob-text-join-mismatch` (P1, 21h+ TODO — direct mission-claim-4 production data-integrity bug)
- `ops-pre-merge-gate-fix-test-failures-not-just-clippy` (P0, 1h+ TODO — gate-hardening for clippy-skip pattern)
- `ops-scheduled-task-prompt-add-explicit-date-u-instruction` (P1, operator-actionable, 7+ rounds open — SKILL.md edits outside repo)
- `mission-agent-runner-wire-process-registry-per-session` (P1, F1 sibling of #2117 triad — already on ledger TODO)
- `mission-cross-tenant-test-audit-export` (P0, fix-by 2026-05-25 in 2 days — CLOSEST deadline, escalate tick-126 if no claim)

## Severity rationale

🟡 P2 because (i) all process gates green, (ii) the round-41 RED finding is recheck-only (no new severity), (iii) the second gate-masking pattern surfaced is brief-candidate-grade, not P0-immediate. The compound observation — 2 gate-masking patterns in 2 consecutive evaluator rounds — IS load-bearing for the conductor; pre-merge gate ergonomics is at the top of the gate-hardening backlog.
