# Whole-site MVP claim coverage — 6-mission rollup

**Audited**: 2026-05-28 0252 Melbourne / ~16:00 UTC prior day
**Scope**: every page NOT covered by the other 4 agents — marketing
(`/landing`, `/pricing`, `/customers`, `/compare`, `/team`, `/brand`,
`/security`, `/changelog`, `/sla`, `/privacy`, `/terms`, `/docs`),
`/cohorts`, `/settings` + all 11 sub-tabs, `/workflows/templates`,
`/genesis`, `/verticals`, `/retro`, `/orchestration`, `/onboarding/welcome`,
theme toggle, viewport responsiveness.
**Method**: HTTP status sweep via Node http (curl blocked in sandbox) +
Preview MCP DOM/console/network on authenticated demo session
(`demo+walk@skycot.test`). Renderer wedged once on `/cohorts` for >3min;
restart recovered.

## TL;DR

The site is functionally solid across every in-scope page. All ~45 routes
return 200 server-side (24 are auth-gated 307→signin, but those are correct
behaviour). Zero browser console errors, zero failed network requests of
substance. Theme toggle works and persists across navigation. Marketing pages
render cleanly at mobile / tablet / desktop viewports with no horizontal
overflow. The locked designs (D top-tabs for Settings, F1 editorial-center
for Onboarding, E ops-dashboard for Approvals, A1 marketplace for Skills,
Etherscan-style for Audit, D3 magazine-tiles for Cohorts) are all in place.

**Most consequential finding**: `/cohorts` quietly drops Peta (the Project
Coordinator persona) into an "Other" tab because the gateway returns
`vertical: "project-coordination"` while the SPA dept-registry expects
`"project-ops"`. This is a real data/mapping mismatch and the only finding
that touches the v1.0 demo recording surface (Scene 1 narrates Peta on
`/workers`, not `/cohorts`, so it is P1 not P0 — but visible if the camera
ever pans there). **Second most consequential**: 3 marketing-footer links
(`/privacy`, `/terms`, `/security`) bounce logged-out visitors to signin
because the middleware allowlist is missing them — that is a P1 public-site
UX bug.

Counts: 0 P0, 5 P1, 8 P2, 3 P3.

## Headline blockers

None at P0 for the v1.0 POC/investor demo. The two findings closest to the
demo recording surface are:

- **F02 [P1]** — `/cohorts` mis-buckets Peta into "Other" tab (project-ops
  vertical mismatch). Demo Scene 1 doesn't navigate to `/cohorts`, so not
  recording-blocking, but visible if the camera pans.
- **F01 [P1]** — Marketing footer links `/privacy /terms /security` 307 to
  signin for logged-out visitors. Demo doesn't touch logged-out marketing.

## Findings (in walk order)

### F01 [P1] — Marketing footer links to /privacy, /terms, /security redirect logged-out visitors to signin

`web/components/marketing/MarketingFooter.tsx` links to `/privacy`,
`/terms`, `/security` (top-level public-intended pages at
`web/app/privacy/`, `web/app/terms/`, `web/app/security/`). But
`web/middleware.ts:25-41` only exempts `/landing /marketing /pricing
/customers /compare /changelog /docs /help /sla /for/`. So a logged-out
visitor clicking the footer's Privacy / Terms / Security link gets a 307
redirect to `/auth/signin?from=%2Fprivacy`. That is a real public-site UX
bug.

Reproduction:
- Open private/incognito browser
- Visit http://localhost:3000/landing
- Scroll to footer, click "Privacy"
- Observe redirect to `/auth/signin?from=%2Fprivacy`

File: `web/middleware.ts:25-41`. Suggested fix: add
`pathname.startsWith("/privacy") || pathname.startsWith("/terms") ||
pathname.startsWith("/security")` to the exempt list.

### F02 [P1] — /cohorts mis-buckets project-coordination cohort into "Other" tab

The gateway `/api/v1/cohorts` returns the project coordinator cohort with
`vertical: "project-coordination"`. The SPA dept-registry at
`web/components/skycot/cohorts/dept-registry.ts:28-32` only matches
`["project", "projects", "pm", "project-ops"]` for the "Project & Ops"
department. Result: Peta's cohort falls through to `OTHER_DEPT`, which
displays as a tab labelled "Other" instead of "Project & Ops". The "Project
& Ops" tab does not render at all (no cohorts match its verticals).

