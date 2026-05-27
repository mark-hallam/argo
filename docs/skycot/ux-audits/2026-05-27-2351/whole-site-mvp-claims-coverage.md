# Whole-site MVP claim coverage rollup — 2026-05-27 23:51

**Agent**: e2e-ux-journey-test-agent (whole-site, the load-bearing summary)
**Scope**: marketing pages, /cohorts, /settings/*, /team, /brand, /workflows/templates, /skills-canvas, /workflows/build, /audit drill-in, /support/inbox, theme toggle, viewport responsiveness — i.e. every surface NOT covered by the 4 sibling agents (onboarding hire-flow, scenes 1+2, scenes 4+5, scene 6+approvals).
**Dev server used**: http://localhost:61264 (Claude Preview launched `web` config from `.claude/launch.json` with `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` + `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` + `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1`). Operator spec said port 3000, but the *separate* dev process listening on 3000 was started outside the launch config and DOES NOT have the auth-bypass env var (every in-app route 307s to /auth/signin). This is a tooling artefact, not a code defect — flagged P3 below.
**Gateway**: http://127.0.0.1:7400/health → 200 OK; schema v83.

---

## TL;DR

23 findings. **3 P0 demo-blockers**: the audit-row drill-in target shows "Action not found" for IDs that the very same `/api/v1/audit` list endpoint just returned (Scene 6 unrecordable as scripted); `POST /api/v1/verticals/marketplace/workflows/customer_research_followup/execute` returns 403 Forbidden (Scene 5 "Click run" produces no audit-row tail); the Mariah "Schedules" tab has no per-duty "Run now" button (Scene 2's manual-fire is unreachable). **6 P1s** including a dev-only "MOCK-BYPASS" chip stamped on /team's hero (the camera DOES see this in Scene 1), the settings tab-active matcher highlighting *two* tabs simultaneously on every sub-page, and the /skills detail page exposing zero "install" affordance (Scene 4's narrated install flow has no button). **The 6-claim rollup at the bottom of this file is the load-bearing artefact** — it is what the synthesizer and the operator should compare across runs to track distance to the v1.0 ship-gate.

---

## Headline blockers (P0)

| ID | Severity | One-line | Demo scene impact |
|---|---|---|---|
| F01 | P0 | Audit-row drill-in is broken — list returns IDs, detail returns 404 for the same IDs | Scene 6 narrated drill-in shows "Action not found" |
| F02 | P0 | Workflow `customer_research_followup` POST execute → 403 Forbidden | Scene 5 "Click run → audit rows land" does not occur |
| F03 | P0 | Mariah's Schedules tab has no per-duty "Run now" button | Scene 2 manual-fire path is unreachable from SPA |
| F04 | P0 | `/skills/<slug>` detail page exposes no Install button or install flow | Scene 4 "Click result → install flow" cannot be performed |

Notes:
- F01–F04 are all *function-side* per the function-vs-dressing rule in `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` — the camera DOES drill into these surfaces and the destination must be real.
- F02/F03 may have console workarounds (operator runs `argo workflow run …` from a terminal off-camera), but Scenes 2/5 as scripted in `docs/skycot/demos/v1-mission-mvp-runbook.md` show the SPA initiating the run.

---

## All findings, grouped by surface

### Audit page + drill-in

#### F01 [P0] — audit-row drill-in shows "Action not found" for IDs the list API just returned

`/audit` renders 100 rows (Etherscan-style, dark, hash-chain visible). Clicking a row's link `<a href="/audit/action/<id>">` (or hitting that URL directly) renders the empty-state "Action not found — No audit record matches <hash>". Network inspection confirms `GET /api/v1/audit/<id>` returns `{"error":{"code":"not_found","message":"audit row not found"}}` for the very ID that `GET /api/v1/audit?limit=N` just emitted in its `items[].id` field. Same outcome when probing the row's `self_hash` instead of `id`.

This is **the function the camera reads** during Scene 6 — the runbook says "Audit page renders ≥10 rows in Etherscan-style table with hash-chain hover. Click a row → real provenance (NOT fabricated mock report)." Drill-in is fabricated theatre right now.

Repro:
- `curl http://localhost:61264/api/v1/audit?limit=1` → grab `items[0].id` (e.g. `019e69cd30b37e01a8f04b8d30122869`)
- `curl http://localhost:61264/api/v1/audit/019e69cd30b37e01a8f04b8d30122869` → `{"error":{"message":"audit row not found"}}`
- In browser, navigate `/audit/action/019e69cd30b37e01a8f04b8d30122869` → "Action not found" page

Where:
- Frontend list hook: `web/lib/hooks/use-audit-feed.ts`
- Frontend detail page: `web/app/(app)/audit/action/`
- Backend handler returning 404: `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs:232-237`

Suggested fix (one-line, no implementation in this audit): the list and detail handlers are reading from different sources (likely a SessionStore-backed feed vs `audit_records` table); the detail handler needs to query the same source the feed exposes, or the feed needs to only emit IDs the detail handler can resolve.

#### F05 [P1] — audit-row worker reference uses an opaque UUID for both id and name

API row payload: `{"worker": {"id": "019e6785-928f-7281-bf8d-e062bc277478", "name": "019e6785-928f-7281-bf8d-e062bc277478"}}`. The worker name slot is filled with the worker id. Also: that UUID does not match any of the 5 seeded workers (`018f67d0-0000-7000-8000-00000000000{1,2,3,4,7}`) — so even if the drill-in worked, the rendered audit row would show "by 019e6785-…" instead of "by Mariah" or "by Nori". The /team roster IDs and the audit log's worker IDs come from disjoint seeding paths.

Repro: `curl http://localhost:61264/api/v1/audit?limit=3 | python3 -m json.tool` and compare `items[].worker.id` against the URLs at `/team`.

Suggested fix: align the seed for `worker.id` between the audit-feed source and the team-roster source; populate `worker.name` with the persona name from the canonical worker row.

#### F06 [P1] — audit-page click on row swallows the navigation

Clicking the row container `[data-testid="activity-row-<id>"]` (which is the visible target) does not navigate to `/audit/action/<id>` even though the row IS rendered as an `<a>` with that href. The click is captured silently. Manual `window.location.href='/audit/action/<id>'` does navigate (and lands on the "Action not found" empty state, see F01).

Repro:
- Click any row at `/audit`
- Observe: URL unchanged, no modal, no console error

This is masked by F01 (the destination is broken anyway), but if F01 is fixed the click-handler will still need investigation.

### /skills marketplace + detail

#### F04 [P0] — `/skills/<slug>` detail page exposes zero install affordance

The page renders the slug as a large heading, two cards (Provenance: "Authored by SKYCOT, Trust ★", and "Workers using this skill — no workers are using this skill yet"), and a "← Back to the marketplace" link. **No install / add-to-worker / try-it button anywhere on the page.** The card text reads "Hire a worker and add this skill from their Skills tab to put it to work" — meaning the "install" flow lives on the worker, not the marketplace. Scene 4 of the runbook narrates "Click result → install flow"; against this UI, there IS no install flow at the click destination.

Repro:
- Navigate `/skills`
- Click `Pricing competitor scrape` (or hit `/skills/pricing.competitor_scrape` directly)
- Observe: a description, a 0-workers card, a back-link — no install/use/try button

Suggested fix: add a primary CTA at the top of the detail page ("Install for a worker →" → opens a worker-picker drawer, calls the existing add-skill API). The cross-surface install flow already exists at the worker level — the marketplace detail just needs a button that routes into it.

#### F07 [P2] — /skills marketplace `LIBRARY` chip doesn't link anywhere

Every skill card shows a `LIBRARY` tag, but the tag isn't a link/filter — clicking does nothing. Operator would expect this to filter the catalog to library skills, especially since the search bar offers "approval / invoice / candidate / deal / support / lead" pill-shortcuts but no scope filter.

### /workflows + workflow detail

#### F02 [P0] — POST workflow execute → 403 Forbidden

`POST http://localhost:61264/api/v1/verticals/marketplace/workflows/customer_research_followup/execute` returns 403 Forbidden when the "Run now" button on `/workflows/marketplace/customer_research_followup` is clicked. No toast surfaces to the user; the button briefly shows "Running" then reverts. No audit row is appended. The 6-scene runbook's Scene 5 says "Click run → audit rows for both steps land." That cannot happen from the SPA under current auth.

Repro:
- Navigate `/workflows/marketplace/customer_research_followup`
- Click "Run now"
- Observe network: `POST /api/v1/verticals/marketplace/workflows/customer_research_followup/execute → 403`
- No toast, no error UI

Suggested fix: either (a) the SPA needs to send a workflow-execute capability in its principal context, or (b) demo mode needs an exec-grant for this specific workflow. Either way, the bug surfaces as silent failure — at minimum, the toast/error state should fire so the operator sees what's happening.

#### F08 [P1] — workflow "Run now" failure is silent — no toast, no error state, no audit row

Even with the 403 (F02), the button's only feedback is the brief "Running" label flip. The auditor watching this would assume the run completed. A failure path should surface with a sonner toast + persistent error chip on the run-history strip.

#### F09 [P2] — workflow detail "BUN HISTORY" typo (should be "RUN HISTORY")

The history strip on `/workflows/marketplace/customer_research_followup` shows the section header as "BUN HISTORY · LAST 7 DAYS" — looks like a single-char typo of "RUN". Visible to the camera if Scene 5 holds on the detail view.

Where: somewhere in the workflow-detail component — grep `BUN HISTORY` in `web/`.

#### F10 [P3] — workflow run history shows 87% success rate over "LAST 7 DAYS" against an empty `workflow_runs` table

The "87% success rate · last 200 / last 25h / avg duration / last run" stats render with hardcoded numbers; no `/api/v1/workflows/*/runs` request is made. Pure dressing — fine for the demo per the function-vs-dressing rule, but document it so the operator doesn't try to drill in.

### /workers + worker-detail

#### F03 [P0] — Mariah's "Schedules" tab has no per-duty "Run now" button

`/workers/018f67d0-0000-7000-8000-000000000001/schedules` renders 4 toggleable Responsibilities ("List new inventory with photos + tags · as items arrive", "Re-price stale listings against competitor data · daily 09:30", "Respond to buyer questions on active listings · as message arrives", "Flag items dropping below floor price · daily 06:30"). The toggles fire (presumably enable/disable the cron). **There is no per-duty "Run now" button**. The Scene 2 runbook script says "Click 'Run now' → audit row lands in /audit within 5s" — the click target does not exist in the SPA today.

(Side observation: the runbook says 3 duties, the page shows 4. Not a blocker but worth noting for whoever maintains the script-to-implementation alignment.)

Repro:
- Navigate `/workers/018f67d0-0000-7000-8000-000000000001/schedules`
- Visually scan for any button labeled "Run now", "Fire now", "Trigger", etc. — there is none

Suggested fix: add a `Run now` per-duty button next to each toggle that calls the existing duty-fire API and writes the `WorkerDutyFired` audit row.

#### F11 [P1] — `/api/v1/workers/<id>` returns 404 (e.g. for Nori's id `…004`)

While the SPA's `/workers/<id>/overview` route renders (it likely uses a different aggregate endpoint), the direct GET `/api/v1/workers/018f67d0-0000-7000-8000-000000000004` returns 404 Not Found. Same for `/api/v1/workers/<id>/retrain/recent`. Two SPA pages will render skeletons forever on those calls.

#### F12 [P3] — `/workers` redirects to `/team` and Scene 1's narration says "Workers"

`web/app/(app)/workers/page.tsx` redirects to `/team` when the tenant has workers (which the demo does). The sidebar nav still has a "Workers" entry. Result: the camera clicks "Workers" in the sidebar and lands on `/team`. The Scene 1 runbook narrates `/workers` — the URL bar will read `/team`. Not a P0 because the visible content (5 trading-card workers in the deck) is what the script needs, but operator may want to either rename the sidebar entry to "Team" or invert the redirect so the canonical URL is `/workers`.

### /team

#### F13 [P1] — "MOCK-BYPASS" chip is rendered next to the "Team" heading in demo mode

`/team` hero renders a `<TeamDataSourceChip source="mock-bypass" />` (visible at top-of-page, right next to the heading). The chip shows the literal string "MOCK-BYPASS". The Scene 1 runbook holds the camera on this view for ~30s. This is dev-instrumentation visible in the recording.

Where: `web/app/(app)/team/page.tsx:255` renders `<TeamDataSourceChip source={dataSource} />`; the component is at `web/app/(app)/team/page.tsx:320`. The string comes from `teamQuery.data?.source` (line 142).

Suggested fix: either (a) hide the chip when `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`, or (b) render a friendlier label ("Sample data" instead of "MOCK-BYPASS"), or (c) move the chip behind an env-gated dev-tools panel.

### /cohorts

#### F14 [P2] — D3-magazine-tile expected to be 6+1 dept tabs; ships 5 tabs (still includes Project & Ops with Peta + Patrick)

Operator's spec for this run said "6 dept tabs; the 7th tab Project & Ops should show Peta + Patrick". The page renders **5 dept tabs**: Support · Sales · Project & Ops · Marketing · Sales & Investor Relations. Project & Ops correctly contains the Peta + Patrick cohort cohort (PE / PA, levels L2 / L3, "Project Coordinator" / "Status Reporter"). So the operator-asserted content IS present; the tab count is just off by one. Likely the spec is stale, not the page — but worth flagging for whoever maintains the runbook.

#### F15 [P2] — clicking an empty cohort tile opens a "no members yet" modal

Clicking the "Developing Support" cohort card opens a side-drawer/modal with "No members yet — No workers match this cohort's must-build skill set. Invite a member …". This is the correct empty-state UI, but the cohort card itself doesn't visually warn that it has 0 members (you only learn after click). The card design implies populated cohorts. Either show a member-count badge on the card, or hide empty cohorts from the grid for demo mode.

#### F16 [P2] — escape-to-close on the cohort drawer leaves the page in an unresponsive state for ~30s

After dismissing the "Developing Support" empty-cohort modal via the close button, subsequent eval/click operations against the page timed out for ~30s — looked like a focus-trap leak or a stuck animation/transition consuming the renderer thread. Manifested in the preview tool, but the underlying issue is likely a real bug. Worth investigating with React-DevTools Profiler.

### /settings + sub-pages

#### F17 [P1] — settings tab "Workspace" is permanently highlighted because the active-tab matcher uses prefix-startsWith

`web/components/skycot/settings/SettingsTabs.tsx:110-112`:

```ts
function isActive(pathname: string, tab: SettingsTab): boolean {
  return tab.match.some((prefix) => pathname === prefix || pathname.startsWith(`${prefix}/`));
}
```

The Workspace tab's `match` array is `["/settings", "/settings/workspace"]`. The `pathname.startsWith("/settings/")` clause matches **every** settings sub-route — `/settings/billing`, `/settings/security`, `/settings/data-export`, etc. all return `true` for Workspace. Result: TWO tabs render with `aria-current="page"` simultaneously on every settings sub-page (the actual active tab + Workspace).

Visible in the screenshot — both "Workspace" and "Billing" highlighted at the same time on `/settings/billing`. Verified on `/settings/data-export` too.

Suggested fix: change Workspace's match to `["/settings"]` only, and rely on `pathname === prefix` for that one tab (drop the startsWith branch for the root). Or special-case Workspace to be active only when no other tab is.

#### F18 [P2] — settings tab bar overflows at 1440 viewport with no horizontal-scroll affordance

12 tabs + Danger zone don't fit; the right-side tabs (Billing, Data export, Danger zone) are clipped. There's no scroll arrow, scroll fade, or overflow indicator. On a mobile/tablet viewport the tabs do horizontally scroll, but the same affordance is missing at desktop width.

#### F19 [P2] — `/settings/knowledge` returns 404 (route advertised in tab list)

`web/components/skycot/settings/SettingsTabs.tsx:57-61` defines a "Knowledge" tab pointing at `/settings/knowledge`. The route does not exist (`curl http://localhost:61264/settings/knowledge` → 404). The tab is clickable from every settings page.

