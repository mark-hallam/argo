# fix-onboarding-per-step-demo-banner-dedupe-against-global-shell-banner

**Owner:** either
**Authored against:** origin/main `410e6ed1240d26f9ab6b48eb94a2fdf177c38270` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-onboarding-per-step-demo-banner-dedupe-against-global-shell-banner`
**Source:** ux-audit-2026-05-27-2351

🟢 **P2 — Onboarding step 4 (pick-vertical) and step 3 (connect-provider) each render their own inline demo-mode copy. Once the global `<DemoBanner />` mounts on the wizard shell (per `fix-onboarding-wizard-shell-demo-mode-banner-pass-through`), there will be up to 3 competing demo-mode indicators on step 4.**

## Root cause

Three demo-mode hints currently coexist on the wizard:
1. **Step 3 (`/onboarding/connect-provider`)**: subtitle reads "Demo mode — workers run with a mock provider. Type any sk-ant- key to continue; no Anthropic call is made." (`web/app/(setup)/onboarding/connect-provider/page.tsx:142-153`)
2. **Step 4 (`/onboarding/pick-vertical`)**: own demo banner below the back-CTA row "Demo mode — install deferred. Picking a vertical loads its workflows and skills as samples; nothing is provisioned to the gateway until you connect a real provider in Settings." (`web/app/(setup)/onboarding/pick-vertical/page.tsx:295-322`)
3. **Future global banner**: `fix-onboarding-wizard-shell-demo-mode-banner-pass-through` will add `<DemoBanner />` at the top of every `OnboardingShell` page.

Once (3) lands, step 4 will show banners (3) + (2) + the step-specific guidance — three competing indicators stacked vertically. Pick one.

> Step 4 (`/onboarding/pick-vertical`) renders its own demo banner... Step 3 (`/onboarding/connect-provider`) renders "Demo mode — workers run with a mock provider…" inside the subtitle prop. If F14 lands (`DemoBanner` mounted globally on the wizard), there'll be 3 demo-mode indicators on step 4: the global banner, the inline subtitle, and the step-4-specific banner. Pick one. The global `DemoBanner` is the right level (matches the in-app shell).
> — `onboarding-hire-an-ai-teammate.md` F15

This brief should land AFTER `fix-onboarding-wizard-shell-demo-mode-banner-pass-through` (which adds the global banner) — i.e. it depends on that brief shipping first.

## Findings this brief closes

- `onboarding-hire-an-ai-teammate.md` F15 [P2] — Pick-vertical "demo banner" + step-3 inline copy + future wizard-level banner = three competing demo-mode indicators

## Fix recipe

**Dependency**: `fix-onboarding-wizard-shell-demo-mode-banner-pass-through` should land first to establish the global banner.

1. **Delete the step-4 demo banner** at `web/app/(setup)/onboarding/pick-vertical/page.tsx:295-322`.
2. **Trim the step-3 subtitle** at `web/app/(setup)/onboarding/connect-provider/page.tsx:142-153` so it no longer mentions demo mode — just describes the actual connect-provider step. Demo-mode framing is handled by the global banner.
3. **Verify the global banner still reads correctly across steps 3 and 4** in both light and dark.
4. **Snapshot tests** for the affected pages should be updated.

If `fix-onboarding-wizard-shell-demo-mode-banner-pass-through` is delayed, this brief stays BLOCKED until it lands.

## Acceptance criteria

- [ ] Only ONE demo-mode indicator visible per onboarding step (the global banner from the shell brief).
- [ ] Step 3 subtitle is clean of demo-mode framing.
- [ ] Step 4 page no longer renders its own banner block.
- [ ] Snapshot tests pass.
- [ ] PR body cites finding F15 + confirms `fix-onboarding-wizard-shell-demo-mode-banner-pass-through` has landed.

## May-touch

- `web/app/(setup)/onboarding/pick-vertical/page.tsx`
- `web/app/(setup)/onboarding/connect-provider/page.tsx`
- Snapshot test fixtures

## Out of scope

- Establishing the global banner (covered by `fix-onboarding-wizard-shell-demo-mode-banner-pass-through`)
- Demo banner copy (covered by `fix-demo-mode-banner-investor-safe-copy` + `fix-demo-banner-copy-branch-on-data-source-or-suppress`)
- Pick-vertical grid contents (covered by `fix-onboarding-pick-vertical-grid-restrict-or-expand-to-mapped`)

## Effort

XS — ~20-40 LOC delete + snapshot update. ~30-45 minutes.