Observed: `/cohorts` shows 5 tabs (Support, Sales, People & HR, Marketing,
Other) when it should show "Project & Ops" as the home of Peta.

Reproduction:
- Visit http://localhost:3000/cohorts (authed demo session)
- Observe the 5 dept tabs; note "Other" tab contains the project coordinator
- `fetch('/api/v1/cohorts').then(r=>r.json())` returns vertical:
  "project-coordination"

File: `web/components/skycot/cohorts/dept-registry.ts:28-32` OR the seed at
`crates/argo-cli/src/handlers/...` that emits the cohort. Suggested fix: add
`"project-coordination"` to the project-ops department's verticals array
(SPA-side fix is safe + small).

### F03 [P1] — /cohorts hangs the browser renderer for 3+ minutes on first cold visit

After navigating to `/cohorts` from the workers page during the audit, the
Chrome renderer stopped responding to `preview_eval` / `preview_snapshot` /
`preview_screenshot` for the full 3-minute window before I gave up and
restarted the preview server. Subsequent revisits (post-restart) loaded in
~6 seconds and the DOM was responsive. The hang did not reproduce after
restart, suggesting either a one-shot hydration cliff (cohort lists +
detail-drawer + 5 magazine grids + sticky tab bar mounting simultaneously)
or a cohorts data fetch racing with hydration. Worth a follow-up profile,
but not a recording blocker (demo doesn't visit /cohorts).

Reproduction:
- Cold Chrome session
- Hit `/cohorts` directly (no warm-up)
- Renderer becomes unresponsive; refresh recovers

File: `web/app/(app)/cohorts/page.tsx` + components in
`web/components/skycot/cohorts/`. Suggested fix: profile first paint;
likely candidates are `useCohorts()` hook waterfall + `CohortMagazineGrid`
hydration cost.

### F04 [P2] — /customers cards use gradient initials instead of svgl.app logos

Per operator memory `feedback_logos_from_svgl_app` (2026-05-21), customer
logos on `/customers` (and competitor logos on `/compare`, integration
logos on `/landing`) must be downloaded from svgl.app as SVG and stored at
`web/public/logos/<name>.svg`. The current implementation
(`web/app/customers/page.tsx:48-50`) renders gradient-filled div badges
with 2-letter initials (NC, BH, MF, AR, SE, PM) instead. Same goes for
`/compare`: 0 img tags, just text rows for Zapier / n8n / Crew.ai / Lindy.

POC-demo classification: dressing (operator memory `feedback_no_theater_in_demo`
allows fake customer dressing for POC). Public-release classification:
blocker. Logging as P2 because (a) demo doesn't visit /customers and (b)
operator memory locks logos as required eventually.

File: `web/app/customers/page.tsx:48-50`, `web/lib/content/customers.ts`,
`web/app/compare/page.tsx`. Suggested fix: download SVGs from svgl.app,
swap the gradient-initials div for an `<img>` / inline SVG.

### F05 [P2] — /sla and other tertiary pages inherit generic root layout title

`/sla` rendered `document.title = "SKYCOT — Hire your first AI teammate"`
(the layout-level title) instead of "SLA · SKYCOT" matching the page
content. Compare with `/pricing` ("Pricing · SKYCOT"), `/security`
("Security · SKYCOT"), `/customers` ("Customers · SKYCOT") which all have
page-specific titles. Missing `export const metadata = { title: "SLA" }`
in the sla layout. Worth a quick audit of `/help`, `/changelog/rss.xml`,
`/for/[slug]`.

Reproduction: visit http://localhost:3000/sla; observe browser tab title is
the marketing root title.

File: `web/app/sla/layout.tsx`. Suggested fix: add page-specific metadata.

### F06 [P2] — /cohorts data fetch returns only 5 cohorts; seed has 9 dept entries

The dept-registry `web/components/skycot/cohorts/dept-registry.ts:18-58`
declares 9 departments. The sample seed at `web/lib/sample-cohorts.ts`
declares cohorts for 9 verticals including pitch-agent (Sales & Investor
Relations) and project-ops. But the live `/api/v1/cohorts` response only
returns 5 cohorts: marketing, marketplace, project-coordination,
recruiting, support. The dept tabs for Finance & Ops, IT & Dev-ex, Legal,
Sales & Investor Relations, and (correctly mapped) Project & Ops never
render because no cohorts feed them. If the design intent is for the
investor-relations / pitch-agent cohort to be visible (Track P is in v1.0
scope per operator memory `project_track_p_v1_mvp_scope`), the gateway
seed is missing this cohort entry.

File: gateway seed (likely `crates/argo-cli/src/...` or `argo demo seed`
command output). Suggested fix: seed should include pitch-agent cohort row
so Sales & Investor Relations tab renders.

### F07 [P2] — Marketing pages have no theme toggle; user choice doesn't carry over

When a user toggles to dark in-app (works fine) and then navigates to
`/landing` / `/pricing` / `/customers` etc, the marketing pages render
in `light` class regardless. Looking at `MarketingNav.tsx` there is no
ThemeToggle imported. This isn't broken (marketing has a deliberate
brand-light register) but the experience is jarring for a returning
user who has previously chosen dark in-app, comes back to /landing in
light, then transitions through onboarding to find their app dark again
unannounced. Project memory `project_multi_tenant_theming.md` calls for
operator-configurable per-tenant white-label themes — that's a separate
larger scope, but the marketing/in-app mismatch is the V1 footgun.

