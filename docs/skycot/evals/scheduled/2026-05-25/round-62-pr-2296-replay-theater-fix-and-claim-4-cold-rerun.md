# Round 62 — PR #2296 replay-theater fix PRE-MERGE PASS + Claim 4 cold rerun + process drift

- **Date**: 2026-05-25 (Melbourne 21:35 AEST = UTC 11:35Z)
- **Round**: 62
- **Scope**: 3 scenarios — (S1) PRE-MERGE verify PR #2296 (Scene-6 demo-gate fix for replay theater), (S2) Claim 4 (Skills-shared-and-learned) cold-rerun on current main (untouched in last 2 rounds), (S3) process drift sweep.
- **Time budget**: ~45 min wall, ~40K tokens. Within budget.

## Scenario 1 — 🟢 PR #2296 `fix(web): replay page no longer fabricates forensic report for unknown action IDs` PRE-MERGE PASS

- **Verdict**: 🟢 PASS — CLEAN for merge.
- **Pillar**: Scene-6 demo-gate / "no theater the camera sees" (operator-locked [feedback_no_theater_in_demo](../../../memory/feedback_no_theater_in_demo.md)).
- **Severity**: This PR CLOSES a P0 theater finding from the Scene-6 ux-audit-agent-4 batch (commit `19aa064ed`). Pre-fix code was a literal demo-recording blocker.

### Setup

