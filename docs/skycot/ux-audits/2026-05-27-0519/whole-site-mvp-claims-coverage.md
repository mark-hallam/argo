# Whole-site MVP claims-coverage rollup — 2026-05-27 05:19 Melbourne

**Scope**: every SPA page NOT in the 6-scene runbook's hot path. Marketing (`/landing`, `/pricing`, `/customers`, `/compare`, `/security`, `/sla`, `/changelog`, `/privacy`, `/terms`, `/for/*`), `/team`, `/brand` + `/brand/components`, `/genesis`, `/cohorts`, `/settings/*` (11 sub-pages), `/workflows/templates`, `/verticals`, `/retro`, `/orchestration`, `/support/inbox`, `/billing` + `/billing/subscribe`, `/docs` + its 6 sub-pages, `/skills-canvas` flat route, theme toggle, viewport responsiveness markers. Inputs to /workers, /workers/[id], /skills, /workflows, /audit, /approvals, /onboarding, /auth deferred to the 4 sibling agents.

**Method**: HTTP probe of 50+ SPA routes (curl), DOM-attribute grep, gateway same-origin proxy probe (browser path), title/H1 extraction per page, file-level metadata-export verification. Preview chrome wedged again on `/cohorts` navigation (consistent with prior T0415 audit) — no interactive screenshots this run; all evidence is server-rendered HTML + gateway/proxy responses + source-of-truth `.tsx` inspection.

