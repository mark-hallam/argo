# Wave-8 Audit #3 — Frozen-worker behavior across surfaces

**Date:** 2026-05-21
**Scope:** `web/` only (read-only review)
**Question:** When a worker is frozen, does EVERY surface that mentions them show the frozen state consistently?
**Reference PR:** #1878 — shipped FreezeBanner on /workers/[id]/* layout

## TL;DR verdict

**Frozen state is shipped as a single-surface feature, not a tenant-wide signal.**
The freeze infra is implemented exactly once — at the worker-profile layout.
Every other surface (Team grid, Cohorts, Audit, Approvals inbox, Workflows, Notifications bell, sub-tab pages, command palette, sidebar nav) is **completely freeze-unaware**. The `WorkerStatus` union does not include `"frozen"`. Sample data carries no frozen rows. There is no `worker-frozen` notification kind.

Of 8 surfaces audited: **1 handles frozen correctly, 0 partial, 7 not-at-all.**

---

## Surface-by-surface findings

### 1. /workers/[id]/* (worker profile + all sub-tabs) — CORRECT

- Layout `app/(app)/workers/[workerId]/layout.tsx` (lines 17-38) is a server component that calls `fetchFreezeState(workerId, session?.gatewayToken)`.
- `WorkerLayoutShell.tsx` (line 71) renders `<FreezeBanner state={freezeState} />` above the hero. Banner is red, has `role="alert"`, shows reason + frozen_by + frozen_at.
- The banner appears on every sub-tab (overview/activity/memory/skills/trust/settings/evals/responsibilities) because it's at the layout level — confirmed by directory walk.
- FreezeButton (lines 38-117) flips label to "Unfreeze worker" when `isFrozen={true}`.
- Files: `app/(app)/workers/[workerId]/{layout.tsx,WorkerLayoutShell.tsx,FreezeBanner.tsx,FreezeButton.tsx,fetchFreezeState.ts}`.

**Status: ✅ shipped + consistent.**

### 2. /team grid (Mariah's card) — NOT HANDLED

- `WorkerStatus` (`web/lib/types/worker.ts:15-20`) = `"working" | "waiting" | "idle" | "blocked" | "paused"`. **Frozen is not a status.**
- `TeammateCard.tsx` (`components/skycot/TeammateCard.tsx`) renders `STATUS_TONES` + `STATUS_LABELS` — no frozen branch.
- `TeammateCardContextMenu.tsx` (lines 45, `PAUSABLE_STATUSES`) only knows pause/resume. No Freeze action in card right-click menu.
- `compareWorkersForGrid` (`worker.ts:140`) has no priority bucket for frozen.
- A frozen Mariah would render in the grid identically to her not-frozen self. **No pill, no dimmer, no icon, no border, no sort priority.**

**Status: ❌ not handled at all.**

### 3. /cohorts page + drawer — NOT HANDLED

- `lib/sample-cohorts.ts` lines 1-160 carry no `frozen_members` aggregate, no `frozen` boolean per member.
- `CohortCard.tsx` and `CohortDetailDrawer.tsx` contain zero references to freeze/frozen (grep clean).
- A cohort containing 5 workers where 2 are frozen surfaces as 5 active members.

**Status: ❌ not handled at all.**

### 4. /audit feed — NOT HANDLED

- `lib/sample-audit.ts` SEEDS array (60+ rows) has no `action_type: "worker_frozen"` row.
- `lib/types/audit.ts` taxonomy doesn't include a freeze verb.
- The freeze gateway endpoint (`POST /api/v1/workers/<id>/freeze`) is presumed to write to the audit log server-side (the FreezeButton modal copy says "Reason (required, recorded in audit)") but the frontend has no read path for it.
- Action verbs visible in sample data include `policy_refusal`, `escalation`, `spend_approval_asked`, `skill_learned` — no `worker_frozen` or `worker_unfrozen`.

**Status: ❌ event type not modeled in frontend.**

### 5. / Approvals inbox — NOT HANDLED

- `app/(app)/page.tsx` (ApprovalsInboxPage) reads `useApprovals()` → `ApprovalItem[]`.
- `lib/sample-approvals.ts` carries no `worker_frozen` boolean on items, no "blocked because owner frozen" treatment.
- `ApprovalCard.tsx` / `BatchGroupBlock.tsx` (referenced in page.tsx) have no freeze awareness.
- A frozen worker with pending approvals: those approvals continue to render normally — the operator has no indicator that approving them would be a no-op or held until unfreeze. No "FROZEN" pill on the card. No hide. No escalation badge.
- **This is the highest-impact gap:** an operator could approve an action that the runtime will then refuse to execute because the owner is frozen, with no UI signal at the decision point.

