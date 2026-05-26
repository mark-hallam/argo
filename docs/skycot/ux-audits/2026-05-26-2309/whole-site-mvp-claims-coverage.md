# Whole-site MVP claim coverage + load-bearing rollup

**Audit run**: 2026-05-26 23:09 AEST (00:30 UTC offset run)
**HEAD**: `2cc2ec42b` (status: claim fix-approvals-ops-dashboard-inline-approve-reject-buttons)
**Gateway**: 7400, schema v83, demo-mint tenant `019e5057-1d3f-7fa2-9c1b-c34ef01cc21e`
**SPA**: `pnpm dev` on :3000 with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` + `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`
**Method**: read-only walk via `mcp__Claude_Preview__*` + direct `curl` against gateway + source-code grep. No cargo, no mutations.
**Auditor**: e2e-ux-journey-test-agent (whole-site rollup scope)

## TL;DR

23 findings: **5 P0**, **8 P1**, **8 P2**, **2 P3**. The big load-bearing surprise is that **every page outside `/landing → /pricing → /docs → /security → /changelog → /compare → /customers` runs on client-side mock fallback**: the real gateway has **0 workers visible at `/api/v1/workers` (404), 0 cohorts beyond Maya (one marketing-intern placeholder), 0 marketplace skills, 0 workflows, 0 approvals**. The SPA renders Mariah/Peta/Priya/Jamal/Nori, the 6-skill marketplace, the 7-workflow gallery and the 9-row audit page entirely from `lib/api/{team,cohorts,skills,approvals,audit}.ts` seed fixtures. Per the function-vs-dressing rule, **rows the camera drills into are function-side** — so Scene 2 ("click Mariah, click Run now, audit row appears in 5s") and Scene 6 ("click an audit row, see real provenance") are at risk: the drill-in shows "Action detail unavailable — drill into another row for full provenance" against an "Unknown worker" with `mem_placeholder` / `generic-skill` / `generic-policy` provenance. The biggest blockers (top of file F1, F2, F3, F4, F5) are demo-recordable footguns the camera will catch. The 6 mission claims map to 🟡-with-gaps for 5 of 6 in **SPA visibility** — different from the runtime LIVE state recorded in CLAUDE.md.

## Headline blockers (P0 only)

1. **F1 [P0]** — `/audit` drill-in renders "Action detail unavailable" + "Unknown worker" + placeholder provenance for the seeded SPA rows. Scene 6 of the v1.0 runbook drills into an audit row; the camera will see fabricated-looking "not available" theatre.
2. **F2 [P0]** — `/api/v1/workers` returns 404 from the gateway; the SPA falls back to client-side mocks for the Mariah/Peta/Priya/Jamal/Nori roster. The team-headline row that Scene 1 narrates exists ONLY in `web/lib/api/team.ts` seed data, not on the wire. If the operator clears the cache or signs in as the real demo-mint principal, Scene 1 shows a single worker named "Maya".
3. **F3 [P0]** — `/cohorts` shows ONE cohort ("L0 Pre-Trust Marketing-Intern" under "Other · 1 cohort") with a single dept tab. The locked `project_cohorts_page_design_locked` (D3 magazine-tiles with 6 dept tabs incl. Project & Ops with Peta + Patrick) is **not implemented in MVP**. If the demo navigates to /cohorts the viewer sees an empty, single-tab layout that disagrees with the workers page (5 teammates across 5 verticals).
4. **F4 [P0]** — `/workflows` and `/workflows/templates` cards have `background: rgb(255,255,255)` AND `color: lab(96.5,…) ≈ #f6f6f6` (off-white) in dark mode. Workflow names ("first_listing", "qualify_lead") and TRY pills ("approval flows", "data extraction") are unreadable when a viewer is in dark mode. Per `project_multi_tenant_theming` the SPA promised light + dark + tenant white-label theming.
5. **F5 [P0]** — The sidebar nav exposes BOTH "Team" and "Workers" as separate links pointing at different routes that resolve to the same data (`/workers` soft-redirects to `/team` once workers exist). Two nav rows for the same surface is a function-side confusion the demo cannot afford.