- PR head: `52fcfe0a685895cfe192ad9077bd12ee9c84faeb`
- `gh pr view 2296 → mergeable=MERGEABLE, mergeStateStatus=CLEAN, +170/-90 across 3 files`
- Files: `web/lib/api/replay.ts` (+54/-90 — gate added, fabrication primitives removed), `web/lib/sample-replay.ts` (+0/-71 — `mockOutcomeForActionId` deleted), `web/lib/api/replay.test.ts` (NEW 135 LOC regression suite)
- Isolated worktree: `/tmp/argo-r62-pr2296-wt` (continuing r61's pattern to bypass supervisor-watchdog mid-cargo branch resets; one worker `mission-pr2296-related` was running an unrelated `cargo test -p argo-cli routes_audit_list_detail_alignment` PID 90616 in parallel — no collision because PR is web-only)
- node_modules symlinked from main checkout (not copied — `cp -r` failed with OOM under memory pressure; symlink works for vitest with no native rebuild needed)

### Anti-theater verification

The pre-fix code on `origin/main:web/lib/api/replay.ts` contained two literal fabrication primitives:

```rust
// pre-fix (origin/main:60-100)
export function mockVerdict(actionId: string): Verdict {
  if (actionId.length === 0) return "reproducible";
  const h = actionId.charCodeAt(actionId.length - 1) % 3;
  return (["reproducible", "drifted-aligned", "drifted-misaligned"] as const)[h];
}
// ... mockOutcome(actionId) called unconditionally in catch(404) and isAuthBypassed()
```

A 32-char-hex action ID ending in `e` (code 101) hashed `101 % 3 = 2` → `drifted-misaligned` → the page rendered a red "Policy regression detected" banner whose ORIGINAL/REPLAYED diff strings were concatenated from URL bytes. A buyer clicking any 404'ing audit row landed on synthesized forensic content with the buyer's own URL parameter visible as quoted "before" / "after" text.

The PR head removes the primitives ENTIRELY — not gates them. `grep -rn "mockOutcomeForActionId\|mockVerdict\|mockOutcome" --include="*.ts" --include="*.tsx" /tmp/argo-r62-pr2296-wt` returns ZERO matches across the entire web/ tree. A future `grep + import` cannot revive the fabrication because the symbols no longer exist. This is the strongest possible fix — "delete the gun, don't put the safety on."

The replacement `sampleOrFailed(actionId)` at `replay.ts:78-81` short-circuits to `UNKNOWN_ACTION_FAILED` envelope unless BOTH (a) `isDemoMode()` OR `isAuthBypassed()` is true AND (b) `findSampleReplay(actionId)` returns an entry from the canonical TABLE of 5 personas + 3 anchor IDs. Strict-prod path (`NODE_ENV=production` + `NEXT_PUBLIC_ENV=production`) bypasses sampleOrFailed entirely — 5xx errors re-throw to `<FailedView>`.

### Test verification

Cold vitest runs in worktree:

```
$ npx vitest run lib/api/replay.test.ts
 ✓ lib/api/replay.test.ts (7 tests) 108ms
 Test Files  1 passed (1)  Tests  7 passed (7)
 Duration  1.79s

$ npx vitest run "app/(app)/audit/action/[actionId]/replay/page.test.tsx"
 ✓ app/(app)/audit/action/[actionId]/replay/page.test.tsx (11 tests) 325ms
 Test Files  1 passed (1)  Tests  11 passed (11)
 Duration  3.23s
```

**18/18 PASS** total. Test assertions are meaningful (not vacuous):

- `replay.test.ts:33-44` — feeds `01234567890abcdef0123456789abcde` (the exact pre-fix attack vector with last char `e` → previously hashed to `drifted-misaligned`), mocks `ApiError(404)`, asserts `outcome.kind === "failed"` and `reason` matches `/seeded demo personas/i`. Pre-fix would have asserted `success/drifted-misaligned`. Bug regression would FAIL loudly.
- `replay.test.ts:46-63` — loops over three unknown IDs spanning the `% 3` hash space (`e`, `f`, `9`) — exhaustive coverage of the URL-byte fabrication pathway.
- `replay.test.ts:65-90` — confirms canonical TABLE IDs still return the right verdict in demo mode + auth-bypass (forward compatibility — sample fixtures still work for the recorded demo).
- `replay.test.ts:102-110` — re-throws 5xx (503), proving the 404-fallthrough is path-specific not a generic error swallower.

### Functional observations

- `replay.ts:78-81` `sampleOrFailed` gate is BOOLEAN AND — both conditions required. No accidental `||` that would re-enable the fallthrough in dev.
- `replay.ts:95-97` auth-bypass branch returns sampleOrFailed BEFORE hitting the gateway — no network call leaks tenant info for an unknown id under bypass.
- `replay.ts:102-107` strict-prod check is preserved (`isStrictProduction` predicate unchanged). Production never falls through to the sample table even for a TABLE-hit id — strict-prod is the safety floor.

### LOC budget

- `replay.ts` 109 LOC (well under 500 ceiling, down from ~150 pre-fix due to mockVerdict + mockOutcomeForActionId removal)
- `sample-replay.ts` 136 LOC (down from ~207 — `mockOutcomeForActionId` block deleted)
- `replay.test.ts` 135 LOC (new file, clean)
- No `find ... -size +500` violations in the diff. Unrelated >500-LOC files exist on main (`workflow-templates.ts:654`, `CommandPalette.tsx:1023`, etc.) but unchanged by this PR.

### Recommendation

**MERGE on next conductor tick.** Closes a P0 theater finding the camera would see. No follow-up brief required — the fix is complete and defensive (primitives deleted).

## Scenario 2 — 🟢 Claim 4 (Skills-shared-and-learned) cold rerun on current main

- **Verdict**: 🟢 PASS — claim 4 LIVE status holds on current `origin/main` (commit `41ebf3a82`).
- **Pillar**: MISSION claim 4. Not touched in r60 or r61 evals; previous cold rerun was the 2026-05-25 morning session `ops-claim-4-and-6-live-cold-rerun-on-current-main`.

### Setup

- Worktree: `/tmp/argo-r62-claim4-wt` (separate from S1 worktree to avoid PR-head pollution)
- Target dir: `/tmp/argo-r62-claim4-target` (cold — empty before run)
- Test: `cargo test -p argo-skill --test claim_4_live_install_invoke`

### Observed

```
Compiling argo-skill v0.0.1 (/private/tmp/argo-r62-claim4-wt/crates/argo-skill)
Finished `test` profile [optimized + debuginfo] target(s) in 2m 55s
Running tests/claim_4_live_install_invoke.rs

running 1 test
test claim_4_live_skill_install_invoke_and_audit_with_provenance ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;
finished in 0.23s
```

Cold compile 2m55s, runtime 0.23s. Test does Phase 0 schema preflight (`SELECT name FROM pragma_table_info('skill_provenance')` asserting 6 required columns: `skill_name`, `skill_version`, `provenance_kind`, `actor`, `parent_id`, `ts`), then real install → invoke → audit-join through `LayeredLookup`.

### Anti-theater check

Production wire confirmed at `crates/argo-agent-runner/src/turn_helpers.rs:71`:

```rust
let lookup = LayeredLookup::with_library(workspace, library);
let mut rows = lookup.list_for_action("low").await?;
```

This is the load-bearing call referenced in [project_v1_strategic_decisions_2026_05_14](../../../memory/project_v1_strategic_decisions_2026_05_14.md) as "MISSION.md success scenario #2 is now runtime-live". The test exercises the real call path — `claim_4_live_install_invoke.rs:178` opens a workspace + session, attaches a resolved skill, then invokes the agent runner. If the workspace fallback skipped the library (e.g. a regression that drops `library: Option<&SkillStore>` from the function signature), the test would fail on the missing provenance row in `skill_provenance`. The assertion is the SQL preflight + an end-to-end install/invoke/audit round-trip — non-vacuous.

### Recommendation

Hold claim 4 at LIVE. No follow-up needed.

## Scenario 3 — 🟢 Process drift sweep ALL GREEN

- **Gateway**: `curl http://127.0.0.1:7400/health → schema_version=81, uptime_seconds=19690 (~5h28min since 09:44Z #2283 restart)`. `bash scripts/check-gateway-freshness.sh → gateway_schema=v81 codebase_latest=v81 drift=0 STATUS: FRESH`.
- **Findings-only fix-by**: 4/4 within budget — all 2026-05-30, 5 days remaining (`eval-cron-duty-fires-agent-runner-end-to-end`, `eval-workflow-execution-runs-real-step`, `mission-hire-audit-payload-shape-validation`, `mission-cu-macos-cross-tenant-capability-test`).
- **Drift sweep**: `python3 scripts/find-stale-claim-drift.py --quiet → 0 drift rows`.
- **Unread handoffs**: 12 entries above conductor cutoff `2026-05-25T09:15:00Z`. Composition: 6 old conductor scheduled-tick ticks from yesterday (T17-33 / T18-06 / T19-08 / T20-08 — known race the conductor flags as informational), 2 supervisor-watchdog R5-kill entries from T11-07Z (informational — workers shipped PRs #2294 + #2290 BEFORE the post-completion hang per the watchdog's own note), 4 evaluator entries (r58/r59/r60/r61 — actioned by conductor T10-00 + T10-25 + T10-30 commits but cutoff bookkeeping lags ~1-2 ticks). No NEW unread handoffs since r61.

**7th consecutive round of process-loop health 🟢**: r55 BLOCK-in-0min · r56 MERGE-WITH-FOLLOWUP+2-followups-in-5min · r57 5-carry-closures · r58 carry-now-tracked + marker-auto-resolved · r59 #2292 post-merge clean + #2290 intent-sound · r60 #2274 PASS + #2268 P1 LOC-fix · r61 #2295 P1 LOC-fix + #2290 rebase landed · r62 #2296 theater-fix CLEAN.

## 46th anti-theater verification this week.

## Recommendations to next conductor tick

1. 🟢 **P0 MERGE #2296** — Scene-6 demo-gate fix, theater hole sealed, 18/18 tests PASS, primitives deleted not gated, LOC well under ceiling. Closes a P0 ux-audit finding from `19aa064ed`. No follow-up brief needed.
2. 🟢 **P0 MERGE #2290** — claim-2 KPI rollup, take 2. Carries r61 verdict; still CLEAN MERGEABLE per `gh pr view`. Closes (n-r55) 6-round carry.
3. 🟢 **P2 MERGE #2274** — pack-lint memory schema. Carries r60+r61 verdict.
4. 🔴 **P1 Author** `fix-pr-2295-split-event-tests-rs-claim-truth-boundary` AND `fix-pr-2268-split-event-tests-rs-loc-budget-breach` (or batch as a single `fix-event-tests-rs-loc-budget-pair` brief) — both PRs need the same mechanical split. From r60 + r61 carries (q-r60, NEW-r61).
5. housekeeping: **Close #2279** as superseded by #2290 per PR body.

## Carry status

| Round | Carry | State this round |
|---|---|---|
| (h) #2268 | 8th-round | Still OPEN; needs `event_tests.rs` split before merge. |
| (i) #2274 | (closes on merge) | Still MERGEABLE; rec #3 closes. |
| (m) cycle-detector | held | Operator downgrade pending. |
| (n-r55) #2290 | (closes on merge) | CLEAN per r61 + r62; rec #2 closes 6-round carry. |
| (o-r56) `fix-demo-seed-emit-worker-hired-audit-rows` | tracked TODO row 214 | #2294 in review — may bundle the audit emission. |
| (p-r57) `mission-demo-seed-real-llm-driver` | PR #2294 OPEN | Pending PRE-MERGE eval; post-completion-hung but shipped. |
| (q-r60) split `event_tests.rs` for #2268 | OUTSTANDING | Same shape as r61 carry. |
| (r-r61) split `event_tests.rs` for #2295 | OUTSTANDING | Could batch with q-r60. |
| (NEW r62) — | none | No new evaluator finding this round. |

## Cost

- Wall time: ~45 min (within 30-60 min budget).
- Token spend: ~40K (within 30-80K budget).
- Notes: Vitest in symlinked-node_modules worktree was the unblock — `cp -r` of node_modules failed with exit 144 (OOM-kill under memory pressure); symlink works for vitest because vitest doesn't write into node_modules. Recommend pattern for future scheduled-eval web verifications under active worker pool.
