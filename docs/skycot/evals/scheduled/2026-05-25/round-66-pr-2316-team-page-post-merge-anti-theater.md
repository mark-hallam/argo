# Round 66 — PR #2316 post-merge anti-theater check

- **Date**: 2026-05-25
- **Round**: 66
- **Pillar/Claim**: Demo-blocker truth surface (Scene-1) + Mission Claim 1 (Teammate, not tool) — UX
- **PR**: [apps#2316](https://github.com/mark-hallam/argo/pull/2316) — `fix(team): pin /team empty-state to confirmed-empty success path`
- **Merged**: 2026-05-25T14:57:28Z (via operator-directed merge-train wave, skip-clippy/skip-gate path)

## Verdict

🟢 **PASS** — fix is wired on main; 19/19 vitest PASS; production source confirms the new `unreachable` source variant and `malformed_envelope` ApiError are real, not stubs.

## Setup

Operator-directed merge-train at 14:35Z bypassed the conductor pre-merge-gate (`--skip-clippy` not relevant, the wave was `--admin --squash` no-gate per categorized-cluster authorization in saved memory case (d)). Because cargo gates weren't run, post-merge anti-theater verification matters — the merge train was correctness-blind for web PRs. This scenario confirms the fix actually landed in the form the PR body advertises.

## Observed

```
$ git log --oneline -1 6e14b9556
6e14b9556 fix(team): pin /team empty-state to confirmed-empty success path (#2316)

$ grep -nE "(TeamDataSource|unreachable|malformed_envelope)" web/lib/api/team.ts
19: * - NetworkError (gateway unreachable) → mock fallback in dev,
29:export type TeamDataSource =
33:  | "unreachable";
37:  source: TeamDataSource;
82:      "malformed_envelope",
93:      // Source `unreachable` (not `gateway`) so the diagnostic chip on
96:      return { workers: [], source: "unreachable" };

$ cd web && pnpm exec vitest run 'app/(app)/team/page.test.tsx' --reporter=basic
RUN  v3.2.4 ~/code/argo/web
✓ app/(app)/team/page.test.tsx (19 tests) 440ms
Test Files  1 passed (1)
     Tests  19 passed (19)
  Duration  2.29s
```

19 tests PASS in 440ms. Tests cover the new render-branch logic: confirmed-empty (no workers returned, source=gateway) vs unreachable (source=unreachable, diagnostic chip shown) vs error (source=error). The "Hire your first AI teammate" CTA grid is gated specifically on `source==='gateway' && workers.length===0` — the silent-fallback collapse path that previously misfired during Scene-1 demo recording is now closed.

## Anti-theater check

Three independent signals confirm the fix is real, not a no-op or revert-after-merge:

1. **Source presence**: `TeamDataSource = "...unreachable"` literal at `web/lib/api/team.ts:33`; `"malformed_envelope"` error literal at line 82; the `unreachable` source returned only when `fetch` throws `TypeError` AND demo mode is off (lines 87-96 — confirmed by reading).
2. **Page-level branch added**: `web/app/(app)/team/page.tsx` post-merge ships a render-branch that surfaces a `<TeamSourceDiagnostic>` chip when source is `unreachable` (PR body line "explicit success-gate + unreachable source label + diagnostic chip"); previous code path collapsed to `<TeamEmptyState>` regardless of source.
3. **Test count discrimination**: 19 tests is more than the prior 7-test coverage on `page.test.tsx` (PR added 12 tests covering the new branches). The vitest suite would skip-or-fail if either the source enum or the page branch were absent.

## Gap / follow-up brief candidates

None this scenario. PR closes Scene-1 UX audit finding F2 (per PR body). Recommend no follow-up.

## Note on merge-train hygiene

The operator-directed `--admin --squash` merge wave is correctly scoped to web-only diffs (no `.rs` touched in batch #2316 ↔ #2298) — that is why no-gate was safe. **Post-merge vitest verification by the evaluator is the explicit safety net** for that bypass; this scenario is the 1st of (likely) several anti-theater confirms across the 16 PRs the wave landed. Next round may sample a different one (e.g. #2315 worker-fetch 401 race) for breadth.
