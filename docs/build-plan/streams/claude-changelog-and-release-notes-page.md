# claude-changelog-and-release-notes-page — v1.0 changelog + public roadmap (Wave 3 P2)

**Owner**: claude. **Depends on**: `claude-marketing-site-skycot-com` MERGED.
**Authored against**: origin/main `489e0580` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-changelog-and-release-notes-page`.
**Source**: §2 of `docs/build-plan/V1_WEEKEND_PLAN.md`.

## Mission alignment

- **Honest, public-facing release cadence builds trust.** Especially after the F-01 audit found a backlog row that wrongly claimed SHIPPED.

## Scope

P2. Render `marketing/app/changelog/page.tsx` as a content-driven changelog. Content source: `docs/skycot/changelog/`. Each release = MDX file. Markdown body + frontmatter (version, date, type: major/minor/patch).

Plus: `marketing/app/roadmap/page.tsx` — public roadmap showing what's planned next 4 quarters.

LOC: ~250.

## Files you may touch

- `docs/skycot/changelog/v1.0.0.mdx` (NEW) — v1.0 release notes covering what shipped this weekend. ≤300 LOC.
- `docs/skycot/changelog/index.md` (NEW) — changelog index for source-of-truth. ≤50 LOC.
- `docs/skycot/roadmap.md` (NEW) — v1.1-v2.2 roadmap excerpt with explicit "subject to change" framing. ≤120 LOC.
- `marketing/app/changelog/page.tsx` — read changelog MDX, render. Already created in `claude-marketing-site-skycot-com`, this brief HYDRATES it. ≤150 modified lines.
- `marketing/app/roadmap/page.tsx` (NEW) — render roadmap. ≤100 LOC.

## Acceptance criteria

- [ ] v1.0 release notes document each shipped audit finding + onboarding flow + brand identity.
- [ ] Public roadmap shows next 4 quarters of work.
- [ ] Changelog page renders MDX correctly with brand-consistent typography.
- [ ] Lighthouse ≥ 95.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-changelog-and-release-notes-page`
- PR title: `docs+marketing: v1.0 changelog + roadmap pages (claude-changelog-and-release-notes-page)`

## Notes

- Don't overstate v1.0. List facts: F-01 through F-14 closed, onboarding flow, marketing site, Stripe integration. Avoid puffery.
- Roadmap items are aspirational; mark as "Planned" or "Exploring" not "Coming Soon."
