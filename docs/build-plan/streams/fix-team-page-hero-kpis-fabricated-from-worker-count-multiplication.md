# fix-team-page-hero-kpis-fabricated-from-worker-count-multiplication

**Owner**: claude.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-team-page-hero-kpis-fabricated-from-worker-count-multiplication`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/whole-site-mvp-claims-coverage.md` F18 [P2].

🟡 **P2 — `/team` brand-loud hero shows fake KPIs (Key Performance Indicators): `weeklyActions = workers.length × 187`, `hoursSaved = workers.length × 12`. With 1 real worker (Maya) the hero reads "187 actions this week · 12 hours saved" — both fictional, computed by multiplication on the worker count. Per function-vs-dressing rule, the camera doesn't drill into `/team` in any of the 6 scenes, so this is P2 polish not a demo blocker. But the numbers are visible on the camera sweep.**

## Root cause

`web/app/(app)/team/page.tsx:151-152`:

```ts
const weeklyActions = workers.length * 187;
const hoursSaved = workers.length * 12;
```

These are decorative KPIs on the brand-loud hero — there's no real activity-data source wired up, so the page synthesizes plausible-looking numbers from the only metric it has (worker count). Per memory `feedback_no_theater_in_demo`, this is exactly the "theater the camera sees" failure mode the operator banned for the v1.0 demo MP4.

Two honest paths:
- **(a)** Wire real activity data from `/api/v1/audit?range=7d` (filtered to non-self-traffic per `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request`). Real count of agent-attributed audit rows over 7 days = `weeklyActions`.
- **(b)** Label the KPIs as samples: prepend "Sample —" or "Demo —" until real data is wired. Reads as honest demo theatre rather than fabrication.

For this audit, the camera doesn't pause on `/team` (the runbook drills into individual worker profiles via `/workers/{id}`). So option (b) is sufficient for the v1.0 demo gate; option (a) is the customer-trial fix.

## Findings this brief closes

- whole-site-mvp-claims-coverage F18 [P2] — Team hero shows fake KPIs: `weeklyActions = workers.length × 187`, `hoursSaved = workers.length × 12`

## Fix recipe

**Phase 1 (P2 — quick honest fix, ships first):**

1. **Add an "estimated" badge** to the hero KPIs. The brand-loud hero accepts a `kpis` prop array with `{label, value}` entries; extend with `{label, value, isSample?: boolean}`. When `isSample`, render a small "≈ sample" eyebrow above the number.
2. **Update copy** — "187 actions this week" → "≈ 187 actions this week (sample)" or similar.

**Phase 2 (P2 follow-up — real data):**

3. **Query real audit data.** Use `useAudit({range: '7d', verb_not_in: ['http.request', 'audit.detail.viewed', 'audit.replay']})` to count meaningful agent-attributed rows.
4. **Compute `hoursSaved`** from a simple heuristic: `realActionCount × 0.5` (~30 sec saved per action) OR pull from a dedicated `/api/v1/insights/hours-saved` endpoint if available. Document the heuristic inline.
5. **Drop the "(sample)" eyebrow** once real data lands.
6. **Vitest pin** for both phases.

## Acceptance criteria

- [ ] Phase 1: `/team` hero KPIs render with a visible "≈ sample" eyebrow OR a "Demo" label.
- [ ] Phase 1: the fabricated `× 187` / `× 12` multiplications are visibly tagged as estimates.
- [ ] Phase 2 (follow-up): real audit count from the past 7d replaces the synthetic number; eyebrow removed.
- [ ] No regression on the locked C2/D3 hero design.

## May-touch

- `web/app/(app)/team/page.tsx:151-152`
- `web/components/skycot/BrandLoudHero.tsx` (add `isSample` prop)
- `web/lib/api/audit.ts` (if Phase 2 needs a new helper)
- `web/lib/hooks/use-team-stats.ts` (new for Phase 2)

## Out of scope

- The /team page duplicate H1s (covered by `fix-team-page-duplicate-h1-and-settings-shell-h1-hierarchy`)
- The audit feed self-traffic filter (covered by `fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request`)
- Approvals KPI strip em-dash vs zero (covered by `fix-approvals-kpi-em-dash-vs-zero-empty-state-vocab`)

## Effort

Phase 1 — XS — ~20-30 LOC. ~30 min. Phase 2 — S-M — ~80-150 LOC. ~2-3 hours. P2 because no demo scene drills into `/team` hero KPIs; visible on camera sweep but not the focal point.