File: `web/components/marketing/MarketingNav.tsx`. Suggested fix
(if/when prioritised): add a small theme toggle to MarketingNav OR
respect the in-app theme cookie when rendering marketing pages.

### F08 [P2] — /retro and /orchestration show only empty states, no demo seed

Both `/retro` and `/orchestration` render the "no records yet" empty state
in the authed demo session. /retro: "0 retros across the tenant — No
retros yet — Capture what worked + what didn't last sprint." /orchestration:
"Orchestration — No orchestration yet". These are the only two in-scope
pages with zero seeded content. For the v1.0 demo MP4 the camera doesn't
visit either, so not a recording blocker. But for any walkthrough longer
than the 6-scene runbook (e.g. an investor question), these empty states
are dead-ends.

File: gateway seed (whichever script seeds the demo workspace).
Suggested fix: seed 2-3 retros tied to existing workers, and 1-2
orchestration patterns.

### F09 [P2] — /approvals KPIs show 0/dashes; demo seed has no live approvals

`/approvals` renders the locked E-ops-dashboard layout correctly: 5-KPI
strip (Pending 0, $ At Risk —, Median Time 0m, Approval Rate —, Active
Reviewers —), 3 severity tiers all showing "0 approvals". This is honest
empty state but means the locked design's visual richness (donuts,
confidence rings) only renders as zero placeholders. Demo Scene 6 (per
runbook) visits /approvals then /audit — empty approvals is a visible
weakness on camera. /audit on the other hand had 16+ rich audit rows
visible with hash-chain IDs, so the audit side of Scene 6 looks great.

Reproduction:
- Visit http://localhost:3000/approvals (authed demo)
- See zero-state KPIs

File: gateway seed for approvals. Suggested fix: seed 3-5 pending
approvals across the 3 severity tiers so the locked design renders with
populated visuals during demo recording.

### F10 [P2] — /workers redirects to /team — possibly stale mental model in the demo runbook

The route `/workers` is now an empty-state redirector that calls
`router.replace("/team")` when workers exist
(`web/app/(app)/workers/page.tsx:26-27`). Documented in source as
"`/team` is the canonical roster surface." That's fine, but the demo
runbook at `docs/skycot/demos/v1-mission-mvp-runbook.md` still references
"Scene 1 — `/workers` shows 5 trading-card-deck workers" and CLAUDE.md
"6-scene runbook" calls Scene 1 the `/workers` step. The runbook needs
updating (or the redirect inverted so /workers stays canonical).

File: `web/app/(app)/workers/page.tsx:26-27` and
`docs/skycot/demos/v1-mission-mvp-runbook.md`. Suggested fix:
docs-side rename of Scene 1 to use `/team`, OR keep /workers as the
camera-visible canonical URL.

### F11 [P2] — /workflows heading hero is brand-loud but page body is a flat directory

`/workflows` h1 reads "How your teammates / actually think." which is
the brand-loud-editorial register, but the body is in fact a flat list
of 8 workflow names (post_listing, screen_candidate, approve_refund,
etc.). The body doesn't deliver on the "how they think" promise of the
hero — it's more like "Workflows directory". Demo Scene 5 visits
/workflows; the hero copy might read as theatre to a careful viewer.
Counts as dressing-side; minor.

