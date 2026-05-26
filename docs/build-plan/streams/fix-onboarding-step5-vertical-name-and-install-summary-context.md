# fix-onboarding-step5-vertical-name-and-install-summary-context

**Owner**: claude.
**Authored against**: origin/main `80e149df55f6` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-step5-vertical-name-and-install-summary-context`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1028/onboarding-hire-an-ai-teammate.md` F11 [P2] + F13 [P2] (merged — same root cause).

🟠 **P1 — Step 5's hero says generic "Describe your first worker"; the wizard knows the user picked sales-sdr in step 4 but doesn't reference it. Install summary (8 skills · 1 workflow · 4 approval gates) is also dropped. Investor narrative loses the moment.**

## Root cause

`web/lib/onboarding/use-onboarding-draft.ts` persists `verticalId: string` only — the display name and install summary never propagate. Step 5 (`web/app/(setup)/onboarding/first-hire/page.tsx`) renders generic copy because it has no way to access "you chose Sales SDR and we installed 8 skills + 1 workflow."

`crates/argo-cli/src/handlers/gateway/routes_onboarding.rs:188-218` returns `{installed:true, skills_added:N, workflows_added:M}` for install-vertical. The wizard's `InstallVerticalResponse` type (`page.tsx:9-14`) reads all 3 fields but the wizard only acts on `installed`.

## DOM evidence

- `web/app/(setup)/onboarding/first-hire/page.tsx` hero header text: "Describe your first worker" (no vertical reference)
- `web/lib/onboarding/use-onboarding-draft.ts` draft store has `verticalId` only, no `verticalDisplayName` / `skillCount` / `workflowCount`

## Findings this brief closes

- onboarding-hire-an-ai-teammate F11 [P2] — `vertical_id` sent but display name + skill count never persisted to step 5
- onboarding-hire-an-ai-teammate F13 [P2] — workflows mentioned in install-vertical response but invisible to user

## Fix recipe

1. **Extend the draft store** (`web/lib/onboarding/use-onboarding-draft.ts`):
   ```ts
   type OnboardingDraft = {
     ...
     verticalId: string | null;
     verticalDisplayName: string | null;
     installSummary: { skillsAdded: number; workflowsAdded: number; approvalsAdded: number } | null;
   }
   ```
2. **In `pick-vertical/page.tsx`**, when the user selects a vertical:
   - Persist `verticalDisplayName` alongside `verticalId`.
   - Persist the install summary from the install-vertical response (after the install call completes).
3. **Add a step-4 → step-5 transition toast** (or inline confirmation card) at the top of `/onboarding/first-hire`:
   `"Sales SDR template installed · 5 skills · 1 workflow · 4 approval gates"`
4. **Rewrite step 5 hero copy** to reference the vertical:
   - Old: "Describe your first worker"
   - New: "Your **Sales SDR** template is ready — meet your first hire"
5. **Reset the install summary on draft clear** so a back-button + new vertical pick gets a fresh summary.

## Acceptance criteria

- [ ] Draft store includes `verticalDisplayName` + `installSummary`.
- [ ] Step 5 hero header substitutes the vertical display name dynamically.
- [ ] An install-summary card / toast appears between step 4 and step 5.
- [ ] Back-button to step 4 + pick a different vertical updates both the display name and the install summary.
- [ ] Web test asserts step 5 hero renders the chosen vertical name.

## May-touch

- `web/lib/onboarding/use-onboarding-draft.ts` (extend type)
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (persist new fields)
- `web/app/(setup)/onboarding/first-hire/page.tsx` (consume + render)
- `web/components/skycot/onboarding/InstallSummaryCard.tsx` (NEW)
- `web/tests/onboarding-step5-vertical-context.test.ts` (NEW)

## Out of scope

- Gateway-side display-name / skill-count truth (separate brief: `fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog`)
- Welcome step Genesis framing (separate brief: `fix-onboarding-welcome-step-genesis-studio-framing`)

## Effort

~80-120 LOC + 1 test. ~2 hours. Gated on `fix-onboarding-pick-vertical-skill-counts-and-gateway-catalog` so the install-summary numbers match gateway truth.