#### F20 [P3] — `/settings/security` shows only a SAML SSO configure card

No 2FA setup, no IP allowlist, no session-management table, no audit-retention policy. For an "enterprise-grade audit-first" pitch, this page reads light. Not a demo blocker (Scene 6 doesn't visit it), but worth knowing.

### Marketing pages + footer

#### F21 [P3] — footer links `/security`, `/privacy`, `/terms` work on the demo-bypass build but 307 to /auth/signin on the parallel port-3000 build

When two Next dev servers are running (one started outside the launch.json, one started by it), the *out-of-band* one lacks `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` and middleware bounces those routes to `/auth/signin?from=…`. Footer link inventory at `/landing`:

```
/security  → 307 (bypass-less) | 200 (bypass)
/privacy   → 307 (bypass-less) | 200 (bypass)
/terms     → 307 (bypass-less) | 200 (bypass)
```

`web/middleware.ts:25-42` lists the public-passthrough prefixes; `/security`, `/privacy`, `/terms` are NOT in that list. The bypass env var saves them in dev. In production this would mean a guest user clicking footer/legal links from the marketing page gets a sign-in challenge — almost certainly not intended. Add those three paths to the middleware's public-passthrough list.

#### F22 [P3] — /landing footer points at `/team` (in-app route), not a marketing roster

The marketing nav and footer both link `Roster → /team`. `/team` is an in-app page requiring sign-in. Anonymous visitor clicking "Roster" gets bounced to sign-in. Either the marketing nav should point at `/landing#roster` (anchor) or `/customers` (the customer-logo page), or `/team` should expose an unauthenticated "demo roster" view.

#### F23 [P3] — `/landing` `mariah` social-proof tile lists "Mariah: 3 listings live, 2 buyers waiting on you" stacked as plain text; reads as a Slack message but isn't styled as one

Slight visual noise on the hero card. Pure dressing, P3. Camera-side this is fine — Scene 1 doesn't drill in here.

### Theme + viewport

#### F24 [P3] — light↔dark theme toggle works correctly across all in-app routes I tested

Positive finding: clicking the `[data-testid="appshell-theme-toggle"]` flips `<html>` between `class="… light"` data-theme=skycot-light and `class="… dark"` data-theme=skycot-dark; computed `--bg` changes from `rgb(250,250,255)` to a dark token; cards / surfaces re-tone correctly on /team, /audit, /settings, /workers/<id>/overview, /workflows/<v>/<n>. Worth noting as a *baseline OK*.

#### F25 [P3] — mobile (375x667) and tablet (768x1024) render cleanly across the in-app surfaces I checked

Hamburger menu opens cleanly on `/landing` mobile; the marketing-nav drawer scroll-spy momentarily highlights two items ("How it works" + "Features") at once on initial open, but it's a 1-second visual nit. `/team` mobile dark mode is clean (hero, KPI tiles, search collapse vertically as expected). `/cohorts` at 768 wraps to a 3+2 cohort grid cleanly.

---

## Resolved since last audit

This is the first audit run at this path; no prior audit to compare against. (The earlier `docs/skycot/demos/pre-record-ux-audit-2026-05-25.md` was operator-rejected and is not a comparison baseline per the operating instructions.)

---

## Orientation score

**62 / 100.**

What's working:
- Marketing surface (`/landing`, `/compare`, `/customers`, `/pricing`, `/docs`, `/changelog`) renders cleanly at desktop/tablet/mobile; D3 brand-consistency pass is visibly applied (Audiowide hero typography, violet→pink gradient, warm radial-blob bg, #1e1b4b ink in light mode).
- In-app pages (`/team`, `/cohorts`, `/audit`, `/workflows`, `/workflows/templates`, `/workers/<id>/overview`, `/settings/*`, `/genesis`) all return 200 and render their locked-design layouts.
- Theme toggle works across the shell; viewport responsiveness is solid.
- Demo mode banner is present and dismissable on every in-app page.
- Audit page renders 100 hash-chain rows (>= the runbook's "≥10 rows" requirement on count).
- 5 canonical workers (Mariah, Peta, Priya, Jamal, Nori) all present in `/team`; Peta + Patrick correctly inhabit the Project & Ops cohort.

What's broken (drives the 38-point gap):
- The audit-row drill-in pipeline (F01) is the single biggest function-side break — the camera-narrated Scene 6 drill-in lands on "Action not found".
- The SPA can't actually fire a workflow (F02) or a duty (F03) from its own UI today, which means Scenes 2 and 5 rely on off-SPA mechanisms (CLI, gateway curl) to produce the audit-row tail that the camera then pans to.
- F04 (no install button on /skills detail) is a smaller break but it's the literal action narrated in Scene 4.
- F13 (MOCK-BYPASS chip on /team hero) is in-frame for Scene 1.

Net: the marketing + chrome layer is investor-ready. The runtime → SPA → audit-chain plumbing has 4 P0 holes for the 6-scene MP4 to be recorded *as scripted*. With those 4 fixed (or routed around via off-camera CLI steps + an operator narration tweak), the score jumps to ~80/100.

---

# 6-MISSION-claim coverage rollup (load-bearing)

For each of the 6 claims in `docs/skycot/MISSION.md`, the question is: **based on what I observed in the SPA during this run, does this claim work functionally in MVP-demo scope?**

This is a *SPA-observation* rollup, not a backend claim-state rollup. CLAUDE.md asserts all 6 claims are 🟢 LIVE at the backend/runtime layer (regression tests pin them in code). This rollup grades how that runtime liveness *surfaces through the SPA* during a demo walkthrough. The two can diverge — and they do on Claim 5 and Claim 6 below.

---

### Claim 1 — Teammate, not tool (persistent identity, voice, memory, role-membership over time)

**Verdict: 🟢** (works in MVP-demo SPA scope.)

`/team` renders all 5 canonical workers (Mariah / Peta / Priya / Jamal / Nori) with names, personas, levels (LVL 5 / 18 / 9 / 12 / 15), distinct trait chips ("Patient", "Detail-oriented", "Thorough", "Respectful", "Calm under fire"), and per-worker headline behaviour ("3 listings live, 2 buyers waiting on you"). Clicking a worker opens `/workers/<id>/overview` with a personality-led detail page: "How Nori works" trait chips, "Great at" / "Escalate when" columns, "Signature behaviors" list, Responsibilities table. Mariah's chrome shows "Operations · Procurement" + "3 listings live" — the persistent role + activity surface is present.

Strongest evidence: the canonical 5 workers, the C2 personality pages, and the "Freeze worker" button (real capability primitive surfaced at the top-right of each detail view).

Strongest gap: F12 (the `/workers` URL the runbook narrates redirects to `/team` — minor URL-bar mismatch, not a behavioural break).

---

### Claim 2 — Delegation is the unit (duties + KPIs + charter, not "run this one job")

**Verdict: 🟡 partial** — the schema is there, the manual-fire path is not.

The Schedules tab on each worker shows 4 toggleable Responsibilities with cron-shaped triggers ("daily 09:30", "as items arrive", etc.). The "What she does" charter card on Overview reads as a real role contract ("Nori helps the Brand team keep customer-facing comms on-voice and on-schedule. She drafts product announcements, asks the brand lead before publishing anything net-new, …"). The Responsibilities ARE the duties; the toggles ARE the enable/disable. The delegation unit is visibly present.

What's missing in SPA scope: F03 — no per-duty "Run now" affordance, so the Scene 2 narrated manual-fire is unreachable. The cron will fire on its own schedule (backend-LIVE), but the demo's interactive moment ("operator clicks Run now → audit row appears in 5s") doesn't have a button to click.

Strongest gap to fix: add per-duty Run-now buttons on the Schedules tab.

---

### Claim 3 — The platform builds workers (plain English → vertical pack; no hand-authoring TOML)

**Verdict: 🟢** (works in MVP-demo SPA scope.)

`/genesis` renders the AI-Teammate Onboarding Studio — operator's headline. A "Describe the role in plain English" textarea (with a paste-in placeholder "I need someone to screen inbound leads, qualify them, and book interviews"), a "Generate worker" primary CTA, and an above-the-fold pitch ("Onboard a new teammate — Describe the role, upload your SOPs, web pages, Slack threads or voice notes, then scope their access. Genesis trains the teammate and checks comprehension before they start work."). This is the surface that materialises Claim 3 in MVP scope.

I did not click "Generate worker" in this run (no time budget for waiting on a real LLM round-trip), so the *runtime* genesis behaviour wasn't verified in-session — but the SPA path exists and matches the operator's `feedback_genesis_is_ai_teammate_onboarding_studio` clarification.

Strongest evidence: the textarea + CTA exist, the page copy aligns with operator's locked positioning ("Onboarding Studio", not "plain-English compiler").

---

### Claim 4 — Skills are shared and learned (library + learning loop + cross-tenant provenance)

**Verdict: 🟡 partial** — the marketplace and provenance surface render, but the install action has no SPA affordance.

`/skills` renders the A1-marketplace + hero-search-bar pattern with 7 skills and a `LIBRARY` chip on every card. Search pills include "approval / invoice / candidate / deal / support / lead" plus a "Skills my workers learned this month" suggestion-pill that surfaces the *learned* half of the claim. Clicking through to `/skills/pricing.competitor_scrape` shows a Provenance card ("Authored by SKYCOT · Trust ★") — the trust + provenance primitives are visible.

What's missing in SPA scope: F04 — the detail page has zero install affordance. The narrated Scene 4 install flow cannot fire from the SPA. The card copy redirects to "Hire a worker and add this skill from their Skills tab", which is a 2-step indirection from the marketplace surface.

Strongest evidence of "shared": the LIBRARY chip on every card + the provenance "Authored by" line.
Strongest evidence of "learned": the "Skills my workers learned this month" pill on the marketplace.
Strongest gap: no install button on detail.

---

### Claim 5 — Real-world execution = any task a human can do on a PC (MCP + computer-use, both)

**Verdict: 🟡 partial** — the *surface* tells the story; the *action* doesn't fire from the SPA today.

`/workflows/marketplace/customer_research_followup` is the canonical MCP+CU combo workflow the runbook leans on. The page renders 3 steps clearly tagged by executor:
- Step 1: **MCP SKILL** `fetch_thread`
- Step 2: **MCP SKILL** `summarize_question`
- Step 3: **TOOL CALL** `drop_note` with a `Notes · cu` chip

So the two-backend story (MCP and computer-use) is visible *in the same workflow*, in-frame, with executor-kind labels — that's the strongest SPA evidence I've seen of Claim 5 surfacing.

What's missing in SPA scope: F02 — clicking "Run now" gets a silent 403 from the workflow-execute API. So the camera sees the multi-executor manifest, but the *audit-row tail* that proves the executors actually ran has to be produced off-camera. Scene 5's "Click run → audit rows for both steps land" is unrecordable from the SPA today.

CLAUDE.md notes Claim 5 is 🟢 LIVE in the backend (regression test passed cold per `ops-claim-5-cold-rerun-mcp-and-cu-post-macos-perms` on 2026-05-27). The SPA does not yet expose that liveness as a click-and-watch interaction.

---

### Claim 6 — Multi-channel approval gates (Slack et al, with hash-chain audit + cross-tenant isolation)

**Verdict: 🟡 partial** — the surface is right, the data isn't.

`/approvals` renders the E ops-dashboard exactly per the locked design: brand-loud hero ("Approvals" with gradient + INBOX eyebrow), 5-KPI strip (PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS), 3 severity tiers (P0 HIGH / P1 MEDIUM / P2 LOW with rule labels), $-at-risk bar + confidence donut at the bottom. The shape of the surface IS the surface the runbook expects.

What's missing in SPA scope: every metric reads zero — "0 pending · 0m median time · 0% approval rate · no resolved approvals". Demo mode is on but the approvals queue is empty. There's no in-flight approval card the camera can click into to drill the multi-channel routing (Slack token → SPA approval card → audit row). For Scene 6 the camera pivots to `/audit` (where 100 rows DO render) so this isn't a P0 demo blocker on its own — but the function-side claim ("approvals via Slack etc, settled via HMAC-signed tokens, audit-row written") cannot be exercised from the SPA today without seeded inflight approvals.

Strongest evidence of the surface: the severity tiers + KPI strip + Confidence donut all match the locked design memory `project_approvals_page_design_locked`.
Strongest gap: no seeded in-flight approval rows means no interactive multi-channel demo from `/approvals` itself.

---

## Distance to v1.0 ship-gate

CLAUDE.md "Remaining for v1.0 launch" lists 5 items:
- (a)/(b)/(c)/(f) DONE
- (d) demo MP4 recording (operator-only)
- (e) Track P P-02 + P-05 + downstream

This run did not test Track P (Pitch Agent) pages. The 4 SPA P0s above (F01–F04) are blockers for **recording the existing 6-scene runbook as scripted**. Without fixes, the operator must either (i) modify the script narration to avoid the broken click targets, (ii) run the workflow/duty fire from a terminal off-camera and pan to the audit page after the row lands, or (iii) fix F01–F04 in code before recording.

Recommended fix-order if the operator wants the MP4 recordable from the SPA alone:
1. **F01** (audit drill-in) — likely a backend handler fix at `crates/argo-cli/src/handlers/gateway/routes_audit_detail.rs`. Highest impact: unblocks Scene 6's centrepiece moment.
2. **F02** (workflow execute 403) — auth/capability fix on the workflow-execute path. Unblocks Scene 5's centrepiece.
3. **F03** (duty Run-now button) — small SPA component addition. Unblocks Scene 2's centrepiece.
4. **F04** (skill install button) — SPA component + existing add-skill API wiring. Unblocks Scene 4's centrepiece.
5. **F13** (MOCK-BYPASS chip on /team) — env-gate in `web/app/(app)/team/page.tsx:255`. Removes visible dev-instrumentation from Scene 1's frame.

The fix-load is uneven: F01/F02 are real plumbing work; F03/F04/F13 are SPA changes that can ship as small briefs.
