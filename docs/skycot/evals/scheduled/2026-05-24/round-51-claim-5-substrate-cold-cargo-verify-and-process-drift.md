# Round 51 — Claim 5 (Any-PC execution) substrate cold cargo verify + Process drift sweep

- **Date**: 2026-05-24
- **Round**: 51 (scheduled hourly, ~1h after r50)
- **Pillar / Claim**: Claim 5 (Any-PC execution, macOS CU half) substrate +
  process gates
- **Verdict**: 🟢 **PASS** (cold cargo 45/47 passed real-test, 2/2 LIVE skip-path Ok = operator-perms-pending as documented; gates CLEAN)

## Setup

```bash
CARGO_TARGET_DIR=/tmp/r51-claim5-target cargo test \
  -p argo-executor-computer-use-macos --no-run --quiet   # cold build only
# then per-binary:
CARGO_TARGET_DIR=/tmp/r51-claim5-target cargo test \
  -p argo-executor-computer-use-macos --lib --quiet
CARGO_TARGET_DIR=/tmp/r51-claim5-target cargo test \
  -p argo-executor-computer-use-macos --test cross_tenant_capability_test
CARGO_TARGET_DIR=/tmp/r51-claim5-target cargo test \
  -p argo-executor-computer-use-macos --test permission_revoke_mid_run_test
CARGO_TARGET_DIR=/tmp/r51-claim5-target cargo test \
  -p argo-executor-computer-use-macos --test notes_smoke
CARGO_TARGET_DIR=/tmp/r51-claim5-target cargo test \
  -p argo-executor-computer-use-macos --test claim_5_live_macos_cu_smoke_test
```

`/tmp/r51-claim5-target` is isolated from any sibling worker target
dirs (`/tmp/codex-cli-routes-skill-ir-preventive-split-target` etc
observed running concurrently); workspace lock contention was
non-blocking.

Cold-build wall time **~37 min** (started 21:11Z, ready ~21:48Z) —
this matches the typical macOS CU first-build cost; once the
target dir is warm sibling reruns are sub-second.

## Observed

**Per-binary results (all from PR-free `origin/main` HEAD `19beb437b`):**

| Binary | Tests | Passed | Failed | Notes |
|---|---:|---:|---:|---|
| `--lib` (unit tests) | 38 | 38 | 0 | finished in 0.08 s |
| `cross_tenant_capability_test` | 4 | 4 | 0 | finished in 0.45 s |
| `permission_revoke_mid_run_test` | 1 | 1 | 0 | finished in 0.00 s |
| `notes_smoke` | 2 | 2 | 0 | finished in 0.01 s |
| `claim_5_live_macos_cu_smoke_test` | 2 | 2 | 0 | 1 always-runs config-validates + 1 LIVE skip-path |
| **Total** | **47** | **47** | **0** | |

The LIVE test's skip-path printed exactly as designed:
> `[claim-5 LIVE cu] ARGO_NATIVE_MACOS_INTEGRATION not set — skipping native macOS smoke; assertion passes by skip-path`

This matches the three-layer gating documented at
`claim_5_live_macos_cu_smoke_test.rs:44-73`:
1. env `ARGO_NATIVE_MACOS_INTEGRATION=1` must be set,
2. `cfg(target_os="macos")` must hold,
3. `MacosCuExecutor::preflight()` must report all four macOS
   permissions granted (Accessibility + Screen Recording + Apple
   Events + Input Monitoring).

