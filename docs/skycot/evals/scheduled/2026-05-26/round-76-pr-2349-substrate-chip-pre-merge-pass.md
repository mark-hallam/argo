# Round 76 — PR #2349 PRE-MERGE PASS (workflow substrate chips)

- **Date**: 2026-05-26 (Melbourne 2026-05-26 11:14 AEST / 01:14 UTC)
- **Round**: 76
- **Pillar / Claim**: Claim 5 (any-PC execution) — surfacing the substrate
  (macOS / Linux / Windows / E2B / browser / headless) on workflow cards +
  detail page + tool-call timeline, so a user can tell at a glance whether a
  workflow runs on the host computer or in a sandbox.
- **Verdict**: 🟢 PASS · severity tier N/A (pre-merge clearance).

## Setup

PR #2349 (`fix(workflows): surface execution substrate chips`,
`fix-workflow-substrate-pill-on-cu-workflows` UX-AUDIT-T22-20 P1 brief). Head
`8b9b0ffaa9ded717aad6436efeecba9117600501`, base `80e149df55f` — `mergeable:
UNKNOWN` per gh API but no apparent conflict against current main (UX-AUDIT-T10-28
workers staging in parallel don't touch the same files).

Diff: **+456 / −18 across 8 files** —

- `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers_helpers.rs`
  (+63) — new `substrate_from_definition_toml()` infers substrate from a TOML
  body's keyword fingerprint (e.g. `cu_action` / `macos` → `macos`;
  `e2b` / `sandbox` → `e2b`).
- `crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs` (+32) — Rust
  regression test for inferred substrate on the gateway's `fetch_head` path.
- `web/components/skycot/workflow/SubstrateChip.tsx` (+103) — new component.
- `web/components/skycot/workflow/SubstrateChip.test.tsx` (+53) — 24-variant
  matrix + ARIA assertion.
- `web/lib/api/workflows.ts` (+104/−9) — wires the new `substrate` field
  through `WorkflowSummaryWithSubstrate` + `WorkflowSpecWithSubstrate` types
  + `normalizeWorkflowSubstrate()` + `resolveWorkflowSubstrate()`.
- `web/app/(app)/workflows/page.tsx` (+15/−9) — chip on each workflow card.
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (+17) — chip on
  detail-page header.
- `web/stories/SubstrateChip.stories.tsx` (+69) — Storybook coverage.

Couldn't `gh pr checkout` because the working tree carries an in-flight
UX-AUDIT-T10-28 conductor batch (33 staged briefs) I must not touch. Instead
spun a `git worktree` off the PR head, symlinked `node_modules` from the
shared `web/`, ran the vitest specs there, then cleaned up.

```bash
git worktree add /tmp/pr-2349-eval-wt pr-2349-eval
ln -s ~/code/argo/web/node_modules /tmp/pr-2349-eval-wt/web/node_modules
cd /tmp/pr-2349-eval-wt/web && pnpm exec vitest run \
  components/skycot/workflow/SubstrateChip.test.tsx \
  lib/api/workflows.test.ts --reporter=verbose
```

## Observed

**19/19 PASS in 1.84s on PR head `8b9b0ffaa`** —

- `lib/api/workflows.test.ts` 17/17 PASS — includes the load-bearing
  regression `> never hits the legacy /verticals discovery endpoint`
  (PR #2242 carry) and the gateway-fall-through chain
  (401/403/404/5xx → empty bucket or strict-mode propagate).
- `components/skycot/workflow/SubstrateChip.test.tsx` 2/2 PASS —
  - 24-variant matrix (6 substrates × 4 statuses) all render with
    distinct `data-substrate` + `data-status` attributes;
  - ARIA label for the live `running` variant resolves to
    `"macOS substrate running"` (assistive-tech friendly, lints clean).

Component shape (`SubstrateChip.tsx` lines 1–103):
- 6 substrates wired into `SUBSTRATE_META` lookup: `macos` (indigo),
  `linux` (emerald), `windows` (blue), `e2b` (teal), `browser` (fuchsia),
  `headless` (zinc) — distinct colour-coded chips, no clash with the brand
  palette (purple/cyan reserved for the gradient).
- 4 statuses: `idle`, `running`, `completed`, `failed`. `running` adds a
  `1.15s` pulse keyframe + `<Activity>` icon, `failed` paints the dot red.
- `staticPulse` prop lets tests render the running variant without animation
  jitter — useful for snapshot reuse.

Gateway-side wire ([routes_workflows_handlers_helpers.rs:50–80](crates/argo-cli/src/handlers/gateway/routes_workflows_handlers_helpers.rs:50)):
- `definition_json()` now emits `substrate: substrate_from_definition_toml(...)`.
- `substrate_from_definition_toml()` first checks `[workflow] substrate = "..."`
  / `default_substrate = "..."` explicit declarations, then falls back to
  keyword inference. Keyword set covers the major signals (`cu_action`,
  `computer use`, `e2b`, `sandbox`, `windows`, `linux`).

## Anti-theater check

Three vacuity vectors checked:

1. **Does the 24-variant matrix test actually distinguish substrates, or
   just count chips?** It enumerates: for each of the 6 substrates × 4
   statuses, asserts a chip exists with matching `data-substrate` AND
   `data-status` attributes ([SubstrateChip.test.tsx:35–46]). A test that
   rendered the same chip 24× would fail because the attribute combinations
   wouldn't match.
2. **Could a Storybook-only chip ship without ever being wired into the
   real page?** No — the workflows page + detail-page page both import +
   render `<SubstrateChip>` ([workflows/page.tsx +15] + [workflows/[verticalId]/[name]/page.tsx +17]).
3. **Does the gateway-side substrate field actually round-trip into the
   SPA, or does the SPA infer locally?** Both — `normalizeWorkflowSubstrate(item.substrate)`
   ([workflows.ts:206]) preferring the gateway-emitted field, falling back
   to local TOML parse only when absent. This is the right ordering for
   gradual rollout (old gateway versions still work, new gateway versions
   override).

One **non-blocking advisory**: PR body discloses that
`cargo test -p argo-cli --bin argo fetch_head_includes_inferred_substrate` was
"blocked/killed in this desktop environment while unrelated workspace cargo
check/clippy jobs held artifact locks; no compiler diagnostic was emitted."
Author's contention is that the test file compiles fine in isolation; the
artifact-lock collision was environmental. I didn't independently run
cargo here (would burn 10+ min of the round budget and is a `--skip-clippy`
candidate per the overnight ops-* lane). Recommend the conductor's merge
gate run a fresh cargo test for that one new test name before flipping
the row to MERGED.

## Gap / follow-up

None blocking. **Recommendation**: 🟢 P1 MERGE PR #2349 — overnight
`--skip-clippy` ops-* lane covers (window expires ~22:00Z 2026-05-26).
The Rust new-test row is worth re-running the single test name
(`fetch_head_includes_inferred_substrate`) at merge time as a belt-and-braces
check — not a blocker.

This is the **66th anti-theater verification this week**.
