# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Audit run**: 2026-05-26 0820 Melbourne
**Scope**: `/skills` marketplace, skill install flow, `/workflows` index, workflow detail card, `/workflows/build` canvas, computer-use indicators
**Methodology**: code read + curl SSR probes against `http://localhost:3000` + gateway probes against `http://127.0.0.1:7400`. No browser MCP. Cross-checked against prior audit `docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md` for what's changed.
**Function-vs-dressing rule applied** per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

## TL;DR

Scene 4 + Scene 5 are still **unrecordable as scripted** — the same three structural P0s from yesterday's audit are unfixed. Counts: 4 P0, 4 P1, 3 P2. The three demo-killers: (1) the runbook tells the operator to search for **competitor pricing scrape** and click **Install** — there is no `competitor pricing scrape` skill in the seed catalog (the 6 hardcoded SEED_SKILLS are approval.route / invoice.extract / candidate.screen / deal.qualify / support.triage / lead.enrich) and there is no Install button anywhere on the skill-detail page (`Install` in the page HTML resolves to the global PWA InstallPrompt, not a skill action); (2) the canonical Scene 5 workflow `customer_research_followup` with a `zendesk:search_tickets` MCP step and a `macos_notes:create_note` computer-use step exists nowhere in the repo — `rg zendesk:search_tickets|macos_notes:create_note` returns 0 hits in `crates/` and `web/`, and the workflow builder palette `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` defines 12 step kinds with neither `mcp_invoke` nor `cu_action` as a first-class kind; (3) `/workflows` cards still surface non-canon worker names (Rio, Sam) alongside the canon Mariah/Priya/Jamal — Scene 5 narrates the card the camera lands on, so the viewer sees a workflow "by Rio" but Scene 1 introduced only Mariah / Peta / Priya / Jamal / Nori. One good improvement since yesterday: the visible eyebrow on `/workflows` now reads **WORKFLOWS · v0.1** (not "WORKFLOWS · MOCK"), so the camera no longer sees a "MOCK" label — the `mock-offline` value moved to a `data-source` attribute. Orientation score 36/100.

## Headline blockers

| # | Severity | One-line finding | Scene impact |
|---|---|---|---|
| F1 | P0 | No "competitor pricing scrape" skill — gateway returns 6 SEED_SKILLS, none match Scene 4's narrated search | Scene 4 search beat (4:00–5:20) breaks at second 0 |
| F2 | P0 | No Install button on `/skills/[skillId]` — only a global PWA prompt | Scene 4 "Click **Install** on the verified result" has no target |
| F3 | P0 | `customer_research_followup` workflow does not exist + `mcp_invoke` / `cu_action` are not first-class step kinds in the builder | Scene 5 MCP-then-CU combo (5:20–6:40) has nothing to run |
| F4 | P0 | Workflow cards show non-canon worker names Rio + Sam alongside the canon 5 (Mariah, Priya, Jamal) | Scene 5 author chip mismatches Scene 1 narration |

## All findings, in order

### F1 [P0] — `/skills` marketplace cannot produce 3 results for "competitor pricing scrape"

`web/lib/api/skills.ts:55-116` defines the 6-row SEED_SKILLS array used as fallback when the gateway returns 0 skills (the gateway currently has no rows installed: a `GET /api/v1/skills` probe to `127.0.0.1:7400` returns the principal-required envelope, and the SPA falls through to mock-offline). None of the six are competitor / pricing / scrape. The `/skills` page reads from `useSkills` which returns this seed. Scene 4 line 209 of the runbook says "Operator searches **competitor pricing scrape**. Three results." — the count-pill will render `6 skills · 0 shown`.
- Reproduce:
  - `curl -s http://localhost:3000/skills -o /tmp/skills.html`
  - `python3 -c "s=open('/tmp/skills.html').read(); print('competitor' in s)"` → `False`
  - Inspect `web/lib/api/skills.ts:55-116`.