## All findings, in order

### Marketing perimeter (function-side OK, dressing-side mostly OK)

#### F6 [P1] — `/customers` cards are fabricated NC/BH/MF/AR/SE/PM logos with real-feeling stat lines and quotes

`web/app/customers/page.tsx:40-68` maps over `SEED_CUSTOMERS` (six entries: Northwind Commerce / Beacon Health Ops / Maple Finance / Apex Recruiting / Skyline Engineering / Polaris Marketing) with hand-authored quotes from "Priya Subramaniam · VP Support · Northwind Commerce", "Dr Mei Lin · CCO · Beacon Health Ops", etc. The detail pages exist (`/customers/northwind-commerce` returns a real-feeling case-study article with "Northwind moved 4.2 million orders in 2025"). Per the function-vs-dressing rule this is permitted dressing — but the v1.0 demo MP4 does NOT visit /customers, so this only matters when an investor opens a side tab. Not a POC blocker. Suggested: tag this surface with `data-pending-real-quote="true"` on every card (already partially present at line 45) and never reference real industries like "AFSL-holder" / "HIPAA-compliant out of the box / patient-portal tickets" — those tip from dressing into untrue-claims territory.

Repro: `open http://127.0.0.1:3000/customers` and read the third Maple Finance card.

#### F7 [P2] — `/for/sales` reverts to "Hire your first worker" / "Sign in" CTAs and a different nav (no Docs / Changelog / Roster)

`web/app/for/sales/page.tsx` (and siblings finance/marketing/ops/recruiting/support) ships a vertical-landing variant with CTA copy that disagrees with the canonical `/landing` ("Hire your first teammate"). Same brand-design language, drift in voice. Memory `project_pm_cohorts_peta_patrick` + the landing-page hero use "teammate"; "worker" is the runtime word the CLAUDE.md status block uses. Pick one for marketing copy. Code keeps "worker" (canonical primitive), UI standardizes on "teammate".

Repro: `open http://127.0.0.1:3000/for/sales`, then `open http://127.0.0.1:3000/landing` and diff the buttons.

#### F8 [P2] — `/pricing` Free tier copy ("Mock provider only · Single channel (web inbox)") tells a public visitor they're getting a non-functional sample

`web/app/pricing/page.tsx` — the Free column reads "Try SKYCOT with the mock provider. No credit card / 1 teammate / 100 actions / month / Community support / Mock provider only / Single channel (web inbox)". A public landing visitor sees that and concludes the free product is a toy. If the demo isn't trying to convert v0 customers, this is fine; if it is, this copy needs softening. POC demo doesn't visit /pricing in the 6-scene runbook so this is P2 not P0.

#### F9 [P3] — `/customers` cards have visible "Read story →" CTA but the Link wraps the whole card (CTA is decorative span)

`web/app/customers/page.tsx:60-66` shows the CTA inside the Link, which is correct accessibly. The whole card is the clickable target. Visual UX-wise the CTA looks like a button but isn't separately focusable. Minor — accessibility-wise the card link is fine.

### App shell — sidebar nav

#### F5 [P0] (already in headline) — "Team" and "Workers" both in left nav

`web/components/skycot/AppShell*.tsx` (rendered as `nav > a[href="/team"]` + `nav > a[href="/workers"]`). `web/app/(app)/workers/page.tsx:14-21` documents the intent: "/workers is a nav-bar entrypoint... when workers exist, redirect to /team". The redirect works, but BOTH links are still in the sidebar simultaneously. A user clicking Workers gets a brief Loader2 spinner then bounces to Team. Two nav rows for the same surface is broken IA. Suggested: drop the "/workers" sidebar row and let the empty-state CTA live as a starter card inside /team.

Repro: load any in-app page; observe left nav lists Team THEN Workers as two separate rows.

#### F10 [P1] — `/` (root) and `/approvals` render DIFFERENT approvals layouts