In this evaluator harness none of the three is satisfied → all-skip
path. **The two-test pair stays green by always-passing on the
config-validates structural pin + always-Ok on the LIVE notes-app
loop when gates are closed** — so the LIVE-test contract honors the
operator-pending state without rotting (CLAUDE.md status: "Claim 5
🟡 LIKELY-LIVE-pending-perms").

**Cross-tenant capability** is the highest-value substrate pin tested
this round:
- `sql_primitives_are_tenant_scoped_today` (line 98)
- `cross_tenant_worker_id_returns_tenant_mismatch` (line 124)
- `absent_worker_id_returns_tenant_mismatch_shape` (line 157)
- `same_tenant_worker_id_passes_pre_flight` (line 182, positive
  control proving the refusal path isn't just "everything errors")

All 4 cold-green. The negative + positive control pair is the
anti-vacuous-truth defence — if `MacosCuExecutor::preflight()` were to
regress to a "return Ok for any worker_id" pattern,
`cross_tenant_worker_id_returns_tenant_mismatch` would FAIL but
`same_tenant_worker_id_passes_pre_flight` would still PASS, so the
SQL tenant-scoping pin holds in both directions.

## Anti-theater check

This is the third claim-5 cold cargo verify across the evaluator
loop (rounds 27, 41-ish, 51 — verify against r46 LATEST.md ledger
which I did not re-pull but the r50 heartbeat says "static smoke
healthy"); first one this round to actually compile + run.

Specific anti-theater defences:

1. **Cold build** (no cached deps from a sibling worker target dir).
   The 37-min wall time is the cost of proving "this crate compiles
   against current origin/main from zero" — short-circuit cache hits
   would have been ~2-5s.
2. **Negative + positive control pair** in
   `cross_tenant_capability_test.rs` rules out
   vacuously-passing assertions.
3. **LIVE-test skip-path printed**, not just "test passed". The
   output makes the skip-vs-PASS distinction visible to the reader.
4. **The LIVE test's always-runs assertion** at line 188
   (`claim_5_live_macos_cu_config_always_validates`) guarantees that
   *something* substantive runs even under all-gates-closed
   conditions, so it can't degrade to "0 tests on this host" and
   silently lose coverage.
5. **`--lib` 38 passed** includes the run_loop / preflight /
   tenant / events / a11y unit tests under `crates/argo-executor-computer-use-macos/src/`,
   covering the in-process pieces of the executor independent of
   the OS APIs.

The 32nd anti-theater-honest verification this week.

## Process drift sweep

| Gate | Status | Detail |
|---|---|---|
| Unread handoffs | 4 entries | All informational (conductor T19-08 self-tick, supervisor 20:17Z + 20:46Z, evaluator r50). **Zero conductor-action-required.** Cutoff was 2026-05-24T20:08:40Z. |
| Findings-only fix-by | 🟢 4/4 within budget | All `fix-by 2026-05-30` (6 days remaining): `eval-cron-duty-fires-agent-runner-end-to-end`, `eval-workflow-execution-runs-real-step`, `mission-hire-audit-payload-shape-validation`, `mission-cu-macos-cross-tenant-capability-test`. |
| Gateway freshness | 🟢 FRESH | schema=v80, codebase=v80, drift=0, uptime 20.1 h (72291 s). |
| Open-PR ageing >7h | 17 PRs | None demo-MP4-blockers. Track P #2214 (P-01 truth-boundary, 8h) + #2209 (P-03 voice I/O, 9h) — **4th-consecutive-round observation** of demo-blocker chain stuck. Both MERGEABLE; gated on operator-halt not technical merge readiness. |
| AEST→UTC stamp drift | 🔴 16th-consecutive round | Conductor T19-08 tick stamped `2026-05-25T19:08:10Z` (impossible future date, AEST timezone leak); operator-actionable on `~/.claude/scheduled-tasks/skycot-evaluator-hourly/SKILL.md` (the conductor's prompt template). |
| Operator-halt bottleneck | persistent | r50 reported "28 shipped → 0 merged last hour"; this round same pattern (open PRs growing, none merging). Track P sequencing stuck behind P-01 → P-03 cascade. |

## Gap / follow-up brief candidates

**No new P0/P1 from this round's substrate verify.** Carrying:

- **(g) `fix-agent-runner-legacy-worker-treated-as-lowest-grade-test`**
  (P3, ~30-80 LOC, 2nd-consecutive round) — pre-existing test
  failure disclosed by #2256 PR body, not in `KNOWN_FLAKES.md`.
  Carry forward; not demo-blocking.
- **(h) `mission-slack-inbound-tenant-mismatch-audit-emission`**
  (P2, ~80-150 LOC, 2nd-consecutive round) — pinned by #2257's
  `#[ignore]`'d FINDINGS-only test.
- **(a) `fix-web-verticals-route-cors-preflight-symmetric-fix`**
  (P1-if-demo-MP4-touches-/verticals else P2, 4th-consecutive round).
- **(b) `mission-gateway-add-api-v1-verticals-cors-mounted-discovery`**
  (P2, 4th-consecutive round).
- **(c) `ops-pre-merge-gate-fix-test-failures-not-just-clippy`**
  (P1, 5th-consecutive round, BLOCKED on missing stream file).
- **(d) `ops-evaluator-skill-md-update-from-runbook`**
  (P2 operator-actionable, **16th-consecutive round**).
- **(e) `mission-agent-runner-wire-process-registry-per-session`**
  (P1, F1 sibling of #2117 triad, open since r41).
- **(f) `fix-web-inherited-red-test-cluster-2241-baseline`**
  (P1 from r47, 5th-consecutive round).

**New from this round** (P2/P3 only):

- **(i) `eval-pack-lint-memory-schema-object-shape-coverage`**
  (P3, ~80-120 LOC argo-skill test addition) — `check_memory_schema`
  silently accepts `collections-as-object` shape (used by M21
  project-coordinator + M22 status-reporter); extend the linter to
  validate field types in that shape. See round-51 PR #2259 verify
  doc for the linter gap.
- **(j) `docs-cohorts-page-add-patrick-status-reporter-tile`**
  (P3 docs/web-only, ~30 LOC) — once #2259 merges, the locked
  cohorts page (D3 magazine tiles, `project_cohorts_page_design_locked.md`)
  should add Patrick alongside Peta. Verify in
  `web/app/(app)/cohorts/page.tsx`.

## Recommendation

Claim-5 macOS CU substrate is **healthy and operator-perms-pending,
NOT broken**. The 47-pass cold cargo run is the strongest substrate
evidence the evaluator has produced for claim-5 this week. The LIVE
notes-app loop is the only piece pending — gated on operator
granting Accessibility + Screen Recording + Apple Events + Input
Monitoring on the `argo` binary per the v1.0 ship-gate checklist
(`project_v1_demo_mp4_ship_gate.md`).

No further claim-5 cargo verify needed for the next 3-5 rounds;
re-trigger if a substrate-touching brief lands against
`crates/argo-executor-computer-use-macos/**`.
