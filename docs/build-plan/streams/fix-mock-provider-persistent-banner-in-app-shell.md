# fix-mock-provider-persistent-banner-in-app-shell

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-mock-provider-persistent-banner-in-app-shell`.
**Source**: e2e-ux-audit Agent 1 finding F15 (`docs/skycot/ux-audits/2026-05-25-1100-onboarding-hire-an-ai-teammate.md`).

🟡 **P1 — operator who skipped provider key in onboarding never sees that workers are running on mocks.**

## Root cause

`web/app/(setup)/onboarding/connect-provider/page.tsx` Skip path: `setProvider(false, true); router.push("/onboarding/pick-vertical")`. The onboarding-page copy is honest: "Skipping uses a mock provider. Workers will run, but responses are simulated until you add a real key in Settings."

But once the user lands on `/team` after the hire, there is **no banner** anywhere saying "this workspace uses a mock provider". A user who skipped 5 minutes ago hires Maya, watches her stay idle forever, and has no clue why. The honesty during onboarding evaporates the moment the wizard completes.

Per `feedback_no_theater_in_demo`: this isn't theatre exactly — it's the opposite (real behavior that LOOKS broken because the context is lost). But it produces the same buyer-confusion pattern, and the fix is the same shape.

## Findings this brief closes

- Agent 1 F15 [P1] — Onboarding `/onboarding/connect-provider` "Skip for now" leaves a mock-provider tenant with no surfacing afterward

## Fix recipe

1. **Add a persistent "Mock provider" banner** at the top of the app shell (`web/components/skycot/app-shell/AppShell.tsx` or wherever the chrome lives).
2. Render condition: `providerKeyConfigured === false && providerSkipped === true`. Read from a settings hook (`useTenantSettings()` or similar; the data is already persisted in the gateway).
3. Banner copy: "Mock provider · Worker responses are simulated. [Connect a real provider key →]" — the link routes to `/settings/providers` (or wherever provider config lives).
4. Style: yellow/amber tint to differentiate from error banners, not red. Dismissible (per-session), but reappears on next mount until a real key is configured.
5. Visible on every authenticated app surface (`/team`, `/workers/<id>`, `/skills`, `/workflows`, `/audit`, `/approvals`).

## Acceptance criteria

- [ ] A tenant in `provider_skipped=true && provider_key_configured=false` state sees a banner on every authenticated page.
- [ ] The banner is dismissible per-session.
- [ ] Configuring a real provider key removes the banner permanently.
- [ ] Settings page link routes to the provider-config surface.
- [ ] No regression for tenants WITH a real provider key (banner hidden).
- [ ] Component test (`MockProviderBanner.test.tsx`) for both states.

## May-touch

- `web/components/skycot/app-shell/MockProviderBanner.tsx` (NEW)
- `web/components/skycot/app-shell/AppShell.tsx` (mount the banner)
- `web/lib/hooks/use-tenant-settings.ts` (if a hook doesn't already exist; otherwise extend)
- `web/app/(setup)/onboarding/connect-provider/page.tsx` (no change unless the skip flag rename is needed)

## Out of scope

- The provider-config surface itself (assumed shipped under settings page lock)
- Unifying `/onboarding` and `/hire` wizards (separate brief: `fix-onboarding-hire-parallel-wizards-unify-or-document`)

## Effort

~80-150 LOC + 1 component test. ~1-2 hours.
