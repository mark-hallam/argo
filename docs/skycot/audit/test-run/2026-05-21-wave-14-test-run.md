# Wave-14 deep audit — workflow-builder "Test run" button

**Auditor**: claude (audit agent #1)
**Audited against**: `origin/main` `cb578888` (2026-05-21)
**Scope**: `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` → `handleTestRun` (current main, lines 698-706) + in-flight wiring PR #1884 (`claude-web-workflow-builder-live-execution-overlay`).
**Preceding waves**:
- Wave-6 (2026-05-21 morning) — softened the "All 5 steps passed" lie to `toast.message("Test run — preview build", { description: "Real run streams from the gateway in an upcoming release." })`.
- Wave-13 (2026-05-21 midday) — stripped internal `v1.x` / PR-ref identifier leaks from the same toast.

---

## TL;DR — readiness score 2 / 10

| Dimension                | On main (today) | In PR #1884 (IN_REVIEW) | Verdict                                   |
| ------------------------ | --------------- | ----------------------- | ----------------------------------------- |
| 1. Button UX             | Honest stub     | Wired (idle / running / done) | PR is good; not on main                 |
| 2. Mid-run progress      | None            | Yellow pulse → green check → red error (full) | PR is good; not on main                 |
| 3. Sample inputs         | None            | None — fixture-only            | **Gap** — even the PR doesn't ask user for input |
| 4. Output preview        | Toast           | Banner + collapsible tray   | PR is good; not on main                 |
| 5. Audit trail           | None            | None — mock route doesn't emit | **Gap** — test runs leave zero footprint |

**Demo-voiceover honesty (today's main)**: The button is dishonest if the voiceover claims it executes anything. The toast itself is honest. **The changelog page is dishonest** — `web/app/changelog/page.tsx:58` already advertises "live execution overlay" as shipped (line 58, the workflow-builder card). PR #1884 has not landed.

**Recommendation**: Either land PR #1884 before recording the demo, or scrub the changelog claim. Do not voice over "Test run" as a working feature against main as it stands.

---

## File-by-file findings

### 1. Current main — the stub itself (`build/page.tsx:698-706`)

```tsx
function handleTestRun() {
  // Honest demo stub — wave-6 action-fires audit flagged the prior
  // "All 5 steps passed" toast as a buyer-catchable lie. Wave-13
  // audit asked to drop the "v1.x" / internal-PR-ref leaks; this
  // copy stays neutral while keeping the not-yet-wired honesty.
  toast.message("Test run — preview build", {
    description: "Real run streams from the gateway in an upcoming release.",
  });
}
```

**Verdict**: The toast itself is honest. The button (lines 947-949) renders unchanged regardless of click state — no loading spinner, no can-cancel, no error path. A demo voiceover that says "and here's the run executing on the canvas" against current main would lie because the canvas does not animate.

### 2. PR #1884 — `claude-web-workflow-builder-live-execution-overlay` — IN_REVIEW, CI red, NOT on main

**PR state**: `OPEN`, `UNKNOWN` mergeability (needs rebase post-#1885 + #1886), all CI checks `FAILURE` (fmt, clippy, test on 3 OSes, loc-budget, prepared-statements, etc.). Per ledger row 891: "REBASED post-#1885 2026-05-20 — PR #1884 now MERGEABLE" — but conductor cluster-coordination comment from 2026-05-21 puts #1888 as the canonical `build/page.tsx` splitter; #1884 must rebase **after** #1888 lands.

**Files in PR** (none of these exist on main):
- `web/app/(app)/workflows/[verticalId]/[name]/build/useTestRun.ts` (+215 LOC) — driver hook with AbortController, NDJSON parser, per-step state folding, summary state.
- `web/app/(app)/workflows/[verticalId]/[name]/build/RunOverlay.tsx` (+379 LOC) — top-of-canvas summary banner (running / success / failure variants) + bottom collapsible "Run details" tray with per-step icon (pulse / check / X), duration, tokens, response/error rows.
- `web/app/(app)/workflows/[verticalId]/[name]/build/useTestRun.test.ts` (+139 LOC) — 4 vitest cases (NDJSON fold, error path, abort, transport-error).
- `web/app/api/workflows/[verticalId]/[name]/test-run/route.ts` (+212 LOC) — Next.js route that **tries** `${gatewayBase()}/api/v1/workflows/<v>/<n>/runs?stream=true` with bearer token; falls back to an in-process **mock NDJSON simulator** in non-prod. In `production` env it returns `502 gateway_unreachable`.
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (+184 / -12) — replaces the stub `handleTestRun` with a `useTestRun(...)` hook, drives three-state button (idle / Running… / Run again), syncs `stepStates` into ReactFlow node `data.runStatus` + `data.runError` so the canvas pulses yellow → checks green → goes red, recolors edges (green for both-success, yellow for inflight, red for downstream-failure), and mounts `<RunOverlay … />` over the canvas.

**Hook contract** (NDJSON wire shape, stable per the route file's comment so the gateway can swap in later):
```
{"type":"step:start","stepId":"s1","ts":...}
{"type":"step:end","stepId":"s1","ok":true,"durationMs":418,"tokens":312,"response":"…"}
{"type":"run:summary","ok":6,"fail":0,"durationMs":4218,"costUsd":0.014}
```

### 3. Gateway side — nothing exists, nothing claimed

`crates/argo-cli/src/handlers/gateway/routes_workflows.rs` and `routes_workflows_handlers.rs` own `GET`/`PUT` definition CRUD only (no `runs` endpoint). Grep across `crates/` for `workflow_test_run`, `workflows/.*runs`, `test_run` returns only A/B-test code (`AbTestRun`, completely unrelated). The PR's brief explicitly notes "gateway-side SSE endpoint follow-up needed" — there is no Rust-side stream in flight.

### 4. Sample-input collection — **dimension #3 has no answer anywhere**

Neither the stub nor PR #1884 prompts the user for sample input. The PR's mock simulator hard-codes fixture responses per step kind:
```ts
const SAMPLE_RESPONSES: Record<string, string> = {
  trigger: "Trigger fired — payload normalized.",
  skill_invoke: "Skill returned structured output. Confidence: 0.92. No PII detected.",
  …
};
```
A real test run needs to ask "what does the trigger payload look like?" (form values, webhook JSON, cron firing-time). The brief is silent on this. **This is the biggest UX gap for an n8n-parity moment** — n8n's brilliance is that you can paste a sample webhook payload and re-run it. SKYCOT has nothing.

### 5. Audit-trail emission — **dimension #5 has no answer anywhere**

The PR's route `web/app/api/workflows/[…]/test-run/route.ts` does not write an audit row. The mock simulator just streams events to the client and exits. No `gateway POST /api/v1/audit` call, no `audit_sink` analogue. A test run leaves zero footprint. This is wrong: a buyer-catchable lie at demo time is "and every test run is captured in the audit log" — which is **false today and stays false even with PR #1884 merged**.

The audit-emission expectation should be: every test run emits an `audit_row` with `verb=workflow.test_run`, the step list, the requesting principal, and a link back to the run-id so the audit page can show "Mark ran a test of post_listing 4 minutes ago."

---

## Detailed dimension scorecard

### Dimension 1 — Test-run button UX (loading state, can-cancel)

- **On main**: Button is static. Click → toast → returns to identical static state. No spinner, no disable, no cancel. **Score 1/5.**
- **In PR #1884**: Three-state button (Play "Test run" → spinning Loader2 "Running…" disabled → RotateCcw "Run again"). AbortController on unmount + on re-trigger. No explicit "Cancel" button while running — the hook can be cancelled by `reset()` but the UI doesn't expose it. **Score 4/5.**
- **Recommended improvement**: While running, the disabled "Running…" button should become a click-to-cancel "Cancel" button. Trivial: bind `onClick={resetTestRun}` and swap the label.

### Dimension 2 — Mid-run progress (yellow pulse → green check / red error)

- **On main**: No progress at all. **Score 0/5.**
- **In PR #1884**: Yellow pulse animation on running step's icon (`@keyframes run-pulse` 1.1s scale + opacity), green check on success, red X on error. Edges recolor: yellow inflight, green both-success, red downstream-failure. Mini-map node color is updated via the same `data.runStatus`. **Score 5/5 — this is exactly the n8n-parity moment the brief calls out.**

### Dimension 3 — Sample inputs (does the runner ask, or fixture-only?)

- **On main**: N/A. **Score 0/5.**
- **In PR #1884**: Fixture-only. The mock simulator emits canned `SAMPLE_RESPONSES` per step kind. The form payload sent to the route is `{ steps, failStepId }` — no user-supplied input. **Score 1/5 — this is the n8n competitive gap.**
- **Recommended follow-up brief**: `web-workflow-builder-test-run-sample-input-drawer` — when "Test run" is clicked the first time, open a drawer that asks for the trigger payload (JSON editor with a sensible default per `triggerKind`: `form` → empty form fields, `webhook` → blank JSON, `cron` → "fire as of <ISO>"). Cache the most-recent input as `localStorage[`test-run:${workflowId}:input`]` so subsequent runs are one-click.

### Dimension 4 — Output preview (where does test output land?)

- **On main**: Toast disappears in 4s. No output anywhere. **Score 0/5.**
- **In PR #1884**: Two surfaces:
  1. **Banner** at the top of the canvas: "Test run in progress…" / "Test run complete. 6/6 steps succeeded. TOTAL 4.2s · $0.014" / "Test run failed. …" — with running / success / failure visual variants and a dismiss-X when finished.
  2. **Collapsible tray** at the bottom of the canvas (`max-height: 50vh`, expand/collapse with chevron toggle): per-step row with status icon, name, duration, token count, status label, and a `<div class="run-tray-response">` mono-font response preview / `<div class="run-tray-error">` red error block.
- **Score 5/5.** Excellent surface choices — the banner gives the headline, the tray gives the forensics. Both surfaces are positioned correctly to avoid covering the canvas.

### Dimension 5 — Audit trail (does test run emit an audit row?)

- **On main**: No. **Score 0/5.**
- **In PR #1884**: No. The mock route streams events to the client and exits; no `audit_sink` write. Even when the gateway endpoint lands, the brief does not specify audit emission. **Score 0/5 — silent gap.**
- **Recommended fix to the brief**: Add acceptance criterion "Each test run emits one `audit_row` with `verb=workflow.test_run`, `actor=current_principal`, `tenant_id=principal.tenant`, `subject=<verticalId>/<name>`, `metadata={ steps: <count>, ok: <int>, fail: <int>, durationMs: <int>, costUsd: <float> }`." The Audit page can then surface test runs alongside real runs (perhaps with a `TEST` badge).

---

## Cluster coordination + sequence to ship

PR #1884 is one of 8 stale PRs all touching `build/page.tsx` (the "workflow-builder cluster"). Conductor decision on 2026-05-21:

1. **#1888** (node-palette-sidebar) lands first as the canonical `BuilderTypes`/`BuilderStepNode`/`BuilderStyles` splitter.
2. **#1884 rebases against the post-split state**, accepting the canonical BuilderTypes/StepNode/Styles files unchanged and re-applying only the unique additions:
   - The new files (`useTestRun.ts`, `useTestRun.test.ts`, `RunOverlay.tsx`, `api/.../test-run/route.ts`) drop in cleanly.
   - The page.tsx wiring (replacing the stub `handleTestRun`, wiring `useTestRun(...)` + `<RunOverlay …/>`, syncing `stepStates` into the node/edge styles) must land against the post-split `BuilderStepNode` so the `runStatus` / `runError` props on `data` are read in the right place.

If the worker has exited, this PR is eligible for stale-IN_REVIEW demote on the next conductor tick (per the cluster-coordination comment). Recovery brief would be `claude-rebase-pr-1884-live-execution-overlay` — but it should only be authored once #1888 lands.

---

## Cross-claims that should also be audited

- `web/app/changelog/page.tsx:58` advertises "live execution overlay" as shipped. **This is currently false** — must either land PR #1884 before the next prospect lands on `/changelog`, or remove that claim from the workflow-builder card copy. This is the buyer-catchable lie that survived Wave-6 + Wave-13 because those waves audited the toast surface, not the marketing copy.
- The brief itself misses two of the five dimensions (sample input + audit emission). Even if #1884 lands as-authored, those gaps remain. Follow-up briefs needed:
  - `web-workflow-builder-test-run-sample-input-drawer` (dimension #3)
  - `web-workflow-builder-test-run-audit-emission` (dimension #5) — gateway-side, requires the gateway endpoint to exist first.
- Gateway-side endpoint `POST /api/v1/workflows/<v>/<n>/runs?stream=true` is unauthored. Recommended brief id: `gateway-workflow-test-run-ndjson-stream` — Rust handler that drives the runtime executor in `argo-agent-runner`, streams `step:start` / `step:end` / `run:summary` events back to the Next.js route per the wire shape the route already speaks.

---

## Recommended next actions (priority order)

1. **Today** — Scrub the `/changelog` "live execution overlay" claim until #1884 ships. One-line edit to `web/app/changelog/page.tsx:58`.
2. **This week** — Land #1888 (the splitter), then rebase + land #1884. After landing, the front-end side is honest down to the mock-simulator level.
3. **Next sprint** — Author + land the gateway brief so the front end stops needing the mock fallback in dev. Author + land the sample-input brief so the test run becomes useful (not just visually impressive). Author + land the audit-emission brief so the runs leave an auditable footprint.
4. **Demo-day discipline** — Until #1884 lands, the voiceover should say "and here's where the test-run animation will play once we wire the gateway stream" rather than "watch the steps execute live." This is the same shape as the Wave-6 toast honesty: surface the action, do not claim completion of work that didn't happen.
