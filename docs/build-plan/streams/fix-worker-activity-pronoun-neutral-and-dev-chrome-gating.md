# fix-worker-activity-pronoun-neutral-and-dev-chrome-gating

**Owner**: claude.
**Authored against**: origin/main `5c4abb23e1f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-activity-pronoun-neutral-and-dev-chrome-gating`.
**Source**: e2e-ux-audit (scene-1-and-2-claim-1-and-2) findings F10 + F11 + F12 — `docs/skycot/ux-audits/2026-05-26-0820/scene-1-and-2-claim-1-and-2.md`.

🟢 **P2 — Three small worker-detail polish gaps bundled into one PR: hardcoded "she/her" pronouns in empty-state copy; console-warn from overflow menu visible in dev tools; data-source chip ("GATEWAY"/"MOCK-OFFLINE") visible next to "Team" heading during demo recording.**

## Root cause

Three independent micro-defects on the worker detail / team pages, all small enough to bundle:

**F10 — pronoun-coded empty state**:
`web/app/(app)/workers/[workerId]/activity/page.tsx:201-205` renders:
> "{workerName} hasn't taken any actions yet. Once **she's** working, you'll see **her** activity here."

Hardcoded "she/her". Jamal (male persona) reads as misgendered. Any worker with they/them pronouns will too. P2 because Scene 2 doesn't dwell on this view, but visible on any fresh-hire's first detail-page visit.

**F11 — dev-only console.warn leaks into recording**:
`web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:122-123`:
```ts
console.warn(`[E4-a] worker hero action '${action}' pending wiring (E3-b/E4-f)`);
```

If the operator has dev tools open during recording (per memory `project_v1_demo_mp4_ship_gate` they're told to close them — but sometimes don't), the warning appears in the console panel. Cosmetic, but breaks the "production-ready" feel.

**F12 — data-source chip visible during recording**:
`web/app/(app)/team/page.tsx:316-329` renders a chip with `data-source` value (e.g. `gateway` / `mock-bypass` / `mock-offline`) next to the "Team" heading. Code guards with `NODE_ENV === "production"` — but the demo is recorded in dev, so the chip is visible on camera. Reads as engineering plumbing the buyer doesn't need to see.

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F10 [P2] — Activity-tab empty state uses "she/her" without checking worker pronouns
- scene-1-and-2-claim-1-and-2 F11 [P2] — Worker overflow menu emits a console.warn on click in dev
- scene-1-and-2-claim-1-and-2 F12 [P2] — Team-data-source chip ("GATEWAY") visible next to "Team" heading in dev

## Fix recipe

**F10**:
1. Touch `web/app/(app)/workers/[workerId]/activity/page.tsx:201-205`.
2. Replace with pronoun-neutral copy:
   > "{workerName} hasn't taken any actions yet. Once they start working, you'll see their activity here."
3. Audit sibling pages for the same pattern: `web/app/(app)/workers/[workerId]/*/page.tsx` — grep for `\bshe\b|\bher\b|\bhim\b|\bhis\b` in empty-state strings; rewrite to they/them throughout.
4. Add an ESLint rule (or a simple test) at `web/tests/copy-pronoun-neutral.spec.ts` that scans `web/app/(app)/workers/**/*.tsx` for the disallowed pronouns in JSX text nodes — keeps the regression risk low. Use a lightweight grep-based check rather than a full AST walker.

**F11**:
1. Touch `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx:122-123`.
2. Gate the console.warn:
   ```ts
   if (process.env.NODE_ENV !== "production") {
     console.warn(`[E4-a] worker hero action '${action}' pending wiring (E3-b/E4-f)`);
   }
   ```
   But the proper fix: **wire the missing actions** (E3-b/E4-f are the brief references for this work) rather than guard the warn. If wiring is out of scope, remove the warn entirely — silent no-op is better than spamming dev console.
3. While there, grep `web/app/(app)/` and `web/components/skycot/` for other un-gated `console.warn` + `console.log` calls in component code; gate or remove.

**F12**:
1. Touch `web/app/(app)/team/page.tsx:316-329` and the `<DataSourceChip>` component (or wherever the data-source eyebrow lives).
2. Tighten the visibility guard:
   ```ts
   const showDataSourceChip =
     process.env.NODE_ENV !== "production" &&
     process.env.NEXT_PUBLIC_ARGO_DEMO_MODE !== "1";
   ```
   The chip stays visible during normal dev for debugging, hides during demo recording when the flag is set, hides in prod always.
3. Apply the same guard to any other dev-only chip across pages (`/audit`, `/approvals`, `/workflows`). Grep for `data-source` chip / pill renderers and unify the guard.

## Acceptance criteria

**F10**:
- [ ] `web/app/(app)/workers/[workerId]/activity/page.tsx` empty-state uses "they/their" (no "she/her").
- [ ] Sibling worker-detail pages (`memory`, `skills`, `trust`, etc.) audited for the same pattern; all rewritten.
- [ ] New copy test (or eslint rule) catches future regressions; passes locally.

**F11**:
- [ ] `console.warn` at `WorkerLayoutShell.tsx:122-123` either gated on `NODE_ENV !== "production"` OR removed entirely.
- [ ] `grep -rnE "console\\.(warn|log)" web/app/\\(app\\)/ web/components/skycot/` shows no un-gated console calls in component code (only in `lib/` debug helpers).

**F12**:
- [ ] `/team` page chrome does NOT render the data-source chip when `NEXT_PUBLIC_ARGO_DEMO_MODE=1`.
- [ ] Chip still renders in normal dev (no demo flag) so developers can debug.
- [ ] Same guard applied across `/audit`, `/approvals`, `/workflows` data-source chips.
- [ ] Visual smoke: `NEXT_PUBLIC_ARGO_DEMO_MODE=1 pnpm dev`, visit `/team` → no "GATEWAY" chip visible.

## May-touch

- `web/app/(app)/workers/[workerId]/activity/page.tsx` (and sibling pages)
- `web/app/(app)/workers/[workerId]/WorkerLayoutShell.tsx` (console.warn gate)
- `web/app/(app)/team/page.tsx` (data-source chip guard)
- Other data-source-chip renderers across `web/app/(app)/`
- `web/tests/copy-pronoun-neutral.spec.ts` (NEW)

## Out of scope

- Worker-level pronoun field on the data model (separate brief if operator wants per-worker `pronouns: "he/him" | "she/her" | "they/them"` — for now, default to they/them everywhere)
- Removing all `console.log` from server-side handlers (different ownership)
- Replacing the data-source debugging mechanism entirely (it stays; just hidden during demo)

## Effort

~40-80 LOC + 1 test. ~1 hour. Cheap, ship as one bundled PR.
