# Round 105 — PR #2541 demo-session derivation POST-MERGE verification

- **Date**: 2026-05-27 (UTC)
- **Round**: 105
- **Pillar/Claim**: NO THEATER IN DEMO (feedback_no_theater_in_demo) + camera-visible v1.0 demo MP4 gate
- **Verdict**: 🟢 PASS (P0 closed)

## Setup

PR #2541 (`fix-demo-session-hardcodes-acmeco-discards-wizard-workspace-inputs`) was PRE-MERGE PASS'd in round 104. It merged to main at `2026-05-27T16:10:43Z` by mark-hallam (~17 minutes before this round fired) as merge commit `a838a489762c33ef22f9adbd5a9f4d783590255f`. Cold-rerun the unit suite against current `origin/main` and confirm the production code path matches what round 104 verified.

Commands:

```bash
git show --stat a838a4897              # merge commit summary
git show HEAD:web/lib/skycot/use-session.ts | head -120
grep -rn "Acmeco\|Sam Harlow" web/lib web/app web/components \
  | grep -v "\.test\.\|\.spec\."
pnpm test lib/skycot/use-session       # cold unit run
```

## Observed

**Merge commit shape** matches the round-104 PRE-MERGE inventory exactly: 3 files (`web/lib/skycot/use-session.ts`, `web/lib/skycot/use-session.test.ts`, `web/tests/e2e/onboarding-workspace-inputs-honoured.spec.ts`), web-only, +195/-13.

**Production code (`web/lib/skycot/use-session.ts`)** confirms all the load-bearing edits actually landed:

- The `DEMO_SESSION` constant is gone (no `export const DEMO_SESSION` in the file).
- `DEMO_SESSION_FALLBACK` is the new neutral identity (`displayName: "Demo Operator"`, `workspaceLabel: "Sample workspace · Owner"`, `tenantId: "tenant_demo"`).
- `getDemoSession(draft)` is a new exported function that branches on `draft.workspaceName.trim() === ""` → `DEMO_SESSION_FALLBACK`; otherwise returns a session whose `workspaceLabel = \`${wsName} · Owner\`` with `email = \`${slug}@demo.skycot.local\`` and `tenantId = draft.tenantId ?? \`tenant_${slug}\``.
- `useSkycotSession()` subscribes to `useOnboardingDraft((s) => s.workspaceName)` and `useOnboardingDraft((s) => s.tenantId)` as primitive selectors, then passes the snapshot into `deriveSession(...)`.
- `deriveSession(data, status, demoDraft)` short-circuits to `getDemoSession(demoDraft)` when `isDemoMode()`.

**Negative-pin grep (production, excluding `.test.` / `.spec.`)** found `Acmeco` / `Sam Harlow` references in:

- `web/lib/skycot/use-session.ts:23` — doc comment explaining the removed identity (intentional history note).
- `web/lib/skycot/demo-mode.ts:5` — doc comment.
- `web/components/skycot/UserPill.tsx:18` — doc comment.
- `web/components/skycot/DemoBanner.tsx:12` — doc comment.
- `web/app/(app)/settings/members/members-data.tsx:47` — `name: "Sam Harlow"` in sample members list.
- `web/components/skycot/CommandPalette.tsx:307` — `title: "Sam Harlow"` in command-palette results.

The brief's may-touch was explicitly **session view-model only**; the two remaining live references (members + command-palette sample data) are out-of-scope sibling surfaces that the brief author called out ("Out of scope (covered by sibling briefs)" in the merge-commit description). The sidebar/userpill / `/team` empty-state path that the demo MP4 camera captures no longer reads "Acmeco" or "Sam Harlow" from `useSkycotSession()`.

**Unit suite cold run**:

```
RUN  v3.2.4 ~/code/argo/web
 ✓ lib/skycot/use-session.test.ts (12 tests) 3ms
 Test Files  1 passed (1)
      Tests  12 passed (12)
   Duration  600ms
```

12/12 PASS. Negative pins inside the test (`.not.toMatch(/Acmeco/i)` + `.not.toMatch(/Sam Harlow/i)`) are confirmed at lines 33-34, and the "never re-introduces the old 'Acmeco · Owner' hardcoded label" test (line 124) double-pins the regression in the inverse direction (`getDemoSession({ workspaceName: "Acmeco Growth" })` returns `workspaceLabel === "Acmeco Growth · Owner"`, NOT `"Acmeco · Owner"`).

## Anti-theater check

- **Dispositive deletion**: The old `DEMO_SESSION` constant is not just renamed or shadowed — `grep -n "export const DEMO_SESSION " web/lib/skycot/use-session.ts` returns no matches (only `DEMO_SESSION_FALLBACK`). If a future regression re-introduced the constant, the test that asserts `.not.toMatch(/Acmeco/i)` on a no-draft state would still pass (because the fallback is neutral), but the e2e walk that types "Acme Growth" and reads back the pill would fail.
- **e2e provides the camera-visible pin**: `web/tests/e2e/onboarding-workspace-inputs-honoured.spec.ts` is the load-bearing assertion — it walks the demo wizard with operator-typed inputs and asserts `appshell-user-workspace` reads `"Acme Growth · Owner"`. Without that, the unit tests alone could be fooled by a no-render bug.
- **Both directions of `tenantId` precedence** are tested (`draft.tenantId` wins when provided; falls back to `\`tenant_${slug}\`` when null) — the round-37 vacuous-pass class of bug (single-side test) is closed.
- **No test self-rewrite**: the test file imports `DEMO_SESSION_FALLBACK` + `getDemoSession` directly (not via re-export), so deleting either export would surface as a compile error first.

## Gap / follow-up brief candidates

No new briefs recommended this round. The 2 remaining "Sam Harlow" production references (members-data + command-palette) are **already covered** by named sibling briefs (`fix-settings-workspace-tenant-identity-surface` + `fix-sidebar-workspace-pill-derive-tenant-type` are mentioned in the merge-commit description; conductor's queue should be checked for additional `fix-*-sam-hallam-*` / `fix-*-acmeco-*` companion briefs if those camera-visible surfaces are also in-scope for the demo MP4 capture).

## Closure note

The demo MP4 ship-gate path (operator-only artifact, per CLAUDE.md "Remaining for v1.0 launch" item (d)) just got one camera-visible theater violation closed. This is the **third end-to-end pilot 2026-05-27 inner-loop closure** in this round-cluster:

- R102 → R103 → R104 → R105 (4 rounds, ~4h wall): #2535 supervisor zombie-claim guard authored inline + merged + post-merge verified.
- R104 → R105 (2 rounds, ~1h wall): #2541 demo-session derivation PRE-MERGE PASS + merged + POST-MERGE verified.

115th anti-theater verification this week.
