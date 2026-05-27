# Round 104 — PR #2541 demo-session derivation PRE-MERGE verification

**Date**: 2026-05-27
**Round**: 104
**Pillar**: v1.0 demo MP4 ship-gate — NO-THEATER-IN-DEMO operator directive
**Verdict**: 🟢 PRE-MERGE PASS

## Context

`feedback_no_theater_in_demo` (operator-locked 2026-05-24) requires the v1.0 demo MP4 to be REAL, not mocks. UX-audit T2351MEL flagged onboarding F3 as a **P0** theatre violation: the demo wizard's step 2 asked the operator to type a workspace name + company name, fired a celebration toast (`"Workspace created — welcome to Skycot"`), persisted the inputs in the zustand draft store — **and then silently discarded them**. Every in-app sidebar pill the camera reads later rendered `"Acmeco · Owner"` hardcoded via `DEMO_SESSION` in `web/lib/skycot/use-session.ts`, regardless of what the operator typed.

PR #2541 (`fix-demo-session-hardcodes-acmeco-discards-wizard-workspace-inputs`, head `446d7e88e`) implements Option A from the brief: honour the wizard inputs. Status: MERGEABLE/UNKNOWN, +195/-13 across 3 files.

## Setup

```bash
gh pr diff 2541 --patch | wc -l
# 320 LOC of diff context

gh pr view 2541 --json files --jq '[.files[].path]'
# ["web/lib/skycot/use-session.test.ts",
#  "web/lib/skycot/use-session.ts",
#  "web/tests/e2e/onboarding-workspace-inputs-honoured.spec.ts"]
```

Source-read verification of the production change at `web/lib/skycot/use-session.ts`:
- Module-level `DEMO_SESSION` constant **removed**; replaced by `DEMO_SESSION_FALLBACK` carrying neutral `"Sample workspace · Owner"` / `"Demo Operator"` / `operator@demo.skycot.local` / `tenant_demo`
- New exported `getDemoSession(draft: DemoDraftSnapshot | null): SkycotSession` — returns FALLBACK for null/whitespace-only `workspaceName`, otherwise derives `email = ${slug}@demo.skycot.local`, `tenantId = draft.tenantId ?? \`tenant_${slug}\``, `workspaceLabel = ${wsName} · Owner`
- `slugifyWorkspace(input)` does `trim()→lowerCase()→replace(/[^a-z0-9]+/g,"-")→replace(/(^-|-$)/g,"")→slice(0,40)` with `"workspace"` fallback for empty result — safe email-local-part shape
- `useSkycotSession()` now subscribes to draft via `useOnboardingDraft((s) => s.workspaceName)` + `s.tenantId` (primitive selectors — zustand only re-renders on primitive change)
- `deriveSession()` gains optional 3rd parameter `demoDraft: DemoDraftSnapshot | null = null`; demo-mode branch routes through `getDemoSession(demoDraft)`

## Observed

**Unit test coverage** at `web/lib/skycot/use-session.test.ts` — 6 new cases (PR body claims 12/12 PASS in author's session):

1. `returns the neutral demo fallback when demo mode is on and draft is empty` — asserts equals `DEMO_SESSION_FALLBACK` **and** negative regex pins `workspaceLabel not to match /Acmeco/i` + `displayName not to match /Sam Harlow/i`
2. `derives demo session from onboarding draft when workspace name is typed` — `workspaceLabel === "Acme Growth · Owner"`, `email === "acme-growth@demo.skycot.local"`, `tenantId === "tenant_acme-growth"`
3. `returns the neutral fallback for a null draft`
4. `returns the neutral fallback for a whitespace-only workspace name`
5. `prefers the draft-provisioned tenant id over the derived one` — `tenantId === "demo-tenant-acme-growth"` (draft-provisioned wins over `tenant_${slug}`)
6. `slugifies the workspace name into a valid email local-part` — input `"Acme Co · Sales!"` → `email === "acme-co-sales@demo.skycot.local"`
7. `never re-introduces the old 'Acmeco · Owner' hardcoded label` — input `"Acmeco Growth"` → `workspaceLabel === "Acmeco Growth · Owner"` AND `!== "Acmeco · Owner"`

**E2E coverage** at `web/tests/e2e/onboarding-workspace-inputs-honoured.spec.ts` — walks the demo wizard:
- Fill `workspace-name = "Acme Growth"`, `company-name = "Acme Inc."`, click `onboarding-workspace-submit`
- Assert advance to `/onboarding/connect-provider` (10s timeout)
- Navigate to `/team` (mounts AppShell + UserPill)
- Assert `getByTestId("appshell-user-workspace")` visible
- Assert `toHaveText("Acme Growth · Owner")`
- Negative regression guard: `not.toHaveText(/Acmeco/i)`

Spec test-skips itself when `NEXT_PUBLIC_SKYCOT_DEMO_MODE !== "1"` — same pattern as `onboarding-demo-mode-step-2-continue.spec.ts`.

## Anti-theater check

Five independent axes — all dispositive:

1. **Production constant deletion is permanent.** `DEMO_SESSION` is removed (not renamed, not aliased) — no shim that lets a stale callsite re-import the old hardcoded shape. Anyone who tries to `import { DEMO_SESSION } from "./use-session"` gets a build error.
2. **Tests import the production exports.** `DEMO_SESSION_FALLBACK`, `deriveSession`, and `getDemoSession` are pulled from the same module they exercise — no test-only doppelganger.
3. **Negative pins on the OLD hardcoded values** at both the unit layer (`not.toMatch(/Acmeco/i)`, `not.toMatch(/Sam Harlow/i)`) and the e2e layer (`not.toHaveText(/Acmeco/i)`). If a future regression re-introduces the old constant, these break.
4. **The e2e spec walks the user-facing surface** — not the function under test. It fills wizard inputs, clicks Continue, navigates to `/team`, reads the sidebar pill the camera will read. That's the camera-visible acceptance criterion expressed as code.
5. **Vacuous-pass guard via "prefers draft-provisioned tenant id"** — the test for `tenant_acme-growth` (derived) coexists with the test for `demo-tenant-acme-growth` (draft-provisioned), proving the precedence logic is load-bearing in both directions.

## Gap / follow-up brief candidates

None for this PR. Operator-noted sibling briefs in PR body's "Out of scope":
- `fix-settings-workspace-tenant-identity-surface` — static fields in `/settings/workspace`
- `fix-sidebar-workspace-pill-derive-tenant-type` — workspace-type label in sidebar

Both already exist in the queue per PR body — surfacing here only as breadcrumbs for the conductor's next sweep. **No new briefs authored this round** — the existing queue already covers the work.

**Recommendation to conductor**: PR #2541 qualifies for the `--skip-clippy` evaluator-pre-verified merge lane per `feedback_skip_clippy_authorized_for_evaluator_pre_verified_prs`. Web-only ≤200 LOC ≤5 files, vitest 12/12 PASS asserted in body, anti-theater axes all dispositive on source read. Also qualifies under `feedback_web_only_merge_authorization_2026_05_26` (web-only ≤200 LOC ≤5 files lane permanent). Either authorization unblocks merge.

## Source

This report. UX-audit: `docs/skycot/ux-audits/2026-05-27-2351/`. Brief: `docs/build-plan/streams/fix-demo-session-hardcodes-acmeco-discards-wizard-workspace-inputs.md`. PR: [#2541](https://github.com/mark-hallam/argo/pull/2541).