`web/app/(app)/page.tsx` renders the "inbox" approval queue (5 KPI cells: PENDING / RESOLVED / WK / AVG RESPONSE / $ AT RISK / APPROVAL RATE with "All clear" empty-state). `web/app/(app)/approvals/page.tsx` renders the locked E-ops-dashboard (KPI strip + P0/P1/P2 severity tiers + $ at risk donut + Confidence donut). The sidebar's "Approvals" row routes to `/` (the inbox). A typed `/approvals` URL gets the ops-dashboard. **Two visual languages for the same data, depending on how you arrived.** The 2026-05-26 fix-cycle note in `approvals/page.tsx:22-27` already removed the empty-state short-circuit but kept the two-layout split. Suggested: pick one. The E-ops-dashboard at `/approvals` matches the locked design memory; route `/` should redirect to `/approvals` (or both layouts converge).

#### F11 [P2] — `/brand` lives inside the app shell, exposed to logged-in users via direct URL

`web/app/(app)/brand/page.tsx` is an internal brand reference (color tokens, typography, wordmark panel). It renders inside `(app)` so the sidebar nav is visible and Demo banner sits on top. Not linked from any nav row — discoverable only by typing the URL. Either link it from Settings (developer reference) or move to `(setup)` / `_dev/` so it doesn't pollute the route tree. P2 because it's internal-team utility, not customer-facing.

### Workers / Team / Worker-detail

#### F12 [P1] — Workers cards show "MOCK-OFFLINE" status pill next to the page heading

`/team` snapshot: `[2685] heading: "Team" → "MOCK-OFFLINE"`. The pill is a real status indicator (gateway returned offline, falling back to mocks) but the demo MP4 records this exact view in Scene 1. The viewer will read "Team MOCK-OFFLINE" next to "5 teammates, one team" and either ignore it or wonder if the product is broken. Suggested: when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, suppress the MOCK-OFFLINE pill (this is the demo path, not a production fallback).

Repro: `open http://127.0.0.1:3000/workers` (or `/team`), look at the section heading next to "Team".

#### F13 [P1] — Hero KPI strip on /workers shows "0 working right now · 935 actions this week · saving you ~60 hours"

`/team` hero: WORKING=0, ACTIONS/WK=935, HOURS SAVED=60. The "0 working right now" disagrees with "5 teammates, one team" and the per-card "Waiting on you" status. The 935 + 60 are fixture numbers — fine as dressing. The 0 is misleading. Suggested: either pull a non-zero "currently doing something" stat from the audit log, or use a different framing ("5 teammates ready · 935 actions this week").

#### F14 [P2] — Worker detail tab Skills/Trust/Escalation/Settings — content not yet verified

`/workers/{id}` opens to the C2-locked personality view (Overview / Activity / Memory / Skills / Trust / Escalation / Settings tabs). Overview renders strongly (traits chips, "How Mariah works", great-at vs escalate-when, signature behaviors). The other tabs weren't visited in this audit and likely have varying levels of completion. Spot-checking before recording: open Mariah's Skills tab, confirm it doesn't show "Loading worker..." indefinitely.

### Cohorts

#### F3 [P0] (already in headline) — Cohorts page has ONE dept tab ("Other") and ONE cohort

Per `project_cohorts_page_design_locked` the page should have 6 department tabs incl. Project & Ops with Peta + Patrick + asymmetric magazine tiles (1 lead + 2 stacked + 4 compact). Actual:

```
[3097] region: "Cohorts by department"
  [3098] tablist
    [3099] tab: "Other 1 COHORT"
  [3107] heading: "Other · 1 cohort"
  [3122] button: "Open cohort marketing-intern Pre-trust"
    [3127] heading: "L0 Pre-Trust Marketing-Intern"
```

The gateway returns `{"cohorts":[{"cohort_id":"marketing-intern::unknown","vertical":"marketing-intern",...}],"total_count":1}`. The SPA seed file used in demo-mode probably mirrors this. The locked 6-tab design is **not** wired up.

`web/app/(app)/cohorts/page.tsx` (file exists, demo path runs). Code path likely groups cohorts by department but with only one cohort the page degenerates to "Other" bucket.

