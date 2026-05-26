# fix-sidebar-workers-vs-team-nav-label-consolidation

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-sidebar-workers-vs-team-nav-label-consolidation`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/onboarding-hire-an-ai-teammate.md` F8; `scene-1-and-2-claim-1-and-2.md` F18.

🟡 **P1 — Sidebar nav exposes both "Workers" AND "Team" — they're synonymous. Clicking "Workers" redirects to `/team` (URL flicker on every nav). Top brand says "AI TEAMMATES". Three different labels for the same concept = camera-visible navigation incoherence.**

## Root cause

`web/components/skycot/nav-items.tsx` (or `web/app/(app)/_AppShell.tsx`) lists both:
- `Workers` → href `/workers` → redirects to `/team` (`web/app/(app)/workers/page.tsx:19-46` does `router.replace("/team")` when workers exist; renders empty-state `HireStarterPrompts` otherwise).
- `Team` → href `/team` → renders the real roster + heroes.

Plus the brand wordmark above the nav reads "AI TEAMMATES" (`web/components/skycot/AppShellBrand.tsx` or similar).

Camera artifact on every "Workers" click: URL bar flashes `/workers` → `/team` (~50ms), the address bar jitters mid-recording. Plus three names for the same surface confuses the operator and any onlooker.

Sibling brief `fix-workers-redirect-to-team-url-flicker.md` already exists for the URL flicker mechanic, but it does NOT cover the label-consolidation step.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F8 [P1] — "Workers" and "Team" sidebar entries are duplicative; brand says "AI TEAMMATES"
- scene-1-and-2-claim-1-and-2 F18 [P3] — `/workers` is now a redirect-only page; nav label points to a route that bounces

## Fix recipe

Pick one canonical noun. Operator preference per memory `project_vocabulary_duty_vs_schedule` pattern: pick a single user-facing term, keep code names internal.

**(a) Single noun "Workers" everywhere (preferred — keeps `/workers` as the canonical URL).**

1. Delete the "Team" sidebar entry from `nav-items.tsx`.
2. Convert `/team/page.tsx` content → move into `/workers/page.tsx` (or keep `/team` as a 301 redirect TO `/workers`).
3. Brand wordmark stays "AI TEAMMATES" (it's the brand, not a nav label — keep distinct).
4. Update internal links across `web/app/(app)/**` that currently link to `/team` to link to `/workers`.

**(b) Single noun "Team" everywhere.**

1. Delete the "Workers" sidebar entry.
2. Delete `/workers/page.tsx` (or keep as a redirect to `/team`).
3. Brand wordmark unchanged.

**Operator decides between (a) and (b)** — both are clean. Default to (a) because the SPA route tree already uses `/workers/{id}` for detail pages (consistency). 

5. Search-and-replace any user-facing copy that says "your team" → "your workers" (or vice versa per chosen direction). Check: `/onboarding/welcome` H1 + sub-copy, the empty-state hero on `/team`, the `WorkerOverviewContent`'s "HOW {NAME} WORKS" eyebrow.

6. Vitest pin: `nav-items` exposes EXACTLY ONE of {`/workers`, `/team`} in the sidebar list.

## Acceptance criteria

- [ ] Sidebar shows ONE entry, not two, for the workers/team concept.
- [ ] The chosen URL (`/workers` or `/team`) renders directly without a redirect — URL bar does NOT flicker on click.
- [ ] All internal links use the chosen noun consistently.
- [ ] Brand wordmark "AI TEAMMATES" preserved (distinct from nav label).
- [ ] Vitest sidebar-shape pin passes.

## May-touch

- `web/components/skycot/nav-items.tsx` (drop one entry)
- `web/app/(app)/_AppShell.tsx` (if nav config lives here)
- `web/app/(app)/workers/page.tsx` (rewire content OR delete-and-redirect)
- `web/app/(app)/team/page.tsx` (same — symmetric)
- Internal hrefs across `web/app/(app)/**` (sweep)
- Vitest sidebar tests

## Out of scope

- URL-flicker mechanic per se (covered by `fix-workers-redirect-to-team-url-flicker.md` — orthogonal; this brief eliminates the redirect entirely)
- Brand wordmark change (out)
- Cohorts vs Workers distinction (cohorts are groups of workers — distinct concept, keep separate nav)

## Effort

S — ~50-100 LOC + sweep. ~1-2 hours.
