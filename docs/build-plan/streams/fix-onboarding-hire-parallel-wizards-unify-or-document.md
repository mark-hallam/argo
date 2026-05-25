# fix-onboarding-hire-parallel-wizards-unify-or-document

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-hire-parallel-wizards-unify-or-document`.
**Source**: e2e-ux-audit Agent 1 finding F14 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md`).

🟡 **P1 — `/onboarding/*` and `/hire` are parallel 5-step wizards with content drift. CLAUDE.md parallel-abstractions risk.**

## Root cause

Both `/onboarding/welcome` (first-run) and `/hire` (from `+ Hire` on `/team`) wrap `WizardEditorialShell`. They drift on content:

- **Step 1 of `/onboarding/welcome`**: "Build your AI workforce" + 3 trust pillars (Audit-first / Single-binary / Enterprise-grade)
- **Step 1 of `/hire`**: "Build your AI workforce" + "What do you need help with?" + textarea
- **Step 5 of `/onboarding`**: "Describe your first worker" with textarea — same as `/hire` step 1

The first-run user gets the trust pillars pitch; the second-hire user gets straight to the task. That UX choice is reasonable, but the parallel-abstractions rule from `CLAUDE.md` flags this as a maintenance risk: any tweak to the wizard chrome has to be done in both places.

Per the drift-prevention playbook (`docs/skycot/DRIFT_PREVENTION.md`):
> No parallel abstractions. Two structs with the same name and overlapping responsibilities in different crates is a bug, not a feature.

The same applies to two pages with overlapping wizard step definitions.

## Findings this brief closes

- Agent 1 F14 [P1] — `/hire` and `/onboarding/*` are TWO parallel 5-step wizards with content drift

## Fix recipe

Pick (a) for the cleanest end-state:

**(a) Unify behind a `firstRun?: boolean` toggle (preferred)**:
1. Build a single `<HireWizard firstRun={boolean}>` component that lives at `web/components/skycot/onboarding/HireWizard.tsx`.
2. The `firstRun` prop:
   - When `true` (i.e. `/onboarding/*` path): renders the trust-pillars step.
   - When `false` (i.e. `/hire` path): skips the welcome step, starts at "What do you need help with?".
3. Delete the duplicated step definitions in both routes; both routes import the same wizard.
4. The route layer just sets `firstRun` based on whether the tenant already has workers.

**(b) Document the deduplication intent**:
1. If unification is rejected (too risky for v1.0), add a comment at the top of BOTH wizard files explaining why they're separate, what each is for, and a "TOUCH BOTH" reminder for any chrome change.
2. Add a lint that fails if `WizardEditorialShell` props drift between the two.

Recommendation: (a). It eliminates the drift risk permanently. (b) is acceptable as an interim if v1.0 demo timing rules out the refactor.

## Acceptance criteria

- [ ] Either (a) one shared wizard component used by both routes with a `firstRun` toggle, OR (b) both files carry a "PARALLEL-INTENT" comment + a CI lint pinning the chrome.
- [ ] Visiting `/onboarding/welcome` (first-run path) and `/hire` (second-hire path) both ship the same brand register, navigation, error handling.
- [ ] No regression on the first-hire journey (tested against the v1.0 demo runbook).
- [ ] Component test covering both `firstRun=true` and `firstRun=false` rendering paths.

## May-touch

- `web/components/skycot/onboarding/HireWizard.tsx` (NEW under Option a)
- `web/app/(setup)/onboarding/welcome/page.tsx` (route layer)
- `web/app/(setup)/onboarding/connect-provider/page.tsx` etc.
- `web/app/(app)/hire/page.tsx`
- Tests covering both paths

## Out of scope

- `/onboarding/pick-vertical` missing Recruiter / Marketplace verticals (separate brief: `fix-onboarding-vertical-list-align-with-landing-personas`)
- SOC-2 day-one overclaim copy (separate brief: `fix-onboarding-welcome-soc2-day-one-copy`)
- Mock-provider banner missing in app shell (separate brief: `fix-mock-provider-persistent-banner-in-app-shell`)

## Effort

~150-300 LOC for Option (a); ~30 LOC for Option (b). ~2-3 hours for (a).
