# claude-onboarding-wizard — 5-step SPA onboarding from signup to first worker (Wave 2 P0)

**Owner**: claude. **Depends on**: `claude-spa-signup-flow` MERGED + `claude-tenant-provisioning-endpoint` MERGED + `claude-design-system-component-extension` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-onboarding-wizard`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Claim 1, Claim 2, Claim 3:** all 3 are introduced to the customer here. Onboarding is where SKYCOT's positioning becomes a felt experience, not a marketing claim.

## Scope

P0. 5-step onboarding wizard at `app.skycot.com/onboarding/*` (post-signup landing):

1. **`/onboarding/welcome`** — "Welcome to SKYCOT. Let's hire your first worker. (90 seconds.)" Single CTA: "Continue."
2. **`/onboarding/workspace`** — Workspace name + company name fields. Submit → `POST /api/v1/onboarding/provision`. Loading state → redirect to step 3.
3. **`/onboarding/connect-provider`** — Anthropic API key input (with link to Anthropic console). "Skip for now" option (uses mock provider, with warning). Validates key by making a $0.001 test call.
4. **`/onboarding/pick-vertical`** — 5 vertical packs presented as cards (gtm-engineer, marketing-intern, sales-sdr, customer-support, finance-ar). One-click "Choose this template." Visual: vertical manifest summary + skill count + sample duties.
5. **`/onboarding/first-hire`** — Plain-English textarea: "Describe your first worker. Example: 'A marketing intern named Maya who drafts blog posts and runs weekly competitive scans.'" Submit → Genesis call → live worker created → "Maya is hired! 🎉 Take you to her profile."

Persistence: each step writes progress to `useOnboardingDraft` zustand store (same pattern as existing `useHireDraft`). Resume mid-flow on refresh.

State machine: linear forward, can go back. Last step "complete" = redirect to `/(app)/team/<worker_id>`.

LOC: ~1200.

## Files you may touch

- `web/app/(setup)/onboarding/welcome/page.tsx` (NEW). ≤80 LOC.
- `web/app/(setup)/onboarding/workspace/page.tsx` (NEW). ≤140 LOC.
- `web/app/(setup)/onboarding/connect-provider/page.tsx` (NEW). ≤180 LOC.
- `web/app/(setup)/onboarding/pick-vertical/page.tsx` (NEW). ≤200 LOC.
- `web/app/(setup)/onboarding/first-hire/page.tsx` (NEW). ≤200 LOC.
- `web/app/(setup)/onboarding/layout.tsx` (NEW) — shared shell with ProgressDots (1-5). ≤80 LOC.
- `web/lib/onboarding/use-onboarding-draft.ts` (NEW) — zustand store, mirrors useHireDraft pattern. ≤90 LOC.
- `web/lib/onboarding/use-provision.ts` (NEW) — React Query mutation. ≤50 LOC.
- `web/lib/onboarding/use-provider-key.ts` (NEW) — provider-key validation hook. ≤60 LOC.
- `web/lib/onboarding/use-first-hire.ts` (NEW) — Genesis-call hook. ≤80 LOC.
- `web/__tests__/onboarding/*.test.tsx` (NEW) — per-step tests + full-flow integration. ≤300 LOC.

## Files you must NOT touch

- `web/app/(hire)/` existing hire flow — separate code path; this is FIRST-time onboarding.
- Backend endpoints — already provisioned in `claude-tenant-provisioning-endpoint`. This brief consumes them.
- Design system components — consume only.
- `web/app/(app)/` post-onboarding pages — separate; this just redirects.

## Acceptance criteria

- [ ] All 5 steps render with consistent layout (shared `/onboarding/layout.tsx`).
- [ ] Progress indicator shows current step (1/5, 2/5, ...).
- [ ] Forward navigation: each step's submit advances to next.
- [ ] Back navigation: each step has a Back button that returns to prior step preserving form state.
- [ ] Resume: refresh on step 3 → reloads at step 3 with prior fields populated.
- [ ] Step 3 provider-key validation: real call to Anthropic with $0.001 test → success/failure toast.
- [ ] Step 4 vertical-pack pick → API call to `/api/v1/onboarding/install-vertical` (handled by existing vertical-install endpoint).
- [ ] Step 5 first-hire → Genesis call → returns worker_id → redirect to `/(app)/team/<worker_id>`.
- [ ] Accessibility: keyboard navigable, aria-labels, focus order.
- [ ] Dark mode works.
- [ ] Mobile-responsive (test at 360px).
- [ ] Lighthouse ≥ 95 on each step.
- [ ] Test coverage: per-step happy path + edge cases (empty fields, API errors, resume mid-flow).
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-onboarding-wizard`
- PR title: `web: 5-step onboarding wizard (claude-onboarding-wizard)`

## Notes for the implementer

- **"Aha moment":** step 5 is where activation happens. The plain-English textarea is the heart of the product narrative. Make this step feel particularly polished.
- **Step 3 provider-key handling:** never store key plaintext in localStorage. POST to backend, backend stores encrypted. Validation call is one-shot.
- **Step 4 vertical pack cards:** each card uses the existing vertical-manifest JSON to render skill count + sample duties. No hand-coded copy per pack.
- **Step 5 Genesis call:** uses existing Genesis pipeline backend. The brief assumes it works; if it doesn't, fallback to "we'll create Maya for you in a moment" + async creation.
- **Voice & tone:** "Maya is asking for your approval — Slack reply will land it." Not: "AI-powered notifications!"
- After this brief + Wave 2 backend briefs: complete signup → first-worker-hire flow works end-to-end.