**Status: ❌ not handled at all. (severity: P1)**

### 6. /workflows index + detail — NOT HANDLED

- `app/(app)/workflows/page.tsx` `SEED_WORKFLOWS` (lines 22-100) lists each workflow with `lastEditor` (Mariah, Priya, etc.) — no `ownerFrozen` flag, no blocked indicator.
- Workflow detail at `app/(app)/workflows/[verticalId]/[name]/page.tsx` was not opened (out of time-box) but grep confirms no freeze refs.

**Status: ❌ not handled at all.**

### 7. Notifications bell — NOT HANDLED

- `lib/sample-notifications.ts` `NotificationKind` (lines 10-18) =
  `budget-approaching | budget-exceeded | skill-promoted | skill-drift-detected | oauth-needs-reauth | duty-output-ready | autonomy-promotion-suggested | incident`.
- No `worker-frozen` or `worker-unfrozen` notification kind. Freeze action fires no in-app notification.
- This means an operator on another tab does not get a bell-ping when a peer admin freezes a worker — they discover it next time they open that worker's profile.

**Status: ❌ no notification kind defined.**

### 8. Worker hero status pill (on profile page) — PARTIAL OVERLAP

- The freeze banner sits above the WorkerHero, so it's visible.
- But the hero's own `StatusPill` (driven by `WorkerStatus`) still shows "Working" / "Waiting" / "Idle" alongside the red frozen banner. This is contradictory UX — a frozen worker might pill "Working" while the banner says no sessions will start.
- The pill should either flip to "Frozen" (requires extending `WorkerStatus`) or be visually suppressed while frozen.

**Status: ⚠️ banner covers it, but the pill itself doesn't reflect the frozen state. Minor inconsistency.**

---

## Top 5 places where frozen should be visible but isn't

| Rank | Surface | Impact | What's missing |
| ---- | ------- | ------ | -------------- |
| 1 | **Approvals inbox** (every pending Mariah card) | P1 — operator can approve a no-op | FROZEN pill on `ApprovalCard`, or auto-hide / escalation route |
| 2 | **Team grid** (`TeammateCard`) | P1 — primary discovery surface | Frozen status as a 6th `WorkerStatus`, blue-snowflake border treatment, frozen sort bucket |
| 3 | **Notifications bell** | P2 — admin coordination | `worker-frozen` + `worker-unfrozen` NotificationKind |
| 4 | **Audit feed** | P2 — forensic completeness | `worker_frozen` / `worker_unfrozen` action types in sample-audit + types/audit |
| 5 | **Workflows** (where Mariah is `lastEditor`) | P3 — secondary | Block / warn indicator on workflow card when last-editor or owner is frozen |

## Root cause

Frozen was added as a freeze-record table at the gateway (`POST /api/v1/workers/<id>/freeze`) plus a UI banner at the worker-profile layout. **The frozen attribute was never promoted to a first-class field on the `Worker` domain type** (`lib/types/worker.ts`). Because every other surface reads `Worker` (or doesn't read workers at all — workflows, notifications), they have no knob to display frozen state.

Two coherent fixes:
1. **Cheap:** extend `WorkerStatus` with `"frozen"` and have the gateway team-list endpoint set status to "frozen" when a worker has an open freeze record. Cascades to ~80% of the surface gap via existing status-pill code paths.
2. **Right:** add `frozenAt?: string`, `frozenBy?: string`, `frozenReason?: string` as optional fields on `Worker`; let each surface decide its own treatment (pill, border, hide, escalate). More work, but lets the Approvals inbox separate "frozen" from a status — an approval can be pending AND the owner frozen.

PR #1878 was correct in scope ("banner on every worker tab") but the audit reveals freeze is currently a leaky, single-screen concept rather than a tenant-wide worker attribute.

## Verdict

**Inconsistent.** PR #1878 ships exactly what it promised — the freeze banner is on every worker-profile sub-tab. But the broader question "does every surface that mentions a frozen worker show the frozen state?" is **no** for 7 of 8 surfaces. The most material gap is the Approvals inbox, where the operator's primary decision surface gives no signal that the owner of a pending action is frozen.

**Recommendation:** Author a follow-up brief (`mission-frozen-worker-cross-surface-propagation`) covering surfaces 1-5 above. Option 1 above (add `"frozen"` to `WorkerStatus`) is the 80% solution and unlocks team grid + cohort + worker-hero in one stroke. Approvals + notifications still need their own fields.
