# mission-onboarding-e2e-integration-test — Playwright E2E for full activation path (P1)

**Owner**: claude. **Depends on**: `claude-marketing-site-skycot-com` MERGED (#1754), eventually `claude-tenant-provisioning-endpoint` MERGED (currently BLOCKED on substrate brief).
**Authored against**: origin/main `c7275d4c` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-onboarding-e2e-integration-test`.
**Source**: SWOT gap — viral activation path is not covered by any automated test today. Operator can manually verify but can't run before every release.

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — the viral path MUST work; an automated test makes that durable.
- **Repeatability** (#3) — runs in CI on every PR touching `web/`, `marketing/`, or `crates/argo-cli/src/handlers/gateway/routes_auth*`.

## Scope

P1. Playwright E2E test that exercises the full new-user activation path:

1. Browse to `skycot.com/` (marketing site) — assert hero + CTA visible.
2. Click "Hire your first worker" CTA → expect navigation to `app.skycot.com/auth/signup`.
3. Enter throwaway email → submit → expect `/auth/check-email`.
4. Intercept magic-link email (via mock SMTP catcher) → extract verify URL.
5. Navigate to verify URL → expect `/onboarding/welcome` (needs_onboarding=true response).
6. Click Continue → `/onboarding/workspace` → fill "Acme Co" + "acme.com" → submit.
7. `/onboarding/connect-provider` → paste stub provider key → next.
8. `/onboarding/pick-vertical` → select `marketing-intern` card → next.
9. `/onboarding/first-hire` → fill description → submit → wait for worker creation.
10. Land on `/workers/:id` → assert worker name visible + 200 status + audit row exists.

Run modes:
- **Local**: against `localhost:3000` (marketing) + `localhost:3001` (SPA) + `localhost:8090` (gateway), all spun up by test harness.
- **CI**: same, but inside the test job — slower but durable.
- **Deploy-smoke**: env-gated mode targeting `skycot.com` + `app.skycot.com` + `api.skycot.com` after a Fly deploy.

LOC: ~350.

## Files you may touch

- `web/__tests__/e2e/onboarding.spec.ts` (NEW) — main test. ≤200 LOC.
- `web/__tests__/e2e/fixtures/mail-catcher.ts` (NEW) — stub SMTP. ≤80 LOC.
- `web/__tests__/e2e/fixtures/test-env.ts` (NEW) — env detection + URL resolution. ≤40 LOC.
- `web/playwright.config.ts` (existing or NEW) — config. ≤40 modified LOC.
- `web/package.json` — narrowly: add `@playwright/test` if not present. ≤4 modified LOC.
- `scripts/run-e2e-onboarding.sh` (NEW) — convenience runner that spins up all 3 services. ≤80 LOC.
- `.github/workflows/ci.yml` — add `e2e-onboarding` job (matrix: chromium only for now). ≤40 modified LOC.

## Files you must NOT touch

- Marketing site components — read-only consumers via the browser.
- SPA components — same.
- Backend Rust — same (consumed via HTTP only).
- The auth signing-key config — test uses real auth flow with a per-run ephemeral signing key.

## Acceptance criteria

- [ ] `pnpm e2e onboarding` (or equivalent) runs the full 10-step flow locally and passes.
- [ ] Test stubs SMTP so magic-link email is intercepted, not actually sent.
- [ ] Test detects regression at each step with a specific error (e.g. "Step 6 failed: workspace POST returned 400 expected 200").
- [ ] Test takes <2 minutes locally on a developer laptop.
- [ ] CI job runs the test on PR touching `web/`, `marketing/`, or `routes_auth*`.
- [ ] `deploy-smoke` mode targets URL params + skips SMTP stub (uses real magic link delivered by Postmark/SendGrid stub or manual operator intercept — test enters "operator-resume" mode if email not received within 60s).
- [ ] Test does NOT pollute real tenant table — uses per-run unique email + cleans up after.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-onboarding-e2e-integration-test`
- Title: `test(e2e): Playwright coverage for full activation path (mission-onboarding-e2e-integration-test)`
- Worker verification: pnpm test + playwright run + workspace clippy per SWARM_PROTOCOL.md.

## Hidden constraints

- **Tenant-provisioning is currently BLOCKED.** Until `claude-tenant-provisioning-endpoint` merges (waits on substrate brief), step 9 will fail with the same "no tenant table" error workers reported. Add `// TODO(post-substrate)` markers + skip step 9 + 10 if a env flag is set; flip the flag once provisioning ships.
- **Mail catcher** — use `smtp4dev` Docker image OR a simple Python script that captures + parses the link. Don't import a heavy mail-test library.
- **Headless chrome only** for now. WebKit + Firefox are follow-ups (`mission-onboarding-e2e-multi-browser` if needed).
- **Per-run unique email** — `e2e-test-${RANDOM}-${TIMESTAMP}@throwaway.local`. After test, DELETE the magic-link-token row + the (unused) user_account row.
- **No real provider key**. Use a stub Anthropic API server (e.g. `web/__tests__/e2e/fixtures/anthropic-stub.ts`) that returns success on the validate call.
- **The test IS the demo's automated cousin.** Don't diverge from `demo_script.md` step boundaries — when one updates, the other should.
