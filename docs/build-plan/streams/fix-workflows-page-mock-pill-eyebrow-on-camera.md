# fix-workflows-page-mock-pill-eyebrow-on-camera

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-workflows-page-mock-pill-eyebrow-on-camera`.
**Source**: e2e-ux-audit Agent 3 finding F1 (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🔴 **P0 DEMO-GATE — Scene 5 is narrated for ~80s on a page that literally labels itself "WORKFLOWS · MOCK" on camera.**

## Root cause

`web/app/(app)/workflows/page.tsx:68` renders a brand-eyebrow pill that flips between `WORKFLOWS · LIVE` and `WORKFLOWS · MOCK` based on the `useWorkflows()` query's `source` field. With the current gateway state (empty per-vertical workflow lists), `web/lib/api/workflows.ts:155` returns `source: "mock-offline"`, the page falls through to `SAMPLE_WORKFLOW_SUMMARIES`, and the pill shows **MOCK**.

DOM-verified by Agent 3: `document.querySelector('[data-testid="workflows-source-pill"]').textContent === "WORKFLOWS · MOCK"`. The pill is in the brand purple accent, large, top-left — the camera cannot avoid it during Scene 5.

Violates the operator-locked "no theatre in the demo" rule (`feedback_no_theater_in_demo`): the operator's voiceover for Claim 5 talks about "real-world execution" while the page literally labels itself MOCK.

## Findings this brief closes

- Agent 3 F1 [P0] — `/workflows` ships "WORKFLOWS · MOCK" eyebrow pill on screen

## Fix recipe

Two options; pick one based on whether real workflows ship for the demo:

**Option A (preferred if real workflows will be seeded)**:
1. Ship real per-vertical workflow seed data via the demo-seed CLI (coordinate with `fix-demo-seed-cli-for-real-roster` and any in-flight workflows-seed brief).
2. Confirm `useWorkflows()` returns `source: "gateway"` before recording.
3. Leave the LIVE/MOCK pill as a dev tool — but verify it's invisible at demo time.

**Option B (preferred if real workflows won't ship in time)**:
1. Drop the `· MOCK` / `· LIVE` suffix from the eyebrow pill entirely. Replace with a version label like `WORKFLOWS · v0.1` or just `WORKFLOWS`. The source discriminator belongs in a dev panel (e.g. a `?debug=source` query param), NOT on the demo surface.
2. Optionally retain a console.warn in dev mode when source !== "gateway", so devs still see the signal.

**Either way**: the literal "MOCK" string must not appear on `/workflows` at demo time. Apply the same discipline to any sibling page that has a similar source-pill (audit `/skills/page.tsx`, `/audit/page.tsx`, `/approvals/page.tsx` — Agent-3 found the workflows pill; verify the others don't have analogues).

## Acceptance criteria

- [ ] `/workflows` does NOT render the literal text "MOCK" anywhere on the page in either dev or prod mode at demo time.
- [ ] DOM check: `document.body.textContent.includes("MOCK")` returns `false` on `/workflows`.
- [ ] If Option A: `useWorkflows()` returns `source: "gateway"` on the demo tenant.
- [ ] No regression on the empty-state path (when zero workflows exist, page renders gracefully).
- [ ] Sibling-page sweep: `/skills`, `/audit`, `/approvals`, `/team` do NOT render a MOCK pill either.

## May-touch

- `web/app/(app)/workflows/page.tsx:68` (the `{isLive ? "LIVE" : "MOCK"}` ternary)
- `web/lib/api/workflows.ts:155` (only if Option A and a real seed is needed in the API layer)
- `crates/argo-cli/src/cli/demo_seed/workflows*.rs` (if Option A — verify seed CLI populates workflows)
- Any sibling page with an analogous source-pill (verify via grep `MOCK` under `web/app/`)

## Out of scope

- The deeper `customer_research_followup` workflow + MCP/CU step kinds (separate brief: `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Sub-workflow picker using fixture (separate brief: `fix-workflow-builder-subworkflow-picker-uses-static-fixture`)
- `/workflows/build` 404 (separate brief: `fix-workflows-build-top-level-entry-point`)

## Effort

~20-50 LOC for Option B; ~100-200 LOC + seed work for Option A. ~30-90 min depending on choice.