**Working tree**: branch `main`, gateway 7400 (schema v81 / FRESH), SPA 3000 (same-origin proxy active per #2429), HEAD `7891db5ff` "evaluator: round 92".

**Time since prior whole-site audit**: 1h04m (T0415 → T0519). The 4 P0s referenced in T0415 DELTA (#2407 skills seed, #2422 mod-rs LOC, #2424 hire-CTAs, #2429 same-origin) are all merged to main. Several T0415 findings re-verified post-merge, and we found that the same-origin proxy IS working but the **skills seed still doesn't populate** for the tenant the SPA resolves to (so F2 from T0415 unfixed at the data layer, not the wire layer).

---

## TL;DR

24 findings: **2 P0** (skills seed still empty post-#2407 — Scene 4 unrecordable; `/docs` is a fan-out to 6 dead 404 sub-links — investor-visible navigational dead-end), **8 P1** (default-title leak now affects 8+ in-app pages including `/billing` and `/support/inbox`, /workers collection endpoint 404 persists, /verticals API empty so the page falls back to fictional roster, /sla + /workflows/templates have bare titles that break the `… · SKYCOT` suffix, /billing/subscribe has no `<h1>` at all, /team has 3 nested H1s, /settings/* sub-pages mostly miss metadata), **8 P2** (settings shell duplicate H1 across all sub-pages, /compare page still 321 KB / 138 RSC chunks, /retro empties to "Loading retros…" SSR, /genesis H1 still reads as hire-flow not onboarding studio, /cohorts SSR renders empty-state hero before hydration even when API returns rows), **6 P3** (preview-only nits + cohorts subtitle plural agreement + changelog ordering).

**6-MISSION-claim functional verdict from the SPA, T0519**:
1 = 🟡 LIKELY-LIVE (banner gone, but workers collection 404 makes Scene 1 fictional roster the visible truth)
2 = ❓ NOT-OBSERVABLE-FROM-SPA (no /handoffs UI surfaced)
3 = 🟡 LIKELY-LIVE (F1 onboarding 5-step flow all 200; Genesis copy mismatch the studio scope)
4 = 🔴 BROKEN-IN-SPA (skills seed empty post-#2407 — same blocker as T0415)
5 = ❓ NOT-OBSERVABLE-FROM-SPA (workflows render; operator-only macOS perms still owed)
6 = 🟡 LIKELY-LIVE (channel/connector pages render; audit page has real rows)

**Direction vs T0415**: flat on 5 of 6 claims; claim 4 unchanged. The CORS-layer headache cleared (good) but exposed the next-layer skills-seed defect more clearly (still blocking). The cohort dept-tab page now has 1 real cohort in the API but the SSR render still shows the "Build your first cohort." empty-state.

**Single biggest unlock for the demo from the browser**: figure out why the boot-time skills seed (`#2407`) doesn't fire against the SPA-resolved tenant `019e5057-1d3f-7fa2-9c1b-c34ef01cc21e`. The catalog is empty server-side, not just in the browser. Once skills populate, Scene 4 is recordable; until then the marketplace renders hero + search bar over an empty grid.

---

## Headline blockers

- **F1 [P0] `/skills` catalog STILL EMPTY through the SPA-resolved tenant after #2407 merged.** `curl http://127.0.0.1:3000/api/v1/skills` → `{"skills":[],"tenant_id":"019e5057-…","total_count":0}`. Same-origin proxy is wired correctly (SPA can read the gateway), but the boot-time seed from #2407 doesn't materialise for this tenant. Scene 4 remains unrecordable.
- **F2 [P0] `/docs` is a fan-out to 6 dead 404 sub-pages.** Investor or pre-record QA who clicks any link from `/docs` lands on the global 404 ("This page could not be found"). `/docs/api`, `/docs/audit`, `/docs/getting-started`, `/docs/integrations`, `/docs/skills`, `/docs/workflows` — all 404. The camera doesn't drill into `/docs` during the 6-scene runbook, but the public marketing surface implies docs exist.
- **F3 [P1] `(app)/layout.tsx:20` default title "Approvals" now leaks across 8+ pages.** Up from 5 in T0415: now also affects `/billing`, `/billing/subscribe`, `/support/inbox`, and `/verticals`. Most-visible polish gap on the in-app shell.
- **F4 [P1] `/api/v1/workers` still 404s while `/api/v1/team` 200s with real data.** Same dispatch defect as T0415's F3 — `#2429` (same-origin fix) exposed it more clearly, didn't fix it. The SPA workers grid falls back to the fixture roster of Mariah/Peta/Priya/Jamal/Nori (5 fictional teammates) while the real workspace has a single worker "Maya".

---

## All findings, in order

### Marketing / public-facing pages

#### F1 [P0] — `/skills` marketplace catalog API STILL returns 0 skills post-#2407 (Scene 4 unrecordable)
Same gateway endpoint that returned `{"skills":[],"total_count":0}` in T0415 (and the prior T0119) still returns the same payload at T0519 — even though commit `a509bc75e` ("fix: seed demo marketplace skill catalog on gateway boot") merged at 18:16Z. The same-origin proxy is wired correctly (the SPA's request reaches the gateway at the right URL with the right tenant header — confirmed by the response containing the SPA-resolved tenant id `019e5057-1d3f-7fa2-9c1b-c34ef01cc21e`). The defect is upstream of the wire: the boot seed simply isn't populating the catalog for this tenant.

Repro:
- `curl -s http://127.0.0.1:7400/api/v1/skills` → 200 with `total_count:0`
- `curl -s http://127.0.0.1:3000/api/v1/skills` (via SPA proxy) → identical 200 with `total_count:0`
- `curl -s 'http://127.0.0.1:7400/api/v1/skills?q=competitor'` → identical
- Open `http://127.0.0.1:3000/skills` → hero + search bar render, marketplace grid is the empty state, 0 skill cards.

Likely root cause options (didn't run cargo per constraints):
- (a) The seed in #2407 runs against tenant "default" but the SPA's `principal_resolve` picks a different tenant on first hit (the workspace tenant `019e5057-…`)
- (b) The seed is gated on a `cfg(feature = "demo")` flag that isn't compiled into the running gateway binary
- (c) The seed runs only on a fresh DB; this gateway boots against a persisted SQLite where the catalog is "already initialised but empty"

File: gateway boot is `crates/argo-cli/src/handlers/gateway/boot.rs`; the seed should be in `crates/argo-marketplace` or `crates/argo-store/src/seed/marketplace.rs` (suggested grep).

Fix: a 1-LOC repro test that hits `/api/v1/skills` against the tenant the SPA resolves to on cold boot, asserting >0 rows. If empty → re-seed regardless of "already initialised" sentinel.

#### F2 [P0] — `/docs` is a 6-link fan-out to dead 404 sub-pages
`/docs` returns 200 with H1 "Everything you need to ship". The 6 first-class links it advertises:
- `/docs/api` → 404
- `/docs/audit` → 404
- `/docs/getting-started` → 404
- `/docs/integrations` → 404
- `/docs/skills` → 404
- `/docs/workflows` → 404

Repro:
- `curl -sLI http://127.0.0.1:3000/docs/api | head -1` → `HTTP/1.1 404 Not Found` (etc. for all six)
- Open `http://127.0.0.1:3000/docs` → click any header link → "This page could not be found"

The 6-scene runbook doesn't navigate to `/docs` (dressing-side); but `/docs` is linked from the marketing `/landing` footer + the in-app HelpWidget. A pre-record QA pass that clicks "Docs" from the help widget hits 6 dead links. The investor-facing trust signal "we have docs" is misleading until those pages exist.

File: `web/app/docs/page.tsx` (the index that links them) — either the sub-routes should be created OR the links should be removed/redirected to `/changelog` + `/security`.

Fix: simplest — comment out the 6 link cards on `/docs` until the sub-routes ship. Aspirational — create stub MDX or external-link redirects to `github.com/skycot/skycot/blob/main/docs/...`

#### F5 [P2] — `/sla` STILL falls back to global default title (T0415 F5 unresolved)
The HTML `<title>` reads `SKYCOT — Hire your first AI teammate` instead of `SLA · SKYCOT`. Same fix as T0415: add `export const metadata = { title: "Service-level agreement" }` to `web/app/sla/page.tsx`.

#### F6 [P2] — `/compare` still 321 KB / 138 RSC stream chunks
6× the streaming traffic of `/landing` (57 chunks) and 14× that of `/pricing` (22 chunks). T0415 flagged this; not yet addressed. Lighthouse won't love it. POC demo doesn't visit `/compare` so this is a public-marketing perf issue rather than a recording blocker — flag only.

#### F7 [P3] — `/customers` logos still stub avatars (T0415 F7 unresolved)
Per memory `feedback_logos_from_svgl_app`, customer logos must be SVGs from svgl.app, stored at `web/public/logos/<name>.svg`. The page renders 5 fictional customers (Northwind / Beacon / Maple / Apex / Skyline) with `<span>` placeholder badges. Dressing-side; flag only.

#### F8 [P3] — `/changelog` shows 13 entries in reverse-chronological. No "What's next" / "Roadmap" cut-line. Polish only.

#### F9 [P3] — `/pricing` tier cards link to `/auth/signup?tier=pro` (resolved vs T0415 F10)
Previously flagged as missing a Subscribe / Choose Pro affordance. The current implementation deep-links to `/auth/signup?tier=pro` (good). Listed for completeness — closed since T0415.

### In-app pages (outside the 6-scene hot path)

#### F3 [P1] — `(app)/layout.tsx:20` default title "Approvals" leaks across 8+ pages
Up from 5 pages in T0415:
- `/brand` (H1: "SKYCOT") — title reads "Approvals · SKYCOT"
- `/brand/components` (H1: "Design-system components") — title reads "Approvals · SKYCOT"
- `/retro` (H1: "Retros") — same
- `/genesis` (H1: "Hire your next teammate.") — same
- `/orchestration` (H1: "Orchestration") — same
- `/verticals` (H1: "Pick the role.") — **NEW since T0415**, same leak
- `/support/inbox` (H1: "Support inbox") — **NEW**, same leak
- `/billing` (H1: "Billing") — **NEW**, same leak
- `/billing/subscribe` (no H1) — **NEW**, same leak

The metadata template at `web/app/(app)/layout.tsx:18-22` is:
```ts
title: {
  default: "Approvals",
  template: "%s · SKYCOT",
}
```
Pages that don't export `metadata` fall through to the literal default "Approvals". On `/genesis` the tab reads "Approvals" while the page shows the Genesis studio — and Genesis is operator-locked as a v1.0 investor moment.

Fix: change default to `"SKYCOT"` AND add `export const metadata` to all 9 orphan pages (the (app) group has rapidly grown — this default needs to be neutral, not load-bearing).

#### F4 [P1] — `/api/v1/workers` collection endpoint still 404s (T0415 F3 unresolved)
- `curl -s http://127.0.0.1:7400/api/v1/team` → 200 with the real worker "Maya"
- `curl -s http://127.0.0.1:7400/api/v1/workers` → 404 `{"error":"not found"}`

Both routes are supposed to resolve to the same handler (per `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs:14` delegating to `routes_team_list::list_response`). Worth a real Rust read — the dispatch is broken somewhere between `routes_dispatch_get_api.rs` and the route table.

The user-visible consequence: the SPA workers grid renders the fictional `sample-workers-detail.ts` roster (Mariah/Peta/Priya/Jamal/Nori) instead of the real "Maya". Scene 1 of the runbook narrates a roster the operator never hired.

File: `crates/argo-cli/src/handlers/gateway/routes_workers_list.rs` + `routes_dispatch_get_api.rs::dispatch_api_get`.

#### F10 [P1] — `/api/v1/verticals` returns empty `{"verticals":[]}` while the page renders 6 fixture verticals
- `curl -s http://127.0.0.1:3000/api/v1/verticals` → `{"verticals":[]}`
- `/verticals` page renders 6 vertical-template cards (Sales, Support, Marketing, etc.)

The page presumably falls back to a fixture import (`web/lib/sample-verticals.ts` or similar) when the API returns empty. The result is "verticals shown on screen are fictional and the user can pick one but it won't onboard to a real workspace vertical". Scene 3 (`/onboarding/pick-vertical`) likely shares the same data source — could be the wire defect that causes the onboarding flow to "work" with non-real verticals.

Likely fix: the same boot-seed gap as F1 — `/api/v1/verticals` is meant to be seeded at boot with the 5 v1 verticals (sales / support / marketing / recruiting / finance) but isn't.

File: gateway boot + `crates/argo-vertical-templates` registration.

#### F11 [P1] — `/sla` AND `/workflows/templates` break the `%s · SKYCOT` title chain
Two distinct culprits:
- `/sla` (already in F5) — no metadata export, falls back to root layout's marketing default.
- `/workflows/templates` — exports `metadata.title = "Workflow templates"` (a string literal at `web/app/(app)/workflows/templates/layout.tsx:5`). Next.js treats this as an override that DOESN'T inherit the `%s · SKYCOT` template from the (app) layout. The HTML title reads "Workflow templates" without the SKYCOT suffix.

Inconsistent with siblings (`/cohorts`, `/audit`, `/skills`, `/workers`, `/team` all read `<page> · SKYCOT`). The browser-tab-bar fingerprint changes across pages — small but the demo MP4 captures the tab bar on every shot.

Fix: `title: { absolute: "Workflow templates · SKYCOT" }` OR change to plain `"Workflow templates"` and rely on the (app) layout's template (which requires the property to be a string default, not absolute).

File: `web/app/(app)/workflows/templates/layout.tsx:5`.

#### F12 [P1] — `/billing/subscribe` has **no `<h1>`** at all
HTML inspection: 0 `<h1>` elements; title reads "Approvals · SKYCOT" (because no metadata export). Page is a 61 KB pricing tier picker with content like "Free / Pro / Enterprise" but no top-level heading. Accessibility violation (screen-reader users can't orient via top-of-document heading); demo trust signal violation (camera would have nothing to read aloud).

File: `web/app/(app)/billing/subscribe/page.tsx`.

Fix: add an `<h1>` like "Choose a plan" + `export const metadata = { title: "Subscribe" }`.

#### F13 [P1] — `/team` has 3 `<h1>` elements on one page
- H1#1: `<h1 class="text-xl font-semibold tracking-tight">Team</h1>` (app-shell breadcrumb header — duplicated by appshell)
- H1#2: `<h1 class="skycot-brandloud-title">Hire your first teammate.</h1>` (brand-loud hero in empty state)
- H1#3: `<h1 class="text-xl font-semibold tracking-tight">Team</h1>` (a 2nd nav-shell label lower in the doc, suggests double-mount)

Three H1s is a hard semantic-HTML violation and breaks heading-navigation (`H` key) for screen-reader users. The duplicate "Team" H1 indicates `AppShell` renders the page title twice OR a child component re-uses the same `<h1>` tag.

File: `web/app/(app)/team/page.tsx` + `web/components/skycot/AppShell*.tsx`.

#### F14 [P2] — `/cohorts` SSR renders empty-state H1 ("Build your first cohort.") even when API has 1 cohort
The cohorts page server-renders with `cohorts.length === 0` → `<h1>Build your first cohort.</h1>` from `CohortsHero` at `web/app/(app)/cohorts/page.tsx:222`. After hydration, the `useCohorts()` query returns 1 cohort (`marketing-intern::unknown`) and the magazine grid renders. So:
- Initial paint: brand-loud "Build your first cohort." empty-state hero
- Post-hydration: 1-cohort magazine grid

Demo recording captures the initial paint until the JS hydrates. The camera sees ~200-500ms of "you have no cohorts" before the grid appears. Solvable by making the empty-state conditional on `cohortsQuery.isSuccess && cohorts.length === 0` (not on `cohorts.length === 0` alone, which triggers during pending too).

File: `web/app/(app)/cohorts/page.tsx:221-226` (the `CohortsHero` `if (total === 0)` branch).

#### F15 [P2] — `/cohorts/subtitle` reads "0 cohorts across the fleet" SSR → "1 cohort across the fleet" post-hydration
Same hydration-mismatch class as F14. Subtitle pluralises correctly (cohort vs cohorts) but the SSR-rendered value (0) is wrong against the real workspace (1). Visual blink in the first frame after hydrate. Polish.

#### F16 [P2] — `/settings/*` sub-pages each render a duplicate "Settings" `<h1>` (T0415 F17 unresolved)
Verified across all 11 settings sub-pages: 8 of them have 2 `<h1>`s (a sticky shell "Settings" + the page-specific H1). The exceptions (1 H1 only) are `/settings`, `/settings/billing`, `/settings/notifications`, `/settings/providers`, `/settings/workspace` — these don't carry a page-level H1 (so the shell "Settings" is the only H1, but then those pages have no clear top heading at all). Either pattern is a soft semantic violation.

Fix: demote the shell "Settings" label to `<h2>` (and the sub-page H1 stays the primary), OR move the shell heading into a `<header role="banner">` without an `<h1>`.

File: `web/components/skycot/AppShell*.tsx` (the shell that renders the "Settings" duplicate).

#### F17 [P1] — 7 of 11 `/settings/*` sub-pages have no metadata export, so the tab reads "Settings · SKYCOT" instead of "Connectors · SKYCOT" / "Workspace · SKYCOT" / etc.
Metadata exports present only for: `api-keys`, `data-export`, `members`, `security`.
Missing for: `billing`, `connectors`, `knowledge`, `notifications`, `notify-channels`, `providers`, `workspace`.

`/settings/billing` is the second worst offender after F12 (no H1 + no metadata).

Fix: add `export const metadata = { title: "<page-name>" }` to each of the 7 missing pages. Mechanical.

#### F18 [P2] — `/genesis` H1 ("Hire your next teammate.") still mismatches the operator-clarified studio scope (T0415 F21 unresolved)
Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (operator clarification 2026-05-25T11:40Z): Genesis takes voice/text role + SOP-PDF/wiki/Slack uploads + access-controls + comprehension verification. The current H1 reads as a hire-flow CTA, not an onboarding studio.

The current implementation has the right input (`<textarea data-testid="genesis-role-input">` with placeholder "Describe the role in plain English…") + a Generate button — function is there. The H1 copy doesn't match the studio framing.

Suggest: H1 "Onboard a new teammate." or "Build a teammate from a role description."

File: `web/app/(app)/genesis/page.tsx`.

#### F19 [P2] — `/retro` SSR renders "Loading retros…" with no fallback empty-state
HTML at H2 level: "Loading retros…" — the page is fetching but the SSR snapshot captures only the loading skeleton. If the operator drills into `/retro` from the app nav during a demo, the camera sees "Loading…" for the first frame (and possibly the next frame if the fetch hasn't resolved). Not in the 6-scene runbook so non-blocker; polish if `/retro` becomes a feature surface.

#### F20 [P2] — `/skills-canvas` flat route still 404s (T0415 F19 unresolved)
The route exists only at `/workers/[workerId]/skills/[skillId]/canvas`. Briefs in `streams/` reference `/skills-canvas` as a marketplace-level surface. Either: (a) the flat route was never created and briefs should be updated, OR (b) the flat route should redirect to the most-relevant nested canvas. The e2e-ux-journey-test agent prompt also references `/skills-canvas` as if it existed.

Fix: a 1-LOC redirect at `web/app/(app)/skills-canvas/page.tsx` to `/workers` (or remove the brief references).

#### F21 [P3] — `/orchestration` and `/retro` aren't linked from the sidebar nav
Both pages are reachable via direct URL but absent from the app-shell sidebar. They appear to be operator-only / power-user surfaces. Not a bug; informational.

#### F22 [P3] — `/brand` and `/brand/components` are URL-only (T0415 F20)
Brand + components live only at URL level (designers + dev), not visible to operators. Probably intended.

### Theme + viewport + console hygiene

#### F23 [P2] — Theme provider correctly wired, FOUC-suppression script present, but interactive verification still blocked by preview wedge
HTML inspection confirms `<ThemeProvider attribute="class" defaultTheme="system">` from `next-themes` at `web/app/layout.tsx:114`, with the standard inline script that sets the `class` on `<html>` server-side to prevent flash. The Preview chrome wedged on `/cohorts` navigation (same failure mode as T0415), so I couldn't interactively click the toggle. Implementation looks correct.

Quick smoke before recording: open `/audit` in dark theme, take screenshot, click toggle, confirm light renders without a flash. ~2 min manual; closes the verification gap.

#### F24 [P3] — Marketing pages DO have a hamburger affordance (T0415 F24 closed)
Inspected HTML: `data-testid="marketing-nav-open"` with `aria-haspopup="dialog"` is present on `/landing` (and presumably the other marketing pages, sharing a layout). T0415 flagged this as "may not exist" — now confirmed present. Could not test at 375px due to preview wedge, but the markup is right.

---

## 6-MISSION-claim coverage rollup (the load-bearing summary)

Reminder: per the function-vs-dressing rule, "LIVE in the SPA" means a path the demo camera traverses produces real (non-fixture) data and renders the locked design. "LIKELY-LIVE" means the page renders + the backend cold tests pass + the same-origin proxy is wired (post-#2429), but at least one wire defect prevents end-to-end verification from the browser.

### Claim 1 — Teammate, not tool 🟡 LIKELY-LIVE (improved from T0415 🔴)

**Evidence**:
- `/workers` page renders the C2 trading-card-deck layout (Audiowide hero + 5 worker cards in the SSR HTML).
- `/team` page renders with real data: API returns 1 worker "Maya" (`role.vertical = marketing-intern`, "Marketing intern that drafts and asks before publishing.")
- BUT the workers visible on `/workers` are still from `sample-workers-detail.ts` (Mariah/Peta/Priya/Jamal/Nori), because the SPA hits `/api/v1/workers` (404) NOT `/api/v1/team` (200).
- Demo-mode banner: absent from SSR (vs T0415 where it was visible). Could be hydration-gated; can't verify without preview.
- Backend round-19 + round-54 evaluator: claim 1 LIVE.

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/team` returns `{"total_count":1,"workers":[{"name":"Maya","role":{"tagline":"Marketing intern that drafts and asks before publishing.","vertical":"marketing-intern"},...}]}`. `curl -s http://127.0.0.1:7400/api/v1/workers` returns `{"error":"not found"}`.

**Demo-recording implication**: Scene 1 narrates "Mariah, Peta, Priya, Jamal, Nori" — a fictional roster. The real workspace has "Maya". Either the demo seed CLI needs to actually be run with the 5-persona roster (per CLAUDE.md's "remaining for v1.0 launch" item (e)), OR the runbook narration shifts to "Maya, your marketing intern, who…". Fix F4 (route alias) + run seed CLI with personas = recordable.

### Claim 2 — Delegation is the unit ❓ NOT-OBSERVABLE-FROM-SPA

**Evidence**:
- No `/handoffs`, `/delegations`, or visible UI surface for delegation chains in the SPA route tree.
- The audit page is supposed to render handoff-chain hash hovers (per `current_handoff_id` per CLAUDE.md round-54 wire); not directly observable in the SSR HTML without driving a multi-step delegation.
- Backend round-54: claim 2 LIVE (delegate-task fallback-chain inheritance wired in live runner dispatch via #2256).

**Citation**: searched for delegation surfaces; nothing in `web/app/(app)/` matches `delegat*`, `handoff*`. The audit page (`web/app/(app)/audit/page.tsx`) is the closest UI but doesn't dedicate UI to handoff chains.

**Demo-recording implication**: Scene 6 audit drill-in is where delegation chains should surface. The /audit endpoint returns real rows; whether the row-detail drawer (or pop-out per locked B3 Etherscan design) visualises the handoff edge isn't testable from server-rendered HTML.

### Claim 3 — Platform builds workers 🟡 LIKELY-LIVE (flat vs T0415)

**Evidence**:
- All 5 onboarding steps return 200 with the F1 editorial-center layout: `/onboarding/welcome` → `/workspace` → `/connect-provider` → `/pick-vertical` → `/first-hire`.
- `/hire` page returns 200 with title "Hire — SKYCOT".
- `/genesis` page renders the studio shell with the right inputs (`<textarea data-testid="genesis-role-input">` + Generate button) — H1 copy is mismatched per F18 but function is in place.
- BUT `/api/v1/verticals` returns `{"verticals":[]}` (F10). So `/onboarding/pick-vertical` would show fixture verticals — not real onboarding-result verticals.
- Backend round-54: claim 3 LIVE.

**Citation**: `curl -s http://127.0.0.1:3000/api/v1/verticals` → `{"verticals":[]}`. All 5 onboarding pages confirmed 200 via curl.

**Demo-recording implication**: Scene 3 is recordable but the operator should expect the `/onboarding/pick-vertical` step to read from fixtures, not from a real seeded vertical registry — investigate F10 before recording or be ready for the narration to skip the verticals step.

### Claim 4 — Skills shared + learned 🔴 BROKEN-IN-SPA (flat vs T0415)

**Evidence**:
- `/skills` page renders the A1 marketplace hero + search bar (per locked design).
- Skills API STILL empty: `curl -s http://127.0.0.1:3000/api/v1/skills` → `{"skills":[],"total_count":0}` (same-origin proxy works; gateway upstream returns empty).
- `#2407` (seed catalog on gateway boot) MERGED 18:16Z — but the catalog is still empty for the SPA-resolved tenant 1h+ later.
- `/skills-canvas` flat route 404s (F20).
- Backend cold rerun 2026-05-25: `claim_4_live_install_invoke` cargo test 1m53s PASS.

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/skills` → 200 `{"skills":[],"total_count":0,"tenant_id":"019e5057-1d3f-7fa2-9c1b-c34ef01cc21e"}`. Confirmed identical at both gateway-direct + SPA-proxy URLs.

**Demo-recording implication**: Scene 4 (search "competitor pricing scrape" → 3 results) is unrecordable until F1 lands. This is the **single biggest gap to a recordable demo MP4** since the same-origin fix shipped. Backend tests pass; the gateway just doesn't have the rows.

### Claim 5 — Any-PC execution ❓ NOT-OBSERVABLE-FROM-SPA (flat vs T0415)

**Evidence**:
- `/workflows/templates` lists 50+ template cards in the SSR HTML (template H2s include `qualify_lead`, `competitor_intel_digest`, `book_demo`, `triage_ticket`, etc.).
- `/workflows/build` returns 200 (presumably the visual builder).
- Workflow detail pages exist at `/workflows/[verticalId]/[name]` (e.g. `/verticals/sales` returns 200).
- Computer-use macOS executor depends on operator-only macOS permissions (CLAUDE.md "Remaining for v1.0 launch" item (c)).
- Backend round-51: claim-5 substrate 47/47 cold cargo PASS.

**Citation**: 50+ `<h2>` elements named `qualify_lead`, `draft_outreach`, etc. on `/workflows/templates`. Search for `customer_research_followup` (the runbook's named workflow): not present in the template H2 list — the runbook may need to pick a different workflow OR the template needs to be added.

**Demo-recording implication**: Scene 5 has a viable workflow surface and a builder; runbook may need to swap `customer_research_followup` for an actually-present template like `competitor_intel_digest`. Computer-use end-to-end still gated on operator macOS perms.

### Claim 6 — Multi-channel + multi-tenant 🟡 LIKELY-LIVE (flat vs T0415)

**Evidence**:
- `/settings/notify-channels` returns 200 with the channel-config UI.
- `/settings/connectors` returns 200 with 8+ integration cards in the SSR.
- `/audit` returns 200 with real audit rows (e.g. one with `action_type: "audit.detail.viewed"`, `id: 019e659ee80f7440bcd5f2b337114aad`).
- Cross-tenant negative pin shipped via #2257 (per CLAUDE.md Status block).
- Backend cold rerun 2026-05-25: `claim_6_live_cross_tenant` 52.42s PASS.

**Citation**: `curl -s http://127.0.0.1:7400/api/v1/audit | head -c 400` returns a JSON `{items:[…]}` with real `action_type`, `timestamp`, `data_classification` fields. Settings sub-pages confirmed 200 via curl.

**Demo-recording implication**: Scene 6 audit drill-in is recordable. Pending verification that the audit row-detail drawer renders provenance + cross-tenant negative test surfaces somehow — but those are deeper than a server-side HTML probe can show.

---

## Resolved since prior audit (T0415)

- **F1 (T0415 P0 — CORS blocked)** RESOLVED via #2429 same-origin proxy. Verified: `curl http://127.0.0.1:3000/api/v1/skills` reaches gateway, returns 200.
- **F10 (T0415 P2 — `/pricing` tier cards no Subscribe affordance)** RESOLVED. Tier cards now link to `/auth/signup?tier=pro`.
- **F24 (T0415 P3 — marketing mobile nav existence)** RESOLVED — `data-testid="marketing-nav-open"` confirmed in HTML.

Persisting unfixed from T0415:
- F2/F4 (skills seed empty, `/api/v1/workers` 404) — unfixed at data layer
- F4 (default-title leak) — now affects 8+ pages, up from 5
- F5 (`/sla` no metadata)
- F6 (landing H1 line break, mobile-only)
- F7 (customer logos stub)
- F8 (compare page bloat)
- F12 (onboarding step pages no metadata) — partial (not re-walked this run; deferred to onboarding-agent's report)
- F14 (cohorts empty-state vs API content) — same SSR/hydration mismatch
- F17 (settings shell duplicate H1)
- F18 (settings security marketing-register H2s)
- F19 (`/skills-canvas` flat 404)
- F21 (`/genesis` H1 mismatched scope)

---

## Orientation score

**63 / 100.** (vs T0415 = 62)

Reasoning: the same-origin proxy landing (#2429) is a structural win that broke the demo-mode banner across every signed-in page and meant the SPA's data layer can now reach the gateway honestly. But the next layer down is unblocked at the wire and blocked at the data: skills catalog empty, verticals empty, workers collection 404. The 1-point improvement reflects that we've broken through one layer (CORS) without breaking through the next (data seeding).

The locked designs (F1, D3, C2) are all in JSX and SSR cleanly. Most of the remaining P2/P3 is metadata polish that doesn't affect the demo MP4 functionally — but does affect the **camera-readable browser tab bar** that the recorder captures on every shot.

**One-bullet recommendation for the operator**: the single highest-leverage next fix is to figure out why `#2407`'s boot-seed doesn't populate the catalog for tenant `019e5057-1d3f-7fa2-9c1b-c34ef01cc21e`. That's both the F1 blocker AND likely the root cause of F10 (verticals empty) — same boot-seed pattern. Without skills + verticals real data, Scenes 3 + 4 + 5 all stay in "render fixtures, hope nobody clicks" mode.