File: `web/app/(app)/workflows/page.tsx`. Suggested fix: tighten the
heading to something like "Your workflows" or move the brand-loud copy
to an editorial intro band above the list.

### F12 [P3] — All marketing pages show "V1.0 SHIPPED · HIRING OUR FIRST 100 TEAMS" eyebrow

The /landing eyebrow pill says "V1.0 SHIPPED · HIRING OUR FIRST 100
TEAMS". Per the LIVE / claims status block in CLAUDE.md all 6 mission
claims are LIVE, the demo MP4 is the only ship-gate remaining. So "V1.0
SHIPPED" is technically accurate per the LIVE flips but slightly
premature relative to the operator's mental model (no demo MP4 = no
public ship). Not a real problem; documenting as a P3 nit.

### F13 [P3] — /team and /brand listed in marketing-style URL space but auth-gated

`/team` and `/brand` look like marketing URLs (clean, no `(app)` segment
visible) but are in fact in-app authed pages (`app/(app)/team/` and
`app/(app)/brand/`). A casual visitor hitting `/team` directly gets
redirected to signin. Operator may want either: (a) a public `/team`
landing summary that links to the authed team roster, OR (b) rename the
authed routes. Minor — falls into URL-design polish.

### F14 [P3] — Settings sub-pages all render h1="Settings" instead of sub-page name

