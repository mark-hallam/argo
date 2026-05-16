# claude-product-screenshots-and-imagery — Real screenshots for docs + marketing (Wave 3 P2)

**Owner**: claude. **Depends on**: `claude-onboarding-wizard` MERGED (so there's a real flow to screenshot).
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-product-screenshots-and-imagery`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Materials honesty** (Jonny-Ive discipline): show the actual product, not mockups.

## Scope

P2. Captures real screenshots of the SPA + product flows, places them in `docs/skycot/img/` + `marketing/public/screenshots/`. Provides image-optimization pipeline.

Screenshots needed:
1. Signup page
2. Onboarding step 1 (welcome)
3. Onboarding step 2 (workspace)
4. Onboarding step 3 (connect provider)
5. Onboarding step 4 (pick vertical)
6. Onboarding step 5 (first hire)
7. Team page with Maya hired
8. Worker profile (Maya)
9. Action approval card (Slack-style)
10. Audit trail view
11. Marketplace browse
12. Skill canvas (if shipped)

Each: full-screen 1920×1080 desktop + 360px mobile. WebP format (smaller). Alt text in adjacent JSON/MDX.

LOC: ~150 (mostly image files + a manifest + image-optimize script).

## Files you may touch

- `docs/skycot/img/screenshots/` (NEW dir) — 12 desktop + 12 mobile WebP files (~24 files). Binary, but tracked.
- `marketing/public/screenshots/` (NEW dir) — same set, served by marketing site.
- `docs/skycot/img/manifest.json` (NEW) — alt-text + caption for each. ≤80 LOC.
- `scripts/capture-product-screenshots.sh` (NEW) — Playwright-driven screenshot capture script for repeatability. ≤120 LOC.

## Acceptance criteria

- [ ] 24 screenshots captured (12 scenes × 2 sizes).
- [ ] All WebP, optimized < 200KB each.
- [ ] Alt text + captions documented in manifest.json.
- [ ] Playwright script re-generates them deterministically.
- [ ] No sensitive data (real API keys, real email addresses, real names) visible in screenshots — use seeded fixtures.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-product-screenshots-and-imagery`
- PR title: `docs+marketing: real product screenshots (claude-product-screenshots-and-imagery)`

## Notes

- Use seeded fixture data (e.g., `[email protected]`, `Acme Corp`, `Maya Engineer`).
- Run Playwright against `localhost:3000` (or wherever SPA dev-runs).
- Capture in both light + dark mode if dark mode is shipping. (Default light for v1; dark in marketing wave 3+.)
