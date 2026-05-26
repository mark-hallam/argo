# fix-playwright-hire-flow-regression-cluster

**Owner**: claude.
**Authored against**: origin/main `ddf44dfb4` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-playwright-hire-flow-regression-cluster`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2007/chrome-live-walk-and-playwright.md` Playwright failure cluster: hire-flow (8 specs).

🔴 **P0 — Hire-flow Playwright suite has 8 failing specs spanning every step of the 5-step wizard plus the shell + the celebration banner. The wizard is the load-bearing demo journey (Scene 3) and its regression suite is red across the board, indicating a broken state-machine wire shared by all the failures.**

## Root cause

The cluster is concentrated, suggesting a single shared root cause rather than 8 independent bugs:

| Spec | Failure surface |
|---|---|
| hire-flow-shell × 2 | The shared shell layout/state |
| hire-flow-step2 × 2 | Workspace step |
| hire-flow-step3 × 3 | Connect-provider step |
| hire-flow-step4 × 2 | Pick-vertical step |
| onboarding.spec | Overall wizard happy path |
| hire-flow-step2-listen | Redirect timing |
| hire-flow-step3-drafting | Redirect timing |
| hire-flow-step4-introduce × 2 | Redirect timing |
| hire-celebration-banner × 2 | Post-completion |

The audit's "Highest-leverage fix targets" section (#3) calls it out:
> "Hire-flow / onboarding test infra collapse (closes 8 playwright fails + F5): the step2/3/4 redirect failures + happy-path 30s timeouts cluster around a single broken state-machine wire."

Likely root causes (need investigation):

1. **The wizard's per-step navigation is gated on a successful gateway POST** (provision / install-vertical / first-hire). When the gateway POST fails — as it does in demo mode per `fix-onboarding-provision-demo-mode-bypass` — the wizard doesn't advance, so every step-N spec times out waiting for the step-N+1 URL.
2. **The `auto-listen-for-step-N` redirect logic may have regressed** — the specs `hire-flow-step2-listen`, `step3-drafting`, `step4-introduce` all reference a redirect/SSE-based "listen" mechanism that may have broken.
3. **The celebration banner spec may be gated on the wizard reaching `/workers/{id}` after a successful first-hire**, which doesn't happen if the read-back tenant resolver mismatch persists (covered by `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`).

## Findings this brief closes

- chrome-live-walk-and-playwright Playwright cluster: hire-flow (8 failed specs)
  - `hire-flow-shell` × 2
  - `hire-flow-step2` × 2 + `hire-flow-step2-listen`
  - `hire-flow-step3` × 3 + `hire-flow-step3-drafting`
  - `hire-flow-step4` × 2 + `hire-flow-step4-introduce` × 2
  - `onboarding.spec`
  - `hire-celebration-banner` × 2

## Fix recipe

1. **Read the playwright run.log** at `docs/skycot/ux-audits/2026-05-26-2007/playwright/run.log` and identify the actual failure mode per spec (timeout? assertion? selector miss? server error?). Don't guess.

2. **Reproduce locally**: `cd web && pnpm playwright test hire-flow-shell` then progressively the other 7. Capture screenshots of the failure state.

3. **Determine which of the 3 likely root causes** is the real one. Most likely #1 (gateway-POST-required-to-advance), which means this brief becomes a hard dependency on `fix-onboarding-provision-demo-mode-bypass` (and possibly `fix-onboarding-first-hire-invoke-genesis-plan` + `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`).

4. **If #1**: ensure the demo-mode bypass branches (added by the provision brief) extend to install-vertical and first-hire endpoints too. The wizard's per-step Continue handler should resolve a synthetic success envelope in demo mode without hitting the gateway.

5. **If #2**: re-author the listen/redirect logic. Possibly the SSE channel for streaming step transitions has bit-rotted. Look at `web/lib/onboarding/use-first-hire.ts` and sibling hooks.

6. **If #3**: the celebration banner specs are downstream of the tenant resolver fix; mark those two specs `test.skip` with a TODO referencing the tenant-resolver brief until that lands, then re-enable.

7. **Re-run the cluster** until all 8 specs are green. Update the test snapshots if any visual baseline has drifted.

## Acceptance criteria

- [ ] `pnpm playwright test "hire-flow"` passes 0 failed
- [ ] `pnpm playwright test "onboarding.spec"` passes
- [ ] `pnpm playwright test "hire-celebration-banner"` passes
- [ ] The 8 failing specs from the 2026-05-26-2007 run.log are now all green
- [ ] Test wall-time for the cluster < 5 minutes

## May-touch

- `web/lib/api/onboarding.ts` (demo-mode branches per other briefs)
- `web/lib/onboarding/use-first-hire.ts` (SSE/listen if regressed)
- `web/app/(setup)/onboarding/*/page.tsx` (Continue handlers)
- `web/playwright/onboarding/hire-flow-*.spec.ts` (selector updates if DOM shifted)
- `web/components/skycot/onboarding/CelebrationBanner.tsx` (if banner UI changed)

## Out of scope

- The underlying demo-mode bypass fix (covered by `fix-onboarding-provision-demo-mode-bypass`)
- Tenant resolver mismatch (covered by `fix-onboarding-tenant-resolution-cwd-vs-principal-mismatch`)
- Genesis invocation (covered by `fix-onboarding-first-hire-invoke-genesis-plan`)
- Authoring net-new specs

## Effort

~200-400 LOC across multiple specs + supporting test infra. Heavily dependent on root-cause investigation. ~6-12 hours. P0 because the wizard is the demo's load-bearing journey and the regression suite is the operator's safety net.