- File path: `web/lib/api/skills.ts:55`
- Suggested fix: add three competitor-pricing seed skills (e.g. `competitor.pricing.scrape`, `competitor.pricing.compare`, `competitor.pricing.alert`) to SEED_SKILLS so the demo search beat resolves, OR rewrite the talk track to search for one of the existing 6.

### F2 [P0] — No skill-install affordance on the marketplace or skill-detail page

`/skills/[skillId]/page.tsx` renders only Provenance + "Workers using this skill" + adoption count. There is no Install / Add to worker / Try button. The Scene 4 talk track requires "Click **Install** on the verified result" + progress strip "Verifying provenance… Installing skill… Available to workers." Neither button nor progress strip exists. `grep -o "Install[^<]*" /tmp/skill-detail.html` returns only matches against the global `InstallPrompt.tsx` PWA component (visible in the layout but unrelated to skill installation). The only install-shaped flow is `/workers/<id>/skills/new` which opens a Ghostwriter form to *draft a new skill from a prompt* — different verb entirely.
- Reproduce:
  - `curl -s http://localhost:3000/skills/approval.route -o /tmp/d.html`
  - `grep -o 'data-testid="[^"]*"' /tmp/d.html | sort -u` → only skill-adoption-count / skill-provenance / skill-workers-using
  - Inspect `web/app/(app)/skills/[skillId]/page.tsx:38-122`
- File path: `web/app/(app)/skills/[skillId]/page.tsx:38`
- Suggested fix: add an Install button + progress strip + post-install navigation to the worker page; wire to a `POST /api/v1/skills/<name>/install` gateway endpoint that mirrors `SkillStore::install`. Until that lands, Scene 4 cannot be recorded.

### F3 [P0] — Scene 5's `customer_research_followup` workflow + MCP + computer-use step kinds do not exist

`rg "customer_research_followup|zendesk:search_tickets|macos_notes:create_note"` across `crates/` and `web/` returns 0 hits (only the BUILD_STATUS.md ledger entries about the BLOCKED brief mention them). `web/lib/sample-workflows.ts:64-138` seeds 7 workflows; none is `customer_research_followup`. The workflow-builder palette at `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34` defines 12 StepKind values (trigger / skill_invoke / approval / tool_call / notify / decision / data / loop / wait / sub_workflow / http_request / code) — no `mcp_invoke`, no `cu_action`. The "Browser action" entry in the sidebar palette (line 274) is generic `tool_call`, not a typed CU step. Scene 5 cannot be recorded against a workflow that doesn't exist, and even if a wrapper workflow were synthesized, the visual card would render the steps as generic Database / Tool icons rather than distinguishing MCP from computer-use.
- Reproduce:
  - `rg "customer_research_followup" web/ crates/` → 0 hits
  - `rg "kind.*\"mcp_invoke\"|kind.*\"cu_action\"" web/` → 0 hits
  - `cat web/app/(app)/workflows/[verticalId]/[name]/build/types.ts | sed -n '22,34p'`
- File paths: `web/lib/sample-workflows.ts:64`, `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22`
- Suggested fix: ship a `customer_research_followup` seed workflow that the camera can land on, AND add `mcp_invoke` + `cu_action` to StepKind + palette + STEP_KIND_META so the cards render distinguishable icons; OR rewrite Scene 5 to use `support/approve_refund` (which already has a `tool_call` to Stripe and a manager approval) as a stand-in and update the talk track to drop the "MCP and computer-use as first-class step kinds" framing.

### F4 [P0] — Workflow cards surface non-canon worker names (Rio, Sam)

`web/lib/sample-workflows.ts:64-138` assigns workflow `lastEditor` chips to Mariah, Priya, Jamal, **Rio**, **Sam**. The canonical 5-worker demo fleet introduced in Scene 1 + memory `project_pm_cohorts_peta_patrick` is Mariah / Peta / Priya / Jamal / Nori. Rio and Sam are not in any canon roster. Scene 5 narrates whichever card the operator lands on — if the camera shows `qualify_lead by Rio` or `morning_pipeline_digest by Rio` or `clean_customer_list by Sam`, the viewer sees a worker name the talk track has never named. This is exactly the scenario the audit-scope brief calls out as a P0 scene-vocabulary mismatch.
- Reproduce:
  - `curl -s http://localhost:3000/workflows -o /tmp/wf.html`
  - `python3 -c "s=open('/tmp/wf.html').read(); print([n for n in ['Rio','Sam','Peta','Nori'] if n in s])"` → `['Rio','Sam']` (Peta + Nori absent)