Suggested fix: extend the demo seed (or the mocked `useCohorts` hook) to surface six departments (Marketplace / Project & Ops / Recruiting / Support / Marketing / Engineering or comparable) so the locked design renders. Operator should NOT cut a brief that re-architects the page until the seed is fixed.

#### F15 [P2] — "Other · BUILT IN MELBOURNE" eyebrow is permanent on the cohort department block

Minor brand flourish but reads as decorative metadata on a real category. Either remove or move to footer.

### Settings

#### F16 [P1] — `/settings/providers` is read-only — no way to add or rotate an API key from this UI

`/settings/providers` renders "Primary provider: Anthropic / Fallback: Fail closed / Per-worker overrides: Audit logged" with a "Review API keys" link in the corner. Clicking that goes to `/settings/api-keys` which is presumably the actual key form. The provider page itself has nothing actionable. For the v1.0 enterprise positioning (per `project_v1_strategic_decisions_2026_05_14`) operators need to plug in their own provider keys before any workers can run real work — that's the path from Free tier ("Mock provider only") to Pro. The current Providers page hides that. Suggested: inline a "Use my API key" form here.

#### F17 [P2] — `/settings/security` Security page heading uses faint gradient text on dark background

`<h2>Security</h2>` renders as light-purple-on-dark gradient text — readable but well below WCAG AA contrast threshold for hero headings. Same pattern on Cohorts hero. Brand-loud register, but it's the page TITLE — needs ≥3:1 contrast.

Repro: dark mode, `/settings/security`, observe page hero word "Security".

#### F18 [P2] — `/settings/connectors` catalog cards show "Not connected" + "Connect" CTA, but a click of the SPA's Connect button hasn't been verified end-to-end here (29 of 29 visible)

Functional surface exists. Connect flow may be a stub modal that closes immediately — needs a focused journey test of one OAuth path. P2 because the demo MP4 doesn't enter the connector setup flow.

### Skills

#### F19 [P0] — Skill detail page (`/skills/{id}`) has NO install / pin / "Use this skill" CTA

`/skills/approval-route` (clicked from the marketplace) renders:
- PROVENANCE block: "Authored by SKYCOT · Ungraded"
- WORKERS USING THIS SKILL: "No workers are using this skill yet. Hire a worker and add this skill from their Skills tab to put it to work. Used by 0 workers across the SKYCOT marketplace."
- ← Back to the marketplace

There is **no install button**, no "Add to a worker", no pricing, no version-history. The marketplace page promises a clickable browsing experience; the detail page is a dead-end. Scene 4 of the v1.0 runbook narrates "Search 'competitor pricing scrape' → 3 results → click result → install flow". The install flow does not exist.

Suggested: add an "Add to worker" picker on the detail page that pins the skill to one of the 5 demo workers, write a single audit row "skill.installed", and the marketplace counter goes 0→1.

#### F20 [P1] — Skill marketplace hero search has no working empty-state when a query misses

`/skills` shows "6 skills · 6 shown" and TRY pills (approval routing / invoice extraction / candidate screening / deal qualification / support triage / lead enrichment). Typing a non-matching string filters down — but I didn't verify the empty-state. Recommend: search "asdfqwer" and confirm the page doesn't crash or render a blank grid.

### Audit

#### F1 [P0] (already in headline) — Audit drill-in returns "Action detail unavailable — drill into another row for full provenance"

`/audit/{actionId}` for the 9 seeded rows renders:
- Header: "Unknown worker · Action detail unavailable — drill into another row for full provenance · May 26, 23:17 · Succeeded"
- Inputs panel: "Detail payload not available for this action id."
- Provenance & Chain panel: MODEL=claude-opus-4-7 2025-, PROMPT TEMPLATE=generic.placeholder, SKILL VERSION=generic-skill 1.0.0, MEMORY SNAPSHOT=mem_placeholder, POLICY VERSION=generic-policy. Decision chain: "1. Memory loaded · context (cached) · pricing rules" with subsequent steps clipped.

