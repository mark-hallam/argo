# Whole-site MVP — claims-coverage UX audit

**Date**: 2026-05-25 11:00 Melbourne
**Agent**: e2e-ux-journey-test-agent (whole-site / non-scene-focused scope)
**Mode**: READ-ONLY. No code edits, no cargo, no commits.
**Driving**: SPA dev server on `http://localhost:3000` (`NEXT_PUBLIC_ARGO_DEMO_MODE=1`, bypassing the session cookie gate); gateway live at `http://127.0.0.1:7400/health = 200`. Seed data: 1 worker ("Maya", marketing-intern vertical), 16 audit events, 0 approvals, 0 cohorts beyond Maya's pre-trust marketing-intern cohort.

---

## TL;DR

26 findings: **3 P0** (demo-blocking) · **9 P1** · **11 P2** · **3 P3**.

The hard headline: **the v1.0 demo runbook is presently not recordable as scripted** — scenes 1 / 2 / 4 / 5 / 6 each fail at function-side checks (5-worker fleet not seeded; worker-detail throws a React render bug on real backend data; the "competitor pricing scrape" skill and the `customer_research_followup` workflow don't exist in fixture or backend; audit replay falls through to fabricated text). Marketing + onboarding surfaces are strong; in-app surfaces split between two camps — the ones backed by real gateway endpoints (audit, team, approvals KPI strip, verticals, retros) load real data, the ones backed by fixtures only (skills marketplace, workflows list, connectors, billing, orchestration) are pure dressing.

MISSION-claim coverage in SPA: Claims 1 / 2 / 6 are partially substantiated; Claims 3 / 4 / 5 are NOT substantiated in the SPA today (details in §"Claim coverage" below).

---

## Headline blockers

| # | Severity | Title | Scene impact |
|---|---|---|---|
| F1 | **P0** | Every `/workers/{id}/*` page throws a React-render exception against real backend data | Scenes 1 + 2 unrecordable — drill-in from /team to Maya hits a 500 error card, not the C2-personality profile |
| F2 | **P0** | Demo backend has 1 worker ("Maya"), not the 5 (Mariah / Peta / Priya / Jamal / Nori) the runbook narrates | Scene 1 — the "trading-card deck" beat |
| F3 | **P0** | Audit-row drill-in lands on fabricated "Reconstructed original output for {id}" text — gateway has no `/audit/{id}/replay` | Scene 6 — the "click row → real provenance" beat |

---

## All findings

### Functional / claim-blocking

#### F1 [P0] — Worker detail page throws on real `charter` shape

The `(app)/workers/[workerId]/overview/page.tsx` route mounts `<WorkerOverviewContent>`, which passes `overview.charter` (an object `{ text, last_edited_at, last_edited_by, version }` from the gateway) to `<WhatSheDoesCard charter: string>`. React throws: `Objects are not valid as a React child (found: object with keys {last_edited_at, last_edited_by, text, version})`. The page collapses into its error boundary showing "500 — We couldn't load this worker. Try again / Back to team."

Reproduce:
- `/team` → click "Open profile" on Maya (or any worker)
- Page renders "Maya / Idle" hero correctly, then the body throws
- Console: `[skycot:workers/[workerId]] worker detail error: Error: Objects are not valid as a React child`

Schema mismatch:
- Gateway: `crates/argo-cli/src/handlers/gateway/workers.rs` (or wherever `/api/v1/workers/{id}/overview` lives) returns `charter: { text, last_edited_at, last_edited_by, version }`
- SPA: `web/lib/api/worker-overview.ts:61` declares `charter: string`
- Consumer: `web/components/skycot/worker/WorkerOverviewContent.tsx:139` passes `overview.charter` straight to a card expecting a string

A second, separate error fires from the same page: `Attempted to call isWorkerTabId() from the server but isWorkerTabId is on the client` (`web/app/(app)/workers/[workerId]/[tab]/page.tsx`). This is server/client-component boundary mistake — `isWorkerTabId` needs `"use server"` or to be imported from a non-client module. Symptom is the same — error boundary, same UX.

Suggested fix: read `overview.charter.text` in `WorkerOverviewContent.tsx:139`, and update the `WorkerOverview` interface in `web/lib/api/worker-overview.ts:61` to match the actual gateway shape. Separately, move `isWorkerTabId` out of a `"use client"` module.

#### F2 [P0] — Demo backend has 1 worker, runbook narrates 5

`GET http://127.0.0.1:7400/api/v1/team` returns 1 worker: "Maya, marketing-intern". The runbook Scene 1 says `/workers shows 5 trading-card-deck workers (Mariah / Peta / Priya / Jamal / Nori)`. The /team page renders a single magenta-card for Maya — visually a single card, no deck. The starter-prompt 5-grid on /workers (which only shows when the tenant has zero workers) gets bypassed because Maya exists, so the camera lands on a one-card surface.

Reproduce: `curl -s http://127.0.0.1:7400/api/v1/team | python3 -m json.tool` → `"total_count": 1`.

Suggested fix: extend the demo seed CLI (or whatever seeded Maya) to hire the 4 other named workers per the runbook. This is seed data, not code — but until it ships, Scene 1 is unrecordable.

#### F3 [P0] — Audit-row drill-in falls through to fabricated text

Gateway has no `/api/v1/audit/{id}/replay` endpoint (returns 404). The SPA's `replayAction()` in `web/lib/api/replay.ts:79` catches the 404 and returns sample-replay data from `web/lib/sample-replay.ts:144`, which produces `Reconstructed original output for ${actionId}.` and a fabricated diff. The /audit/action/{id}/replay route renders this as if it's real provenance — including a fake "Policy regression detected" red banner and a fake-original-vs-replayed diff that share the SAME body text (`baseOriginal` reused for both).

This is the textbook function-side fabrication the function-vs-dressing rule prohibits: the camera DRILLS INTO an audit row, and the destination is theatre. The runbook explicitly says "real provenance (NOT fabricated mock report)".

Reproduce:
- /audit → click any row (3 hipaa_compliance_check events available)
- Lands on `/audit/action/{uuid}/replay`
- Sees "ORIGINAL: Reconstructed original output for {uuid}." / "REPLAYED: Reconstructed original output for {uuid}."

Suggested fix: either land the gateway `/audit/{id}/replay` endpoint, or short-circuit the drill-in for the demo (render the bare audit row's signed payload as the "provenance" and skip the replay tab). Operator decision.

#### F4 [P1] — `customer_research_followup` workflow doesn't exist (Scene 5)

`/workflows` shows 7 fixture workflows: post_listing, screen_candidate, approve_refund, qualify_lead, triage_ticket, morning_pipeline_digest, clean_customer_list. `customer_research_followup` (Scene 5's hero workflow combining MCP + computer-use) is not in the SPA fixtures (`rg customer_research_followup web/ → no matches`) and gateway has no `/workflows` endpoint either (404). Scene 5 cannot point at this card on the page.

Suggested fix: add `customer_research_followup` to whichever fixture file feeds `/workflows` (the page banner literally says "WORKFLOWS · MOCK", so we know it's fixture-fed), or pick a different signature workflow for Scene 5.

#### F5 [P1] — `competitor-pricing-scrape` skill doesn't exist (Scene 4)

`/skills` marketplace shows 6 skills: approval.route, invoice.extract, candidate.screen, deal.qualify, support.triage, lead.enrich. "competitor pricing scrape" (Scene 4's search beat → expected 3 results) returns 0 results. The route `/skills/competitor-pricing-scrape` does NOT 404 — it renders a generic "no workers using this skill yet" page that echoes the URL slug as the title. So in isolation it looks fine, but the search beat fails (1 result if you search "approval"; 0 if you search "competitor").

Two compounding bugs:
- Skill detail page accepts ANY arbitrary slug (`/skills/this-is-a-totally-fake-skill-id-12345` returns 200 with the slug echoed back) — should 404 on unknown slugs.
- The 3-results beat in Scene 4 needs at least 3 skills matching "competitor" or "pricing" — none of the 6 fixture skills match.

Suggested fix: add 3 competitor/pricing-related skills to the fixture, and gate the skill detail page on slug existence.

#### F6 [P1] — `/approvals` route is a stub; the rich KPI strip lives at `/`

Side-nav "Approvals" points to `/` (per `web/components/skycot/nav-items.tsx:23` — `{ href: "/", label: "Approvals" }`). That route renders the 534-LOC E-ops-dashboard with KPI tiles (78% 7-day rate, 0 pending, 42 resolved/wk, 8m avg response, $— at risk), severity tiers, and a "All clear" empty state.

But the route `/approvals` *also exists* (72 LOC at `web/app/(app)/approvals/page.tsx`) and renders a different, simpler "INBOX / No approvals waiting / Open the inbox / See your workers" surface. There's no redirect — both URLs serve, with two different shells.

This is fine for the demo (camera lands on `/` via side-nav). It's a P1 because: (a) anyone who hits /approvals from a stale link/bookmark/share sees the wrong UI, (b) the locked design memory `project_approvals_page_design_locked` (E ops-dashboard + KPI + severity tiers) doesn't say which surface owns it — and the rich one is at /, the impoverished one is at /approvals.

Suggested fix: either delete `/approvals/page.tsx` and 301 to `/`, or move the KPI shell to `/approvals` and have `/` 301 there.

#### F7 [P1] — Settings hub has 3 "Coming soon" tiles for surfaces onboarding already collects

`/settings` shows 11 tiles. Three are marked "Coming soon":
- **Workspace** — but `/onboarding/workspace` (step 2 of the wizard) creates a workspace. A user who completes onboarding has a workspace they cannot manage anywhere. P1.
- **Provider keys** — but `/onboarding/connect-provider` (step 3) collects Anthropic/OpenAI/etc keys. Same problem — no way to rotate, no way to see what's connected. P1.
- **Notifications** — but the separate tile "Notification channels" (Slack/iMessage/Email/SMS/Voice/Push/web inbox) is live. The two have overlapping names and confusing scope. P1.

Suggested fix: either ship these 3, or remove them from the hub so the user doesn't see broken paths.

#### F8 [P1] — `/audit` design is not the locked B3-Etherscan style

Memory `project_audit_page_design_locked` says the page is "B3-enterprise (Etherscan-style + Inter + pop-out modal)". The live page is a card-list (3 stacked rows: timestamp / verb / id / "Succeeded" pill / arrow). No Etherscan-style table layout, no hash-chain visible, no pop-out modal — clicking a row navigates to a separate `/audit/action/{id}/replay` page, not a modal-over-list. This matches the 25-audit synthesis (`project_25_audit_sweep_complete`) "locked C2/D3 designs not yet ported to JSX" gap.

The camera *will* read this on Scene 6. The card-list is functional and respectable, but it doesn't ship the load-bearing thing — visible hash-chain + provenance.

Suggested fix: implement the locked B3 design before recording. If too costly, accept the card-list as v1 and update the design-lock memory to reflect what shipped (avoid future drift confusion).

#### F9 [P1] — Hero stats on `/team` are dressing labels but contradict the seeded state

`/team` hero band reads: "0 working right now · 187 actions this week · saving you ~12 hours." With the only seeded worker (Maya) at "0 actions today" + "No activity yet.", the "187 actions / 12 hours saved" numbers are a hardcoded fixture label. Per function-vs-dressing rule this is technically dressing — but the camera frames this exact band as it pans to the worker card, and the contradiction (band says 187 actions, card says 0) will read as theatre.

Suggested fix: either gate the band stats on real `team.kpis` summary data (showing 0 / 0 / 0 when the tenant is empty), or remove the band on empty tenants.

#### F10 [P1] — `/workflows` page banner literally says "WORKFLOWS · MOCK"

The /workflows page hero kicker reads `WORKFLOWS · MOCK` in capital letters. This is candid (good!) but reveals to the camera that the entire list is fixture data. The 7 workflows include `BY RIO` and `BY SAM` — two workers who don't exist in /team's roster. If Scene 5 narrates the workflow page, the "MOCK" badge + non-existent worker names break the illusion that the agent runs the playbooks.

Suggested fix: either wire `/workflows` to a real gateway endpoint (currently `/api/v1/workflows = 404`), or remove the "· MOCK" badge and rename the authors to seeded workers before recording.

#### F11 [P1] — 7-of-13 critical gateway endpoints return 404

Probed gateway directly:
```
/api/v1/team           = 200   /api/v1/skills         = 404
/api/v1/cohorts        = 200   /api/v1/workflows      = 404
/api/v1/audit          = 200   /api/v1/connectors     = 404
/api/v1/approvals      = 200   /api/v1/orchestration  = 404
/api/v1/verticals      = 200   /api/v1/billing        = 404
/api/v1/retros         = 200   /api/v1/genesis        = 404
/api/v1/skills/marketplace = 404
```
The 6 missing endpoints all back surfaces the SPA renders today — they all fall through to fixture data. From a "claim works functionally" standpoint, **the SPA today proves Team / Audit / Approvals / Cohorts / Verticals / Retros, and nothing else**.

Suggested fix: this is the v1.0 backend-completion roadmap. Already tracked in MISSION_BACKLOG.md / BUILD_PLAN_V11.md. Flagged here so the demo doesn't try to lean on a 404-fallback surface for a claim-substantiation beat.

### Locked-design / brand fidelity

#### F12 [P2] — `/brand` page tokens contradict the locked brand identity

Per memory `project_skycot_brand_identity`: Audiowide display font, violet→cyan gradient, warm radial-blob bg. The `/brand` page itself uses Inter for headings (`fontFamily: Inter, "Inter Fallback", system-ui...`), shows a lowercase plain "skycot" wordmark sample (no Audiowide), no gradient bg, no warm radial blobs. It's a flat developer-reference table of tokens. (This independently confirms operator's prior-audit F83 about /brand being off-brand.)

Per the function-vs-dressing rule and the operator's note, this is NOT a POC demo blocker because the camera doesn't drill into /brand. Flag as P2 for post-demo cleanup.

Suggested fix: this page is internal-only — either align it with the brand identity (the irony of `/brand` being off-brand is the joke), or move it to `docs/` since dev/designer is the audience.

#### F13 [P2] — `/cohorts` shows 1 tab ("Other"), not the 6+ dept tabs the locked design promises

Memory `project_cohorts_page_design_locked` says "D3 magazine tiles (6 dept tabs + asymmetric lead/stacked/compact layout)". The page only renders tabs for departments that contain cohorts (`web/components/skycot/cohorts/dept-registry.ts:76` — "Empty departments are dropped"). With only Maya's pre-trust marketing-intern cohort seeded, all 7 other depts (Support / Sales / Finance / Marketing / People / IT / Legal) are empty and dropped — only "Other" renders. The page reads as a single-tab list, not a magazine.

The dept registry includes "Project & Ops" (`id: project-ops`) per memory `project_pm_cohorts_peta_patrick` — but no Peta/Patrick cohort is seeded.

Suggested fix: either seed empty cohorts per dept (cheap), or change the empty-dept behaviour to render the tab as a "Pre-populated" empty state (more honest, more design-correct). For the demo MP4 specifically, seed 1 cohort per dept so the magazine layout reads.

#### F14 [P2] — Tab title for `(setup)/onboarding/*` leaks the marketing root title

Every step of the 5-step onboarding wizard shows `<title>SKYCOT — Hire your first AI teammate</title>` (the root layout's marketing default). The (setup) route group has no `layout.tsx` and no per-page metadata. The locked F1 editorial-center design is correct visually — just the browser tab text is wrong.

Reproduce: `/onboarding/welcome`, `/onboarding/workspace`, `/onboarding/connect-provider`, `/onboarding/pick-vertical`, `/onboarding/first-hire` — all show the marketing title. Compare to (app) which has `default: "Approvals", template: "%s · SKYCOT"`.

Suggested fix: add `web/app/(setup)/layout.tsx` with `export const metadata: Metadata = { title: { default: "Set up", template: "%s · SKYCOT" } }`.

#### F15 [P2] — `/brand`, `/retro`, `/verticals`, `/orchestration`, `/billing` inherit "Approvals · SKYCOT" tab title

The (app) group default title is "Approvals" (sensible — /approvals is the root surface), but every sub-route without its own metadata inherits it. Confirmed via `curl` + `<title>` check:
- /retro → "Approvals · SKYCOT" (actual h1: "Retros")
- /verticals → "Approvals · SKYCOT" (actual h1: "Pick the role…")
- /orchestration → "Approvals · SKYCOT" (actual h1: "Orchestration")
- /billing → "Approvals · SKYCOT" (actual h1: "Billing")
- /brand → "Approvals · SKYCOT" (actual h1: "/brand")
- /settings/connectors → "Settings · SKYCOT" (correct via (app) template)
- /settings/notify-channels → "Settings · SKYCOT" (correct)
- /settings/knowledge → "Settings · SKYCOT" (correct)
- /workers → "Workers · SKYCOT" (correct — has own layout)

Suggested fix: drop a `metadata` export in each of the 5 missing pages, or in their sub-folder `layout.tsx`.

### Polish

#### F16 [P2] — `/support` returns 404; child `/support/inbox` works

The route group has `web/app/(app)/support/{inbox/, layout.tsx}` but no `support/page.tsx`. Visiting `/support` yields a Next.js 404, while `/support/inbox` returns 200. There's no nav-bar link to /support (it's not in NAV_ITEMS), so users typically wouldn't hit it — but tab-completion / typed URLs / shared links would. Cosmetic.

Suggested fix: either add a `support/page.tsx` that redirects to `/support/inbox`, or accept the 404 as fine since no nav-entry points here.

#### F17 [P2] — `/skills/{anyslug}` returns 200 with the slug echoed back

Unknown skill slugs are not validated. The skill detail page (`web/app/(app)/skills/[skillId]/page.tsx`) renders a generic shell that uses the URL slug as the title. Test: `/skills/foo-bar-totally-fake-12345` returns 200 with "competitor-pricing-scrape" (well, "foo-bar-totally-fake-12345") as the heading and a "No workers are using this skill yet" stub body.

Combined with F5, this means the runbook can navigate to `/skills/competitor-pricing-scrape` and the page WILL render — but the content is hollow scaffolding, not a real skill page.

Suggested fix: 404 unknown skill slugs (`notFound()` if the slug isn't in the fixture / API).

#### F18 [P2] — Audit page filter defaults to "Today" — backend has 16 events but UI shows only 3

`GET /api/v1/audit?limit=50` returns 16 audit rows. The /audit UI filtering on "Today (default)" shows 3 (5h ago / 5h ago / 6h ago). All 16 events are the same verb (`hipaa_compliance_check`) by the same worker (`unknown`) — monotonous, and the older 13 don't render under the default filter.

For the demo, Scene 6 says "audit renders ≥10 rows". With the default filter, the camera sees 3. The user would need to manually swap to "Last 7 days" to see ≥10.

Suggested fix: change default filter to "Last 7 days" for the demo seed, OR add 7+ more verbs to the seed so "Today" gets ≥10 entries.

#### F19 [P2] — `/compare` table claim-by-claim verifiability not auditable in this pass

The compare table has 23 rows × 5 columns (SKYCOT vs Zapier / n8n / Crew.ai / Lindy). Tick patterns differ (SKYCOT ticks distinct from competitors) so visually the page makes its point. I did NOT verify the factual correctness of each tick — e.g. "Multi-tenant by construction (not bolted on)?" marked YES for SKYCOT only is plausible, but "Visual step timeline with approval gates inline?" needs sanity-check. Per the function-vs-dressing rule, the compare table's feature-tick claims must be honest.

Suggested fix: operator-driven row-by-row sanity check before any public ship (not before the POC demo).

#### F20 [P2] — Marketing pages return preflight CORS 404 from gateway on `/skills` + `/skills/learned-this-month`

`preview_network --filter failed` shows the SPA marketing pages fire CORS preflight (OPTIONS) to `http://127.0.0.1:7400/skills` and `/skills/learned-this-month`, which both return 404. The page still renders (CORS failures don't crash the page; the data is fixture-backed) — but the dev-tools console looks noisy. Not visible on production with prod CSP.

Suggested fix: either drop these fetch calls from the marketing pages, or wire gateway `/skills` (also F11). Probably both.

#### F21 [P2] — `2,847 teams/companies` social proof appears on landing, customer pages, and onboarding

This is the canonical dressing stat per memory `feedback_no_theater_in_demo` (social proof allowed). Operator approved category. No action — calling out for completeness so the operator knows the camera will read it in Scenes 1, 3, and 4.

#### F22 [P2] — `/brand` page mentions the wordmark token as plain-text "skycot"

Inside the locked-tokens reference page (which is internal), the wordmark token displays `skycot` in plain sans-serif (Inter or default). The brand identity is Audiowide. As stated under F12, this is the same root issue — flagged separately because the design-token page is the artifact most likely to be referenced by frontend devs implementing other surfaces and would propagate the wrong wordmark.

### Nits

#### F23 [P3] — Account dropdown shows "Personal workspace" not the workspace name

The account menu in the AppShell shows "mark+ux-walk / Personal workspace" (the demo session). The operator's workspace from onboarding step 2 isn't reflected here. Minor.

#### F24 [P3] — `/team` shows Maya at "LEVEL 16" with "No activity yet"

Maya's trading card has a huge `LEVEL 16` overlay (per the locked C2 holographic showcase design — Workers page memory). With 0 activity recorded, the level number is from fixture data, not earned. Not breaking — but it's a small contradiction the camera reads on Scene 1.

#### F25 [P3] — `/audit` shows "PII redacted — your role shows masked data. Request elevated view →"

This is a real feature (E03 elevated view), but with the demo seed showing only `hipaa_compliance_check` rows by `unknown` worker, there's no PII to redact and no clear thing the elevated view would unmask. Functional but visually anti-climactic.

---

## Claim coverage check — does each MISSION claim "work functionally" in the SPA today?

For each of the 6 MISSION claims (per docs/skycot/MISSION.md), based on what I observed in the SPA at 11:00 Melbourne 2026-05-25:

- **Claim 1 (Teammate, not tool — name, voice, memory, role)**: 🟡 **PARTIAL.** /team renders Maya with a name, role tagline, level, status — the trading-card design exists. BUT the worker-detail page (the C2-personality-profile with trait chips / great-at / escalate-when / signature behaviors) hits F1's React-render bug and renders an error card. Worker has identity in /team, but personality is unreachable in the UI today.
- **Claim 2 (Delegation is the unit — duties + KPIs + charter)**: 🟡 **PARTIAL.** Backend `/api/v1/workers/{id}/overview` returns real `{charter, duties[], kpis[]}` — verified via curl. UI rendering of this data is broken (F1). Once the charter-shape bug is fixed, this becomes 🟢.
- **Claim 3 (Platform builds workers — Genesis from plain English)**: 🔴 **NOT SUBSTANTIATED in SPA.** `/api/v1/genesis = 404`. No "Describe a role" flow that actually compiles to a worker — the hire wizard's starter prompts (`/workers` empty state) route through `/hire` but rely on fixture data. Onboarding step 4 ("pick-vertical") presents 5 prefab roles, not Genesis-from-English. The genesis claim is the headline of `/landing` ("Describe the job in plain English…"), so the SPA promises it but doesn't demonstrate it.
- **Claim 4 (Skills shared + learned — library + marketplace + learning loop)**: 🔴 **NOT SUBSTANTIATED in SPA.** `/api/v1/skills = 404`, `/api/v1/skills/marketplace = 404`. The /skills page renders 6 fixture skills with fake "Used by N workers" stats. /skills/{id} renders a generic shell that works for any slug. No real install flow visible (Scene 4's "click result → install flow" hits a fixture).
- **Claim 5 (Any task a human can do on a PC — MCP + computer-use)**: 🔴 **NOT SUBSTANTIATED in SPA.** No SPA surface demonstrates computer-use today; the Scene 5 workflow (`customer_research_followup` combining MCP + computer-use) doesn't exist. The /workflows page shows 7 fixture workflows; none use computer-use. There's no live-screen-capture surface in the SPA.
- **Claim 6 (Multi-channel approval + multi-tenant by construction)**: 🟡 **PARTIAL.** /settings/notify-channels supports Slack / iMessage / Email / SMS / Voice / Push / web inbox (per /settings hub copy) — multi-channel routing is configurable. Multi-tenant: every gateway call seems tenant-scoped (no cross-tenant data observed), but I did NOT test the cross-tenant negative path in this sweep. Approval flow: /approvals shows "No approvals waiting" — there's no live approval to demo today. The plumbing exists; the demo data doesn't exercise it.

**Summary**: 0 of 6 claims are 🟢 GREEN in the SPA today. 3 are 🟡 PARTIAL (1, 2, 6). 3 are 🔴 NOT SUBSTANTIATED (3, 4, 5).

This is the highest-leverage finding in the audit: **the v1.0 SPA proves about half of what the v1.0 marketing site promises.** If the demo MP4 was intended to "make every claim function-side real on camera", a substantial seed-data + missing-endpoint + workflow-fixture program needs to ship before record.

---

## Resolved since last audit

(none — first run of this audit scope)

---

## Orientation score: 58 / 100

Breakdown:
- Marketing surfaces (landing / customers × 6 / compare / for/* × 6 / pricing / docs / changelog / security / privacy / terms / sla): 22 of 22 routes 200, well-designed, locked brand identity applied → **+18**
- Onboarding F1 editorial-center: gorgeous, matches locked design exactly → **+10**
- In-app surfaces backed by real gateway data (team / cohorts / audit / approvals-as-/ / verticals / retros / orchestration / billing): functional, on-design, no 500s → **+15**
- In-app surfaces backed by fixtures only (skills marketplace / workflows / connectors / brand / templates / skills/{id}): render fine but theatre when the camera drills in → **+5**
- Worker-detail rendering broken on real data (F1): **−12**
- 5-worker fleet missing (F2) + Scene 4 skill missing (F5) + Scene 5 workflow missing (F4): **−7**
- Audit replay fabrication (F3): **−5**
- 6 of 13 gateway endpoints 404 (F11): **−6**

Net 58. Headline: **gorgeous shell, hollow drill-ins.** The marketing + onboarding work could ship to the public today; the in-app demonstration of the 6 claims cannot.

---