- File path: `web/lib/sample-workflows.ts:104,124,136`
- Suggested fix: replace Rio → Mariah and Sam → Nori across SAMPLE_WORKFLOW_SUMMARIES so every card shows a canon worker name. Cheap edit, no schema change.

### F5 [P1] — No "WORKFLOWS · MOCK" visible (improvement over yesterday — confirm holds)

Yesterday's audit flagged the `/workflows` eyebrow rendering "WORKFLOWS · MOCK" on screen. As of this run the eyebrow text is `WORKFLOWS · v0.1` (`web/app/(app)/workflows/page.tsx:67`) and the mock-source value moved to a `data-source` attribute on the same element — not visible to the camera. This is a P1 (not P0) reminder rather than a finding: if anyone re-edits the page and uses the source pill text again, the MOCK label re-appears. Worth a CSS-content guard or a test that asserts the rendered eyebrow text is the literal `WORKFLOWS · v0.1`.
- Reproduce:
  - `curl -s http://localhost:3000/workflows -o /tmp/wf.html`
  - `python3 -c "import re; print(re.findall(r'WORKFLOWS[^<]*', open('/tmp/wf.html').read())[:3])"` → `['WORKFLOWS · v0.1', 'WORKFLOWS']`
- File path: `web/app/(app)/workflows/page.tsx:60-68`
- Suggested fix: leave alone; add a Playwright assertion that the visible eyebrow exactly equals `WORKFLOWS · v0.1` to prevent silent regression.

### F6 [P1] — No visible computer-use or macOS substrate indicator anywhere in the SPA

`rg "cu_macos|computer-use-macos|MacosCu|substrate" web/` returns 0 hits outside node_modules / .next. The workflow detail page (`web/app/(app)/workflows/[verticalId]/[name]/page.tsx`) has no substrate chip, no macOS-executor pill, no "running on your Mac" indicator. Scene 5 talk track says "Right now you're watching the macOS executor drive the Notes app" — there is no on-screen reinforcement that the executor is the macOS substrate, only the visible Notes app behaviour. For an investor demo this is a P1 (Notes opening on screen is the load-bearing visual) but for a public release demo it would be P0.
- Reproduce: `rg -l "cu_macos|computer-use-macos|substrate" web/ 2>/dev/null | grep -v node_modules | grep -v .next` → empty.
- File path: n/a (no surface to point at)
- Suggested fix: add a small "macOS executor · live" pill on the workflow-run header when the workflow contains a CU step; reuses the same chip pattern as the workflows source pill.

### F7 [P1] — No "Skills my workers learned this month" data visible without gateway

The chip exists (`web/app/(app)/skills/page.tsx:133-143`) and toggles a filter against `learnedQuery.data?.learned.skill_ids`. The hook falls back to an empty array when the gateway is unreachable (`web/lib/api/skills.ts:232-252`). With gateway-up-but-empty in dev, the chip toggles, the filter applies against 0 ids, and the grid empties to "No skills installed yet" or "No skills match …". Scene 4 narrates the learning loop ("the new ones learned by one teammate become available to the rest") — pressing the filter chip shows nothing, undermining the talk track.
- Reproduce: click the "Skills my workers learned this month" chip on `/skills`; observe empty grid.
- File path: `web/lib/api/skills.ts:232-252`
- Suggested fix: when source is `mock-offline`, mark one or two seed skills as "learned this month" (e.g. flip `provenance_source: 'learned'` on `support.triage` and `lead.enrich`) and include their ids in the `learned-this-month` fallback. Pure-fixture change.