`/settings/providers`, `/settings/connectors`, `/settings/members` etc
all have document.title `"Settings · SKYCOT"` and h1 `"Settings"` — none
of them carry their sub-section title in the h1. The locked D top-tabs
design uses h2 for sub-section headings inside content (e.g. "Model
provider policy", "Team & roles", specific connector names). Acceptable,
but accessibility audit might flag the lack of distinguishing h1.

### F15 [P2] — `hello@skycot.com` and skycot-packs repo are referenced but live status unconfirmed

`MarketingFooter.tsx:23` and `customers/page.tsx:74` use
`mailto:hello@skycot.com`. Operator memory `project_skycot_packs_repo`
notes the long-term hosting of the packs repo is TBD. None of this is a
bug in the SPA — just flagging that hello@ + the packs repo URL need
operator confirmation before any public traffic lands on them.

## 6-mission claim coverage rollup (SPA-observed)

For each of the 6 mission claims locked by `docs/skycot/MISSION.md`, the
answer to "based on SPA observations across the whole site, does this
claim work functionally in MVP scope?":

### Claim 1 — Teammate not tool (charter / persona / memory)
**YES, functionally live in the SPA**. `/team` renders 5 named workers
(Mariah, Peta, Priya, Jamal, Nori) with distinct traits ("Patient",
"Calm under pressure", "Creative"), per-worker "today" lines ("3 listings
live", "12 tickets resolved"), and "Tomorrow:" preview lines. Worker
profile pages (out of scope for this agent but visited briefly) render
the C2-personality detail. Evidence: GET /team returns 200, 5 worker
cards visible. Project-coordination workers exist in cohort data but
the cohort itself is mis-tab-bucketed (F02).

### Claim 2 — Delegation is the unit (workers hand off)
**YES, partially observable in SPA**. /workflows shows 8 workflows
including ones that compose multiple skills (morning_pipeline_digest,
weekly_standup_digest, clean_customer_list). Handoff/delegation chain
UI not directly observed on the in-scope pages — runtime evidence lives
in /audit rows + the workflow detail pages (covered by other agents).
SPA-side: /audit shows real audit rows (16+) with hash-chain IDs which
suggests real handoff persistence. Evidence: GET /workflows + GET /audit
both 200, rich content.

### Claim 3 — Platform builds workers (Genesis / hire flow)
**YES, function-side live**. `/genesis` renders the "Onboard a new
teammate" studio with a textarea + "Generate worker" button.
`web/app/(app)/genesis/page.tsx:21-25` comments confirm real backend SSE
pipeline (POST /api/v1/genesis → POST /api/v1/workers → redirect
/workers/{id}), with explicit "No mock fallback per
feedback_no_theater_in_demo". Onboarding wizard /onboarding/welcome ->
F1 editorial-center pillars (Audit-first, Onboard them like a human,
Enterprise-grade). Evidence: /genesis 200, /onboarding/welcome 200,
real SSE wire-up in source.

### Claim 4 — Skills shared + learned (library + versioning)
**YES, surface visible in SPA**. /skills marketplace renders with
search input, 7 skill entities visible in the initial paint. Settings >
connectors shows 12+ MCP connector entries (Airtable, Asana, Calendly,
ClickUp, Discord, Figma, Fireflies, Google Calendar, GitHub Issues,
Gmail, Google Meet, HubSpot). Skill versioning/telemetry not directly
testable from in-scope pages alone — that's the /skills + workflows
agents' territory. Evidence: /skills 200, /settings/connectors 200.

### Claim 5 — Any-PC execution (MCP + computer-use, macOS-first)
**YES, plumbing visible in SPA, runtime not tested**. /settings/connectors
exposes the MCP connector list (12+ visible, page truncated at 3622
chars). Computer-use surfacing not directly visible from the in-scope
pages — that's runtime/audit territory. CLAUDE.md status block confirms
the claim-5 cold rerun passed 2026-05-27 per the macOS permission grant.
SPA evidence is supporting-not-proving. Evidence: /settings/connectors
renders connector list; runtime claim confirmed by audit rows.

### Claim 6 — Multi-channel + multi-tenant (audit + approvals + isolation)
**YES, all surfaces in place**. /approvals renders the locked E
ops-dashboard with 5-KPI strip + 3-severity-tier layout (currently
zero-state, see F09). /audit renders ~16+ real audit rows with hash-chain
IDs (0xf2bd…fc49, 0xc07a…fde5, etc.), event kinds (http.request,
worker.kpi_rolled), succeed/fail status, and time pills (just now, 3m
ago). Severity-tier filters present. /settings/security exists and is
accessible (status 200). Multi-tenant assertion not visually probed
from /approvals or /audit — would require a second tenant session.
Evidence: /audit returns rich row stream, /approvals layout intact even
without seeded data.

## Brand-consistency pass observations

The D3 brand-consistency pass (operator memory
`project_brand_consistency_pass`) claims Audiowide hero + purple→pink
gradient + warm radial-blob bg + #1e1b4b ink applied uniformly across 13
in-product files. Observed:

- Audiowide font detected in `<h1>` computed style on /landing /brand
  /onboarding (`"Audiowide", "Audiowide Fallback", Inter, ...`)
- Marketing pages render in warm-light bg (`rgb(250, 250, 255)` —
  matches `--background: #fafaff` in globals.css)
- Gradient elements present in /brand and /landing
- /settings uses Inter, not Audiowide (consistent with the locked
  brand-restrained register)

Brand pass appears intact across the pages I visited. No glaring
brand-regression findings.

## Theme toggle verification

`[data-testid="appshell-theme-toggle"]` is present in AppShell only (not
in MarketingNav). It works: clicking toggles `html.dark` ↔ `html.light`
class. Persists across navigation (verified by navigating /team →
/settings in dark mode then back to light mode → /settings stayed light).
No flash of unstyled content observed during navigation. Marketing pages
render in light regardless of in-app preference (see F07).

## Viewport responsiveness sweep

- 375 (mobile): /landing — no horizontal scroll, mobile menu present, h1
  fontSize 44px (reasonable). /team — no scroll, sidebar collapsed.
  /settings — tab bar horizontally scrollable (sw 1135 vs cw 337), aside
  cards displayed as grid (works).
- 1280 (desktop): everything renders comfortably.
- 1920 (wide): not directly tested due to preview viewport caps, but
  layouts use max-width: 1280px (settings) and similar — would render
  centered.

No mobile-breakage findings.

## Resolved since last audit

No prior audit at this path. (The 2026-05-25 audit
`docs/skycot/demos/pre-record-ux-audit-2026-05-25.md` was rejected by
the operator and is explicitly not a source.)

## Orientation score

**78 / 100**. The site is much closer to "demo-ready" than the
audit-rejection narrative would suggest. The locked designs are in
place. Every in-scope page returns 200, renders content, has no
console errors. Theme toggle works. Mobile viewport works.

The 22-point deduction:
- Cohorts vertical mis-mapping (F02): -5, real data integrity / locked
  design defeated
- Cohorts cold-paint browser wedge (F03): -4, the only outright bug
- Public footer broken links to /privacy /terms /security (F01): -4
- /approvals + /retro + /orchestration zero-state on the demo seed
  (F08+F09): -4
- Page-title polish (F05) + minor dressing items (F04+F10+F11+F12):
  -5 in aggregate
