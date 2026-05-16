# claude-doc-refresh-spa-first-getting-started — Rewrite getting started for SPA-first + dev addendum (Wave 3 P1)

**Owner**: claude. **Depends on**: `claude-onboarding-wizard` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-doc-refresh-spa-first-getting-started`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Self-serve readiness:** customers need docs that match the SPA flow, not the CLI flow.

## Scope

P1. Current `docs/skycot/getting_started.md` is CLI-first (`curl bash → argo setup → argo workspace init`). After Wave 2, real users start at `skycot.com → signup → onboarding wizard`. Rewrite to match.

Sections:
1. **What is SKYCOT?** (1 paragraph)
2. **Hire your first worker** — 5-step walkthrough mirroring the onboarding wizard.
3. **Your first action + approval** — what happens when Maya runs her first duty.
4. **Customize your worker** — adjusting charter, duties, channels.
5. **Add a team member** — multi-user tenant access.
6. **Going to production** — billing tier, custom domain, security.
7. **Developer reference (link)** → `docs/skycot/developer/cli-getting-started.md` (the existing CLI flow, moved + light edit).

Plus:
- Move existing `getting_started.md` content to `docs/skycot/developer/cli-getting-started.md` (preserve for engineers/contributors).
- Update `README.md` top-level links to point at the new SPA-first doc.
- Update `RESUME.md` (the conductor entry doc) to mention SPA-first onboarding.

LOC: ~800.

## Files you may touch

- `docs/skycot/getting_started.md` — REPLACE with SPA-first content. ≤400 modified lines.
- `docs/skycot/developer/cli-getting-started.md` (NEW; moved content from old getting_started.md). ≤300 LOC.
- `docs/skycot/developer/README.md` (NEW or extend) — developer doc index. ≤50 LOC.
- `README.md` — narrowly: update top-level "getting started" links. ≤30 modified lines.
- `docs/build-plan/RESUME.md` — narrowly: mention new SPA-first path. ≤15 modified lines.

## Files you must NOT touch

- The SPA itself (onboarding wizard) — documenting it, not changing.
- Backend code — out of scope.
- Other docs that don't link to getting_started.md.

## Acceptance criteria

- [ ] New `getting_started.md` reads as SPA-first, sub-10-min path to first worker.
- [ ] All 5 onboarding wizard steps documented with screenshots (placeholder images ok; real screenshots in `claude-product-screenshots-and-imagery`).
- [ ] Developer addendum at `docs/skycot/developer/cli-getting-started.md` preserves CLI flow.
- [ ] `README.md` links updated.
- [ ] Markdown renders cleanly on GitHub.
- [ ] All internal links work.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-doc-refresh-spa-first-getting-started`
- PR title: `docs: SPA-first getting started + developer addendum (claude-doc-refresh-spa-first-getting-started)`

## Notes for the implementer

- Voice & tone: match brand identity guide. Direct, intelligent, restrained.
- Screenshots: placeholder for now (e.g., `![Step 1: Sign up](./img/step-1.png)` — image generation is a separate brief).
- Length: keep SPA-first version under 800 lines. Developers can read longer docs; new-user landing should be skimmable.