### F8 [P1] — Workflow detail page hits `/workflow-runs` even with mock data (404 noise)

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:108-132` always fires `fetchWorkflowRuns` regardless of `source === "mock-offline"`. Against a dev gateway with no rows this is a guaranteed 404 per workflow detail page open. Not visible to the camera but pollutes dev-server logs and adds 200-400ms latency to the page load.
- Reproduce: `curl -s http://localhost:3000/workflows/sales/qualify_lead` then check dev-server console for `/api/v1/workflow-runs?...` 404.
- File path: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:108`
- Suggested fix: skip `fetchWorkflowRuns` when the workflow source is mock-offline, or have the hook surface a graceful mock-runs envelope.

### F9 [P2] — `/skills/[skillId]` hero says "your org" but seed-fallback returns generic "Authored by SKYCOT"

`web/lib/api/skills.ts:187-197` `seedUsage` returns `author_label: "Authored by SKYCOT"` for every skill when the gateway is unreachable. The skill-detail page's fallback chip on line 66-67 prefers `usage?.author_label` and only falls back to "Authored by your org" when there's no usage data at all. With mock-offline, every skill detail page shows "Authored by SKYCOT" — fine for the marketplace skills but inconsistent with the "learning loop" narrative that some skills are first-party (org) and some are marketplace.
- Reproduce: `curl -s http://localhost:3000/skills/approval.route | grep "Authored by"` → "Authored by SKYCOT" on all.
- File path: `web/lib/api/skills.ts:187-197`
- Suggested fix: alternate author labels across the seed (3 "Authored by SKYCOT" verified, 2 "Authored by your org", 1 "Learned by Mariah") to surface the provenance range visibly.

### F10 [P2] — Builder canvas blank-vertical entry route uses `__new__` token (visible in URL)

`/workflows/build` → click "Sales" navigates to `/workflows/sales/__new__/build`. The `__new__` segment is visible in the address bar. The camera does not record the URL bar by default for in-app scenes, but if any frame catches it the `__new__` reads as engineering plumbing.
- File path: `web/app/(app)/workflows/build/page.tsx:44`
- Suggested fix: route to `/workflows/build/<vertical>` or use a short query param so the URL reads cleanly.

### F11 [P2] — Workflow-builder "Browser action" palette entry mislabelled given Scene 5 narration

`web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:274` exposes `{ id: "tool-browser", kind: "tool_call", label: "Browser action", description: "Drive a real browser" }`. Scene 5's narrated truth is that macOS computer-use drives the **Notes app** (not the browser). The single palette entry conflates browser automation with desktop control. Not a P0 because Scene 5 does not navigate into the builder during recording, but if the camera ever follows an "Edit this workflow" link the mismatch is visible.
- File path: `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:271-276`
- Suggested fix: split into two entries — `Browser action` (Playwright/headless) and `Desktop action` (macOS / Windows / Linux substrate); back them with distinct step kinds (`cu_browser`, `cu_desktop`) or at minimum distinct palette items off the same `tool_call` kind.

## Resolved since prior audit (2026-05-25 1100)

| # | Resolution |
|---|---|
| F1 (prior) | `/workflows` eyebrow no longer renders "WORKFLOWS · MOCK" — now renders `WORKFLOWS · v0.1`; source moved to data-attribute (`web/app/(app)/workflows/page.tsx:67`). See current F5. |
| Other prior P0s | Unchanged. F1/F2/F3/F4 in this report mirror prior F2/F4/F3/F1. |

## Orientation score — 36 / 100

Most of the Scene 4 + Scene 5 user-visible chain is structurally absent: no install button, no competitor-pricing skill, no MCP / CU step kinds, no Scene-5 workflow seed. The good parts: skills marketplace hero layout matches the locked A1 marketplace design (gradient icon + 18px input + ⌘K + count pill + TRY chips), workflows page eyebrow no longer shouts MOCK, the workflow builder is a real react-flow canvas with save/publish + test-run wiring (not a static mock), and the workflow detail page does render a vertical-card matrix with author + version + last-edited chips (the canonical author-chip pattern is implemented — just populated with non-canon names).
