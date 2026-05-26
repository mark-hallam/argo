# fix-onboarding-workspace-suffix-tip-rephrase

**Owner**: claude.
**Authored against**: origin/main `6f5d5c49da2e` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-onboarding-workspace-suffix-tip-rephrase`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-1441/onboarding-hire-an-ai-teammate.md` F13 [P3].

🟢 **P3 — Workspace step's hint reads `Tip: add an environment suffix like "· production"`. Internal SRE/dev language that an enterprise buyer or investor wouldn't recognise. Camera blows past this step but the polish miss is camera-visible.**

## Root cause

`web/app/(setup)/onboarding/workspace/page.tsx:84-86` — the hint text under the workspace-name input suggests a dev convention ("environment suffix · production") that doesn't match how non-dev operators think about workspaces. Enterprise buyers think in terms of orgs/departments/business units, not environments.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F13 [P3] — Workspace suffix tip uses internal-language

## Fix recipe

1. **Rewrite the hint copy** at `web/app/(setup)/onboarding/workspace/page.tsx:84-86`:
   ```tsx
   <p className="ob-input-hint">
     If you have multiple workspaces — for different teams or business units — add a suffix here.
     Example: "Acme Co · sales" and "Acme Co · ops".
   </p>
   ```

2. Test that the copy renders without overflow at 540px viewport width (smallest supported per the responsive locked design).

3. **Snapshot test** (if onboarding has one): regenerate `web/app/(setup)/onboarding/workspace/__snapshots__/page.test.tsx.snap` after the copy change.

## Acceptance criteria

- [ ] Hint copy no longer mentions "environment suffix" or "· production".
- [ ] New copy reads as enterprise-friendly business-unit example.
- [ ] No layout regression at 540/700/890/1180 viewport widths.
- [ ] Snapshot updated if present.

## May-touch

- `web/app/(setup)/onboarding/workspace/page.tsx` (copy)
- `web/app/(setup)/onboarding/workspace/__snapshots__/page.test.tsx.snap` (if present)

## Out of scope

- The locked F1 wizard chrome
- The workspace-name validation logic
- Per-workspace theming (separate)

## Effort

~5-15 LOC. ~10-15 minutes.