The audit list view shows 9 rows that look real (`http.request` action_type, 16-char hex hash IDs, "Succeeded" status, relative times like "5h ago", PII-redacted banner). The data is REAL gateway data — but it's all `audit.detail.viewed` http.request rows that the SPA itself wrote as it loaded pages. The agent-action rows the demo needs ("approval.granted by Mariah", "skill.invoked by Peta", "computer-use.opened-notes-app by Priya") are not in the seed.

Suggested fix: seed at least 5 real agent-action rows on `argo demo seed --tenant <demo>` so Scene 6 has somewhere to drill in.

#### F21 [P2] — "PII redacted — your role shows masked data" banner appears for the demo-mint principal

`/audit` shows "PII redacted — your role shows masked data. Request elevated view →" for the demo principal. For a POC demo where the audience IS the operator, this banner adds friction. Suggested: either auto-grant elevated for `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, or hide the banner entirely.

### Workflows

#### F22 [P0] partial — `/workflows` index dark-mode bug (covered in F4)

#### F23 [P1] — Workflow detail page (e.g. `/workflows/post_listing`) has 4 buttons in the toolbar (Run now + 3 unreadable)

`/workflows/{id}` renders "Run now ▶ · [3 ghost buttons]" in the version-bar. The three subsequent buttons are white-on-white in dark mode so I couldn't read their labels from the screenshot. Could be "Add a step / Remove gate / View timeline / Save as draft / Publish" (per the builder route earlier). Same dark-mode contrast bug as F4.

### Onboarding

#### F24 [P3] — `/onboarding/welcome` is the F1-locked editorial-center design — works correctly

Verified the locked F1 layout: STEP 01 OF 05 progress bar (top), Audiowide hero "Build your AI workforce.", 3-card grid (Audit-first / Onboard like a human / Enterprise-grade), gradient "Get started" CTA, "HELP · SAVE & EXIT" top-right. This is the operator-locked design and ships correctly. No finding.

### Theme toggle + viewport

#### F25 [P1] — Theme toggles `html.dark` and `data-theme="skycot-dark"` correctly, but some cards / pills are styled with hardcoded white backgrounds that don't theme-respond

Confirmed via `preview_inspect` and DOM eval: top-level HTML carries `data-theme="skycot-dark"` and `class="dark"` after toggle. The cards on /workflows + /workflows/templates use `rgb(255,255,255)` backgrounds with `lab(96.5,…)` near-white text. Same on /skills card detail. The theming system is wired; the components are not. Top affected files (per a grep): `web/app/(app)/workflows/page.css`, `web/app/(app)/workflows/templates/page.css`. CSS modules need `var(--surface)` instead of literal white.

Repro: dark mode + `/workflows` + `/workflows/templates` + `/skills/{id}`. Compare with `/team` which themes correctly.

#### F26 [P2] — Mobile (375 wide) hero "Hire your first teammate" + "See how it works" buttons stack tightly side-by-side

`/landing` at 375×812 shows both CTAs in a row at ~50% width each. Both wrap text to 2 lines and look cramped. Suggested: stack vertically below ~420px width.

#### F27 [P3] — Viewport at 768 (tablet) collapses to hamburger; primary nav not visible until burger expand

`/landing` at 768×1024 hides "How it works / Features / Compare / Roster / Pricing / Docs / Changelog" behind the burger. Reasonable but the breakpoint feels conservative for tablets — the nav probably fits at 768 with smaller font. Tiny polish.

### Operational signals (gateway + bypass)

#### F28 [P2] — Multiple `/api/v1/*` endpoints return 404 (workers, tenants, connectors, openapi.json)

```
curl :7400/api/v1/workers           → 404 not found
curl :7400/api/v1/tenants           → 404 not found
curl :7400/api/v1/connectors        → 404 not found
curl :7400/api/v1/openapi.json      → 404 not found
```

The SPA's `useTeam()` hook is wired to `/api/v1/team` (200 — returns Maya only) but the page header and worker cards came from client-side mock fallback because the seeded gateway only has 1 worker. The 404 on `/workers` is consistent with the team-endpoint canonicalization that shipped. Not a bug per se — but if any new feature code on the SPA expects `/api/v1/workers` it will silently fall through to mocks. Suggested: either return 410 Gone with a redirect-hint, or implement /workers as an alias for /team.

Repro: from a clean shell, `for p in workers tenants connectors openapi.json; do curl -o /dev/null -s -w "%{http_code} $p\n" http://127.0.0.1:7400/api/v1/$p; done`

## Load-bearing rollup — the 6 mission claims, SPA-visibility only

Per `docs/skycot/MISSION.md` (operator-locked) and the function-vs-dressing rule. The CLAUDE.md status block records all 6 as LIVE in the runtime. This rollup asks **a different question**: do the claims work functionally **in the SPA's MVP scope**?

### Claim 1 — Teammate, not tool · 🟡 partially visible

- ✅ `/landing`, `/team`, `/workers/{id}` use teammate-first vocabulary ("5 teammates, one team", traits chips, "How Mariah works", great-at / escalate-when).
- ✅ Workers carry names + personalities, not function-codes (Mariah/Peta/Priya/Jamal/Nori).
- ⚠ `/for/sales` reverts to "worker" (F7). Vocabulary drift on the marketing perimeter.
- ⚠ `/team` headline says "0 working right now" disagreeing with "5 teammates" (F13).
- ⚠ Sidebar nav has both Team + Workers (F5).

Evidence: snapshot of `/workers/{id}` ([2731] StaticText "lists and re-prices items for sale" + [2740] TRAITS "Patient · Detail-oriented · Picture-perfect listings · Polite"); landing `[1596] heading: "Don't buy another tool. Hire a teammate."`

### Claim 2 — Delegation is the unit · 🟡 partially visible

- ✅ `/workflows/{id}` shows step-typed playbooks (TRIGGER → SKILL → DECISION → SKILL) with plain-English summaries: "Slack DM, email, or web inbox carrying photos + price + condition" → "Skill `listing.validate` checks the price against market range, runs photo OCR for category hints, flags below-floor pricing" → "If price is below floor or condition is unverifiable, route to manager review. Otherwise continue." → "Skill `listing.compose` writes a 60-word buyer-facing description..."
- ✅ Builder at `/workflows/build` exposes "Describe a change in plain English" prompt bar with Propose/Add a step/Remove gate.
- ⚠ `/approvals` ops-dashboard renders BUT (a) the inbox surface at `/` shows 0 pending against an empty real-gateway `/api/v1/approvals` so the screen reads "All clear" with 0/0/0m KPIs (F10), and (b) the 4 toolbar buttons on the workflow detail are unreadable in dark mode (F23).

Evidence: workflow detail screenshot, builder palette enumerated (Manual run / Schedule / Webhook / Email / Slack / Web form / Invoke skill).

### Claim 3 — Platform builds workers · 🟡 partially visible, gap

- ✅ `/onboarding/welcome` ships the F1-locked editorial-center wizard with 3-card grid ("Describe the role in voice or text · Upload SOPs, wiki pages, Slack threads · we verify they understand before they start").
- ⚠ The wizard exposes the prompt "What do you need help with?" but the actual workflow-compilation step — turning plain English into a teammate manifest — is not visible from the SPA; you click Continue and presumably the gateway compiles, but the per-step screens beyond Welcome were not walked end-to-end in this audit.
- 🔴 `/settings/providers` is read-only (F16). A user who finishes onboarding can't plug in their own provider keys without leaving the wizard surface.

Evidence: `/onboarding/welcome` screenshot at 1440px; `[3433] heading: "Build your AI workforce."` + textbox `[3448]: "What do you need help with?"`.

### Claim 4 — Skills shared + learned · 🔴 not visible in SPA (install flow missing)

- ✅ `/skills` marketplace renders the A1-locked design (hero search, TRY pills, "Skills my workers learned this month" pill, 6 featured-skill cards).
- 🔴 **Skill detail page has no install / add-to-worker CTA** (F19). Clicking a card shows "No workers are using this skill yet" + "← Back to the marketplace". Scene 4 of the demo runbook narrates "click result → install flow"; **that flow does not exist in the SPA**.
- ⚠ Gateway has 0 skills (`/api/v1/skills` returns `{"skills":[],"total_count":0}` and `/api/v1/skills/marketplace` same). The 6 visible skills are pure SPA mocks.

Evidence: `/skills/approval-route` screenshot; payload `/api/v1/skills/marketplace` → 0 results.

### Claim 5 — Any-PC execution · 🟡 not visible in SPA

- ✅ `/settings/connectors` catalog shows 29 connectors (Airtable / etc, ENTERPRISE tags) with "Connect" CTAs. The substrate-list visibility is there.
- ⚠ No SPA surface visible for computer-use macOS (the v1 demo path per `project_v1_strategic_decisions_2026_05_14`). Likely lives behind the "Run now" button on a workflow with a computer-use step, but I did not exercise that path in this audit because Scene 5 of the runbook is the canonical exerciser.
- ⚠ `/api/v1/connectors` returns 404. The Connectors catalog is rendered from SPA seed data (`web/lib/connectors/seed.ts` or similar), not from the gateway. Operator-blocking per `(c)` in CLAUDE.md's remaining-for-v1.0 list: macOS perms not granted on argo binary.

Evidence: `/settings/connectors` screenshot (hero + first 3 catalog cards: Airtable / [vendor-2] / [vendor-3] all "Not connected · Connect").

### Claim 6 — Multi-channel + multi-tenant · 🟡 partially visible

- ✅ `/workflows/post_listing` first-step copy: "Slack DM, email, or web inbox carrying photos + price + condition" — three channels visible in one trigger description.
- ✅ Builder palette TRIGGERS section enumerates Manual run / Schedule / Webhook / Email / Slack / Web form (6 trigger types).
- ✅ Account menu shows "Sam Harlow · Acmeco · Owner" with workspace name — tenant context visible in the AppShell footer.
- ⚠ No SPA surface for switching tenants. `/api/v1/tenants` returns 404. Multi-tenant **isolation** is invisible from this view (would need a second tenant + cross-tenant impersonation test to see the seam).
- ⚠ "SELF-HOSTED · SINGLE BINARY" pill in the sidebar footer is purely informational; no link to scaling / cloud-tenant docs.

Evidence: workflow detail screenshot; builder palette enumeration.

### Composite verdict

| Claim | Runtime (per CLAUDE.md) | SPA visibility |
|---|---|---|
| 1. Teammate not tool | 🟢 LIVE | 🟡 vocab drift on `/for/*`, two nav rows |
| 2. Delegation is the unit | 🟢 LIVE | 🟡 dark-mode rendering bug on detail/index |
| 3. Platform builds workers | 🟢 LIVE | 🟡 wizard works, provider-keys surface gap |
| 4. Skills shared + learned | 🟢 LIVE | 🔴 install flow missing |
| 5. Any-PC execution | 🟡 perms-pending | 🟡 connectors catalog only |
| 6. Multi-channel + multi-tenant | 🟢 LIVE | 🟡 channels visible, tenant-switch invisible |

The runtime is ahead of the SPA on 5 of 6 claims. The biggest SPA-only gap is Claim 4: a customer browsing the marketplace cannot install a skill.

## Resolved since last audit

This is the first whole-site rollup at this scope; nothing to mark resolved. The 4 sibling agents may have surfaced overlapping findings on specific surfaces (Scene 4 skills, Scene 6 audit) — cross-reference once their reports land.

## Orientation score: 68 / 100

Marketing perimeter is solid (landing/docs/security/changelog/compare/pricing/customers — F6/F7/F8/F9 are polish). Onboarding F1 is locked-design correct. Worker detail C2 personality view is locked-design correct. Settings D top-tabs is locked-design correct. Approvals ops-dashboard is locked-design correct (when accessed via `/approvals`). Skills marketplace A1 is locked-design correct on browse, broken on click. **The drop from 73.9 (May-21 25-audit synthesis) to 68 is driven by**: cohorts design gap, dark-mode card-contrast regression, skill-install missing, audit drill-in placeholder, and the unresolved "Team vs Workers" dual nav. None of those are runtime regressions — they're SPA-implementation gaps the runtime-LIVE status block doesn't surface.
