# fix-genesis-page-h1-onboarding-studio-not-hire

**Owner**: claude.
**Authored against**: origin/main `56f4ea0b4d84ebf52c14e387d95c4e99e1cca0b7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-genesis-page-h1-onboarding-studio-not-hire`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0415/whole-site-mvp-claims-coverage.md` F21 [P3].

🟢 **P3 — `/genesis` page H1 reads "Hire your next teammate." (brand-loud Audiowide gradient) — but the page is the Genesis onboarding studio, not a hire flow. The operator-clarified scope (memory `feedback_genesis_is_ai_teammate_onboarding_studio`) is voice/text role + SOP uploads + access controls + comprehension verification. The current H1 over-promises the hire wizard and under-promises the studio.**

## Root cause

`web/app/(app)/genesis/page.tsx` (verify path) renders an H1 of "Hire your next teammate." This phrasing collides with the `/onboarding/first-hire` wizard's actual hire flow. A visitor reading "Hire your next teammate" on `/genesis` reasonably expects another hire wizard — not a studio for uploading SOPs, voice clips, and access controls.

Per the operator-locked memory `feedback_genesis_is_ai_teammate_onboarding_studio` (2026-05-25T11:40Z):

> "Genesis takes voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification. The investor moment that distinguishes Skycot from 'ChatGPT with prompts'."

So Genesis is the STUDIO surface where a hired teammate gets fully onboarded — uploads, training material, access scoping. The H1 should reflect that.

## Findings this brief closes

- whole-site-mvp-claims-coverage F21 [P3] — `/genesis` page H1 over-promises the hire wizard, under-promises the studio

## Fix recipe

1. **Rewrite the H1** at `web/app/(app)/genesis/page.tsx` (verify line range). Candidates (operator-decisional):
   - "Onboard a new teammate." (parallel to the hire wizard's voice)
   - "Define a role." (more functional)
   - "Show them how the work gets done." (warmer, narrative)
   - "Teach a teammate the job." (most explicit about training intent)
2. **Update the sub-copy** so the page surface accurately describes what the studio does (upload SOPs / wiki / Slack / voice clips → comprehension test → ready to start).
3. **Update the eyebrow** if present so the surface name is "Genesis · onboarding studio" (or similar) — not "hire flow".
4. **Cross-check sibling briefs** `fix-onboarding-welcome-step-genesis-studio-framing` + `fix-onboarding-genesis-manifest-memory-eval-persistence` for vocab consistency.

## Acceptance criteria

- [ ] `/genesis` H1 reads as a studio / onboarding surface, not as a hire-wizard duplicate.
- [ ] Sub-copy describes upload / training / comprehension as the page's job.
- [ ] No vocab collision with `/onboarding/first-hire` wizard's voice.
- [ ] An investor reading the page in 5 seconds correctly classifies it as a studio for onboarding existing hires, not a re-entry to the hire wizard.

## May-touch

- `web/app/(app)/genesis/page.tsx` (H1 + sub + eyebrow copy)
- `web/playwright/genesis/page-h1-vocab.spec.ts` (NEW — pin the H1 wording)

## Out of scope

- Implementing the actual studio multi-modal upload UI (Phase 2 program; covered by `fix-onboarding-genesis-manifest-memory-eval-persistence`)
- Welcome step's pillar #2 vs delivery mismatch (covered by `fix-onboarding-welcome-pillar-vs-step5-promise-mismatch`)
- Page-level redesign

## Effort

~10-20 LOC + 1 spec. ~30 min. P3 polish but high vocab-clarity leverage.
