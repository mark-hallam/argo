# fix-demo-session-hardcodes-acmeco-discards-wizard-workspace-inputs

**Owner:** claude
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-demo-session-hardcodes-acmeco-discards-wizard-workspace-inputs`
**Source:** ux-audit-2026-05-27-2351

🔴 **P0 DEMO-BLOCKER — Onboarding step 2 "Workspace created" toast contradicts the sidebar identity pill the camera reads 30s later.**

## Root cause

`web/lib/skycot/use-session.ts:39-58` defines a module-level constant `DEMO_SESSION` and `deriveSession()` returns it verbatim whenever `isDemoMode()` is true:

```ts
export const DEMO_SESSION: SkycotSession = {
  status: "authenticated",
  isDemoMode: true,
  email: "sam@acmeco.com",
  displayName: "Sam Harlow",
  tenantId: "tenant_acmeco",
  workspaceLabel: "Acmeco · Owner",
  role: "owner",
};

export function deriveSession(data, status): SkycotSession {
  if (isDemoMode()) return DEMO_SESSION;
  // ...
}
```

Meanwhile the onboarding wizard's step 2 collects workspace + company name (placeholder "Acme Growth" / "Acme Inc."), fires a celebration toast "Workspace created — welcome to Skycot" on submit, and stores the answers in the Zustand draft store via `web/lib/onboarding/use-provision.ts:56-63` (which itself short-circuits `provision()` to a synthetic id under demo mode). The draft store is never consulted by `deriveSession()`. The result: investor types "Acme Growth / Acme Inc.", accepts the toast, completes the wizard, and the sidebar / user-pill / "Workspace · X" chip on every in-app surface reads "Sam Harlow · Acmeco · Owner". The wizard step is camera-visible theatre.

This is distinct from `fix-settings-workspace-tenant-identity-surface` (which addresses static fields on `/settings/workspace`) and `fix-sidebar-workspace-pill-derive-tenant-type` (which addresses workspace-type label). The defect here is the `DEMO_SESSION` constant itself overriding the actual onboarding inputs.

Per `feedback_no_theater_in_demo` (operator-locked 2026-05-24) the demo MP4 must not have function-side mocks visible on camera. The wizard ASKS for workspace name and DISCARDS it; that is theater the camera sees.

## Findings this brief closes

- `onboarding-hire-an-ai-teammate.md` F3 [P0] — Demo-mode hardcodes the workspace identity to "Sam Harlow · Acmeco · Owner"; the wizard's typed inputs are discarded the moment the user leaves the wizard

## Fix recipe

Three operator-decisional options, pick one:

### Option A (recommended — honour the wizard inputs)

1. In `web/lib/skycot/use-session.ts`, replace the `DEMO_SESSION` constant with a function `getDemoSession()` that reads the onboarding draft store (`web/lib/onboarding/draft.ts` or equivalent Zustand store) and returns a session derived from typed values:
   - `displayName` ← draft.contactName ?? "Demo Operator"
   - `workspaceLabel` ← `${draft.workspaceName} · Owner` (fallback: "Sample workspace · Owner")
   - `email` ← `${slug(draft.workspaceName)}@demo.skycot.local` (synthetic but consistent)
   - `tenantId` ← stable per-session UUID (`crypto.randomUUID()` once at first read, persist in sessionStorage)
2. Wire `useSkycotSession()` to subscribe to the draft store so the sidebar pill updates live when the wizard step 2 is submitted.

### Option B (lower-risk — hide the wizard step 2 inputs in demo mode)

1. Modify `web/app/(setup)/onboarding/workspace/page.tsx` to render a read-only "Sample workspace — we're using a starter workspace for the demo" panel instead of inputs when `isDemoMode()` is true.
2. Suppress the "Workspace created" toast in demo mode.
3. Keep `DEMO_SESSION` as-is. The sidebar label still reads "Acmeco · Owner" but the wizard no longer asks for inputs it discards.

### Option C (rewrite copy + accept the dressing label)

1. Rename `workspaceLabel` to "Sample workspace · Owner" so at least the label doesn't pretend to be a real company name the operator never typed.
2. Same wizard-input suppression as Option B.

Option A is the function-side fix. Option B is the dressing-side patch (works for POC demo MP4). Operator decides.

## Acceptance criteria

- [ ] Sidebar identity pill no longer reads "Acmeco · Owner" after completing the onboarding wizard with custom workspace + company inputs.
- [ ] Under Option A: typing "Acme Growth / Acme Inc." in step 2 surfaces "Acme Growth · Owner" (or similar) in the sidebar pill on every in-app page.
- [ ] Under Option B/C: the wizard step 2 no longer collects inputs that will be discarded.
- [ ] `fix-no-theater-in-demo` rule honoured — no camera-visible contradiction between wizard toast and post-wizard chrome.
- [ ] Playwright spec `web/playwright/onboarding/workspace-inputs-honoured.spec.ts` (Option A) or `web/playwright/onboarding/workspace-step-suppressed-in-demo.spec.ts` (Option B) pins the chosen path.
- [ ] PR body cites finding F3 + the chosen option.

## May-touch

- `web/lib/skycot/use-session.ts`
- `web/lib/onboarding/use-provision.ts`
- `web/lib/onboarding/draft.ts` (or equivalent)
- `web/app/(setup)/onboarding/workspace/page.tsx`
- `web/components/skycot/UserPill.tsx` (verify pill consumes session correctly)
- `web/playwright/onboarding/workspace-*.spec.ts` (NEW)

## Out of scope

- Settings → Workspace tenant identity fields (covered by `fix-settings-workspace-tenant-identity-surface`)
- Sidebar workspace-type label (covered by `fix-sidebar-workspace-pill-derive-tenant-type`)
- Real multi-tenant signup flow with workspace creation against the gateway (post-v1)
- Other `DEMO_SESSION` fields (email, role) — keep stable

## Effort

S to M — ~80-200 LOC depending on Option. ~2-4 hours.
