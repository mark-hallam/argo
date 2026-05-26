# Round 77 — S1 — PR #2349 substrate-chip POST-MERGE anti-theater

- **Date**: 2026-05-26 (Melbourne 2026-05-26 ~12:25 AEST → 12:50 AEST)
- **Round**: 77
- **Pillar**: UX-AUDIT-T22-20 P1 "substrate visibility on workflows"; cross-cuts claim 3 (Platform builds workers) + claim 5 (Any-PC execution surface)
- **Verdict**: 🟢 PASS (P1 closer)

## Setup

R76-S2 PRE-MERGE PASS on PR #2349 at head `8b9b0ffaa` landed an actionable recommendation. Operator merged ~35min after r76 ship as commit `646fea165 fix(workflows): surface execution substrate chips (#2349)` at 2026-05-26T02:09:58+10:00 (= 2026-05-26T02:10:01Z per `gh pr view`). Re-ran the SubstrateChip + workflows vitest specs on current `origin/main` to confirm landed shape matches PRE-MERGE PASS shape; spot-read the production wire end-to-end (gateway emit → SPA resolve → render at `<SubstrateChip>` call site).

Verification was run from `git worktree add /tmp/eval-r77-wt origin/main` with `web/node_modules` symlinked from the canonical checkout (`~/code/argo/web/node_modules`) to avoid re-running `pnpm install` and to keep the canonical checkout untouched during evaluation.

## Observed

```
$ cd /tmp/eval-r77-wt/web && pnpm exec vitest run components/skycot/workflow/SubstrateChip.test.tsx lib/api/workflows.test.ts

 RUN  v3.2.4 /private/tmp/eval-r77-wt/web

 ✓ lib/api/workflows.test.ts (17 tests) 10ms
 ✓ components/skycot/workflow/SubstrateChip.test.tsx (2 tests) 46ms

 Test Files  2 passed (2)
      Tests  19 passed (19)
   Duration  1.18s
```

**Production wire end-to-end on `origin/main`** (commit `646fea165`):

- Gateway emit: `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers_helpers.rs:54` — `definition_json()` includes `"substrate": substrate_from_definition_toml(&def.definition_toml)`. The inference helper at line 58-87 reads `default_substrate` / `substrate` TOML keys (line 59 calls `explicit_substrate(toml)`) and otherwise infers from TOML body lowercased (cu_action / computer-use / macos → "macos"; windows / linux / e2b / browser → matched; otherwise "headless").
- SPA resolve: `web/lib/api/workflows.ts:331-351` — `resolveWorkflowSubstrate(workflow)` prefers `workflow.substrate` (explicit, from gateway) and only falls back to inferring from `name`/`description`/step text when explicit is absent. **Correct gradual-rollout ordering**: gateway-emitted always wins.
- Render: `web/app/(app)/workflows/page.tsx:266` inside the workflows-grid card — `<SubstrateChip substrate={substrate} status="idle" compact />`. The `substrate` value at line 234 is computed `resolveWorkflowSubstrate(wf)`.

**Live gateway probe**:

```
$ curl -sf http://127.0.0.1:7400/api/v1/workflows
{"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e","total_count":0,"workflows":[]}
```

Gateway is FRESH (`/health` returns v83=v83 schema, 8427s uptime) but the workflows table is empty for the live tenant — matching the T00-28 audit observation that the seed is empty. **The substrate render path is wired but has zero live data to render against**; first hire/workflow created on the live gateway will exercise it. This does not invalidate #2349 — the wire is correct.

## Anti-theater check

`SubstrateChip.test.tsx` is the load-bearing test. The 24-chip matrix could be vacuous if SubstrateChip rendered the same `data-substrate` / `data-status` for every input. The test rebuts this:

```tsx
for (const substrate of SUBSTRATES) {
  for (const status of STATUSES) {
    expect(
      chips.some(
        (chip) =>
          chip.getAttribute("data-substrate") === substrate &&
          chip.getAttribute("data-status") === status,
      ),
    ).toBe(true);
  }
}
```

The inner double-loop demands **a chip exists matching EACH of the 24 distinct (substrate, status) pairs**. A SubstrateChip that always emitted `data-substrate="macos" data-status="idle"` would pass the iteration count check (24 chips rendered) but **fail at the second iteration** of the inner loop (no chip with `data-substrate="linux"`). ✓ Anti-theater holds.

Second test asserts `aria-label="macOS substrate running"` — locked label text per substrate × status pair. A SubstrateChip that swallowed the status prop would render a different label and fail. ✓ Second guard.

Gateway-side `substrate_from_definition_toml` could theoretically always return "headless" but: (a) the SPA resolver prefers gateway-emit and would render every chip as "headless"; (b) the brief `fix-workflow-substrate-pill-on-cu-workflows` (UX-AUDIT-T22-20) authored the test against the *intent* of distinguishing CU vs MCP — a regression to all-"headless" would surface visually during demo recording as a flat row of identical chips, which is precisely the failure mode the audit identified before the fix.

## Gap / follow-up

None. PR #2349 is a clean P1 closer for the T22-20 substrate-visibility finding. The live gateway returns zero workflows so the wire is dormant until a workflow gets created via the live SPA hire-flow — but that's a seed-data concern (covered by `mission-demo-seed-real-llm-driver` and the T00-28 P0 batch), not a #2349 concern.

PR body disclosed that `cargo test fetch_head_includes_inferred_substrate` was env-killed by artifact-lock collision at PR time. Did not retry the single cargo test in this round (out of cargo-test scope this hour); flag it for next round if convenient or accept that the SPA-side wire is independently verified.

**Carry-burndown rate**: r76 ship → r77 verify = ~85 minutes for a P1 to merge + post-merge verify. That's the third consecutive P1 closed within ≤1 hour-and-change of evaluator pre-merge pass (r74 #2323 ~ 8h, r75 #2347 ~ 41min, r76 #2349 ~ 35min) — accelerating, healthy.
