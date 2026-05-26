# Scene 4 + 5 UX audit — Claim 4 (Skills shared + learned) + Claim 5 (Any-PC execution)

**Date**: 2026-05-26 (Melbourne)
**Auditor**: e2e-ux-journey-test-agent (read-only)
**Scope**: `/skills`, `/skills/[skillId]`, `/workflows`, `/workflows/build`, `/workflows/<v>/<n>`, builder palette, run-overlay; mapped against `docs/skycot/demos/v1-mission-mvp-runbook.md` Scenes 4 + 5.
**Backends observed**: gateway live on :7400, returning `skills: []` and `verticals: []` → SPA falling through to seed (`SEED_SKILLS`, `SAMPLE_WORKFLOW_SUMMARIES`, `WORKFLOW_TEMPLATES`). Demo banner visible: *"Demo mode — showing sample data."*
**Reference**: function-vs-dressing rule per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md`.

---

## TL;DR

The skills marketplace and workflows catalog **render**, but neither surfaces the function the runbook narrates. Scene 4's pivotal beats (search "competitor pricing scrape" → 3 results → Install → progress strip → new skill on worker page with source-pack + author chip → `SkillInvoked` audit row) are unrecordable as scripted: the skill doesn't exist, there is no Install button anywhere, and the worker's Active-skill row carries trust + usage + learned-at but neither `source_pack` nor `author`. Scene 5's MCP+computer-use combo workflow is worse — `customer_research_followup` is not in any seed, there is no `mcp_call` or `computer_use` step kind in the builder palette or the WorkflowStepKind union, and the workflow-detail "Run now" button is hard-disabled while `source !== "gateway"` (which it is, because the gateway is empty). Three P0 blockers, six P1s, three P2s. Function-side scope is the problem; dressing is fine.

## Headline blockers

1. **P0 — F1**: Scene 4 search for "competitor pricing scrape" returns 0 results (chip-set is 6 generic skills, none about competitor pricing).
2. **P0 — F2**: No Install button or install flow exists anywhere in `/skills` or `/skills/[skillId]`.
3. **P0 — F3**: Scene 5 workflow `customer_research_followup` does not exist in `SAMPLE_WORKFLOW_SUMMARIES`, `WORKFLOW_SPECS`, or `WORKFLOW_TEMPLATES`; nor does any seed workflow contain an MCP step + a computer-use step in sequence.
4. **P0 — F4**: Workflow detail "Run now" button is disabled whenever `source !== "gateway"` — and the live gateway returns empty for `/api/v1/verticals`, so it's always disabled in the recording environment.
5. **P0 — F5**: Builder palette has no `mcp_call` or `computer_use` step kind; the closest is `tool_call` → "Browser action — Drive a real browser." Scene 5's narration ("the dispatcher receiving `(tenant_id, \"zendesk\", \"search_tickets\")`") has no UI surface that names MCP or computer-use.

---

## All findings, in order

### Scene 4 — Skills marketplace + install + worker provenance

#### F1 [P0] — "competitor pricing scrape" returns 0 results, breaking the Scene 4 search beat

The runbook (Scene 4 step 2): *"Operator searches **competitor pricing scrape**. Three results."* `SEED_SKILLS` in `web/lib/api/skills.ts:55-116` contains exactly 6 skills: `approval.route`, `invoice.extract`, `candidate.screen`, `deal.qualify`, `support.triage`, `lead.enrich`. None matches the substring "competitor", "pricing", or "scrape". A live search will hit `web/app/(app)/skills/page.tsx:177-181` empty state ("No skills match …"). The TRY chips on the hero are also generic (`approval routing`, `invoice extraction`, etc.) — the operator can't even chip-tap their way into the demo phrase.

- Reproduction: open `/skills`, type "competitor pricing scrape" into the search box → empty state shows.
- File: `web/lib/api/skills.ts:55` (SEED_SKILLS) + `web/app/(app)/skills/page.tsx:23-30` (TRY_CHIPS).
- Suggested fix: either add 3 seed skills matching the runbook phrase ("competitor.pricing.scrape", "refund.policy.lookup", "slack.thread.summary") OR rewrite the runbook to use one of the 6 existing seed phrases (e.g. "deal qualification"). Operator decision.

#### F2 [P0] — No Install button or install flow exists; Scene 4 step 3 cannot be performed

Scene 4 step 3: *"Click **Install** on the verified result. Progress strip: \"Verifying provenance… Installing skill… Available to workers.\""* The skill-detail page (`web/app/(app)/skills/[skillId]/page.tsx`) renders Provenance (author + trust grade) and Workers-using sections, plus a "← Back to the marketplace" link. There is no Install button, no progress strip, no install-related state machine. `grep -ri install` across `web/app/(app)/skills/`, `web/components/skycot/skills/`, `web/lib/api/skills.ts`, `web/lib/hooks/use-skills.ts` returns one hit — the literal string "No skills installed yet" in an empty-state copy.

- Reproduction: open `/skills/approval.route` (or any seed skill) → no Install affordance visible; no install mutation hook in `useSkills()`.
- File: `web/app/(app)/skills/[skillId]/page.tsx:38-122` (no install UI) + `web/lib/api/skills.ts` (no installSkill/POST helper).
- Suggested fix: ship a brief that adds `installSkill()` + the install-progress overlay (verifying provenance → installing → available). This is function-side per the function-vs-dressing rule (Scene 4 narrates Install + progress strip + worker-page propagation), so the install must trigger a real backend round-trip even if the demo seed is the source pack.

#### F3 [P1] — Worker Active-skill row lacks source-pack and author chip

Scene 4 step 4: *"The new skill appears under \"Skills available to Maya\" with the source pack + version + author chip."* `ActiveSkill` in `web/lib/api/worker-skills.ts:34-48` has: `id`, `name`, `version`, `trustScore`, `usageCount7d`, `learnedAt?`. No `source_pack`. No `author`. The render in `web/components/skycot/worker/SkillsActiveCard.tsx:58-112` shows version + TrustBadge + usage count + learned-ago chip. The source-pack and author chip the runbook narrates are not present.

- Reproduction: open `/workers/<id>/skills` (any worker) → only version + trust + usage shown, no provenance chips.
- File: `web/lib/api/worker-skills.ts:34` (type) + `web/components/skycot/worker/SkillsActiveCard.tsx:64-82` (meta block).
- Suggested fix: add `sourcePack` + `authorLabel` to `ActiveSkill`, render alongside the existing meta pills. The detail page (`/skills/[skillId]`) already has `usage.author_label` from `fetchSkillUsage`; reuse the same field.

#### F4 [P1] — Skills page hero count pill always reads "6 skills · 6 shown" because the gateway is empty

Gateway `/api/v1/skills` returns `{skills: []}` so the SPA falls to the 6-skill seed. The hero count pill (`web/app/(app)/skills/page.tsx:113-117`) reads `{skills.length} skills · {filtered.length} shown` → always "6 skills · 6 shown" while in mock-offline. The runbook copy implies a deeper catalog ("dozens more"). On camera, "6 skills" undercuts the marketplace feel.

- Reproduction: load `/skills` against current gateway → count pill shows 6 / 6.
- File: `web/lib/api/skills.ts:55` (SEED_SKILLS length).
- Suggested fix: expand `SEED_SKILLS` to ~20-30 plausible skills before the demo, or seed the live gateway with the runbook's expected set. Dressing-side if the camera doesn't zoom on the pill, but the pill is right next to the hero so a wide shot will read it.

#### F5 [P1] — TRY chips don't include the runbook phrase; operator must type from scratch

Scene 4 narration implies the search beat is fast (chip-tap or short type). The chip set is `[approval routing, invoice extraction, candidate screening, deal qualification, support triage, lead enrichment]` plus the "Skills my workers learned this month" filter. If F1 is fixed by adding the seed skills, the matching TRY chips should also land — otherwise the operator types `competitor pricing scrape` letter by letter on camera.

- Reproduction: load `/skills` → no chip matches the runbook phrase.
- File: `web/app/(app)/skills/page.tsx:23-30`.
- Suggested fix: align `TRY_CHIPS` with whichever seed-skill set lands for F1.

#### F6 [P2] — Skill detail page has no "Workers using this skill" data in mock-offline

`fetchSkillUsage` returns `seedUsage(name)` in mock-offline, which is `{ workers_using: [], workers_using_count: 0, adoption_count: 0 }`. The card renders "No workers are using this skill yet" + "Used by **0** workers across the SKYCOT marketplace." Scene 4 doesn't drill into a skill detail page (it goes marketplace → install → worker page), so this is dressing-adjacent — the camera might pass over it but doesn't read it. P2.

- File: `web/lib/api/skills.ts:187-197` (seedUsage).
- Suggested fix: seed a couple of mock workers in `seedUsage` so the card looks populated.

---

### Scene 5 — Workflows: MCP + computer-use combo workflow

#### F7 [P0] — `customer_research_followup` workflow does not exist in any seed

The brief assignment refers to *"the `customer_research_followup` MCP+computer-use combo workflow."* Searching `web/lib/sample-workflows.ts` lists 7 seed workflows: `post_listing`, `screen_candidate`, `approve_refund`, `qualify_lead`, `triage_ticket`, `morning_pipeline_digest`, `clean_customer_list`. Searching `web/lib/workflow-templates.ts` (50+ templates) finds the closest match `competitor_intel_digest` (sales-competitor-intel) but no `customer_research_followup`.

- Reproduction: `/workflows` → search "customer_research" → empty state.
- File: `web/lib/sample-workflows.ts:65-128` (summaries) + `web/lib/workflow-templates.ts` (templates).
- Suggested fix: author a `customer_research_followup` workflow in `sample-workflows.ts` with steps `[trigger, mcp_call(zendesk:search_tickets), computer_use(macos_notes:create_note), notify]` — but that requires F8 (step-kind support) to land first.

#### F8 [P0] — Builder + WorkflowStepKind has no `mcp_call` or `computer_use` step kind

`WorkflowStepKind` in `web/lib/sample-workflows.ts:18-27` is `trigger | skill_invoke | approval | tool_call | notify | decision | data | http_request | code`. The builder palette in `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:236-286` exposes the same kinds plus `loop | wait | sub_workflow`. No `mcp_call`. No `computer_use`. The closest UX hook is `tool-browser` → "Browser action — Drive a real browser" (kind: `tool_call`, icon: MousePointer2). Scene 5's narration ("MCP for the well-known APIs … computer-use for everything else … same capability gate, same audit row") has no first-class UI affordance on the canvas.

- Reproduction: open `/workflows/sales/__new__/build` → drag from sidebar → no MCP or computer-use option visible.
- File: `web/lib/sample-workflows.ts:18` + `web/app/(app)/workflows/[verticalId]/[name]/build/types.ts:22-34, 236-286`.
- Suggested fix: extend the union with `mcp_call` and `computer_use`, add palette entries under the **Tools** group (e.g. `tool-mcp` → "MCP tool — Call a connected MCP server", `tool-cu` → "Computer use — Drive the macOS desktop"), and update `STEP_KIND_META` in `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:59-72` so the viewer can render them. **Function-side** per the rule — Scene 5 visibly narrates which kind is firing.

#### F9 [P0] — Workflow detail "Run now" is hard-disabled when source !== "gateway"; gateway returns empty so it's always disabled

`web/app/(app)/workflows/[verticalId]/[name]/page.tsx:268-281` renders the run button with `disabled={running || !isLive}` and tooltip *"Available once this workflow is published to the gateway."* `isLive` is `source === "gateway"` (line 103). With `/api/v1/verticals` returning `{verticals: []}`, every workflow falls to seed-source → button disabled. The runbook Scene 5 step 2: *"Hit **Run**. Step 1 fires through the MCP pool …"* — that click cannot land in the current state. The build-canvas page has its own test-run path (`useTestRun`) but the **detail page** Run button is the one the runbook describes (it's the natural "Run" entry-point for an operator looking at the canonical workflow).

- Reproduction: `/workflows/sales/qualify_lead` → "Run now" button is greyed out.
- File: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:273-280` (disabled logic).
- Suggested fix: either seed the live gateway with the demo workflows (best path — also resolves F4) or relax the disabled gate to allow seed-source runs that emit a real audit row via a seeded handler. The operator-locked NO-THEATER rule means the run must actually fire the two backends, so seeding the gateway is the cleaner route.

#### F10 [P1] — Workflow card "by Rio" violates Scene-1 worker-roster canon

Per `feedback_no_theater_in_demo` + the v1-demo positioning rule "scene-vocabulary mismatch is a blocker": Scene 1 establishes the worker roster as **Mariah / Peta / Priya / Jamal / Nori**. Scene 5 lands on `/workflows`. The `qualify_lead` card in `web/lib/sample-workflows.ts:96-105` is attributed *"by Rio"* (lastEditor field, rendered on the card meta row in `web/app/(app)/workflows/page.tsx:262`). Rio is not in the canonical roster. The card is visible whenever the operator browses /workflows during Scene 5.

- Reproduction: curl `http://127.0.0.1:3000/workflows` → grep "Rio" → confirms.
- File: `web/lib/sample-workflows.ts:103` (`lastEditor: "Rio"`).
- Suggested fix: change `Rio` → one of `Mariah / Peta / Priya / Jamal / Nori`. One-line edit.

#### F11 [P1] — No visible computer-use indicator on `/workflows` or workflow detail (macOS-only, per project_v1_strategic_decisions_2026_05_14)

The runbook narrates *"right now you're watching the macOS executor drive the Notes app"* — but no card, viewer, or run-overlay UI in `/workflows`, `/workflows/<v>/<n>`, or the builder communicates "macOS computer-use" as a step type, dispatcher target, or visual badge. The closest existing piece is `web/lib/connectors/registry.ts:35` ("OAuth vs computer-use session capture flow") which is a connector-config comment, not a runtime badge. The audit-detail type `audit-detail.ts:50` includes `computer-use` as an output kind, so the audit table can colour-code it post-hoc, but during the live run there's no UI cue that says "this step is computer-use."

- File: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (no CU badge) + `web/app/(app)/workflows/[verticalId]/[name]/build/RunOverlay.tsx` (no CU step coloring).
- Suggested fix: when F8 lands, add a CU step-kind icon (laptop or mouse-pointer, same colour as MCP for parity), and surface it on the workflow card meta row + the run overlay.

#### F12 [P1] — Run overlay doesn't surface MCP dispatcher tuple `(tenant_id, server, tool)`

The runbook Scene 5 step 2: *"the activity feed shows the dispatcher receiving `(tenant_id, \"zendesk\", \"search_tickets\")`."* The builder's `RunOverlay.tsx` renders step state (running / ok / fail), ms, cost — but no MCP tuple, no tenant_id, no upstream tool name. There's nowhere on screen during a run that says "Zendesk search_tickets was called."

- File: `web/app/(app)/workflows/[verticalId]/[name]/build/RunOverlay.tsx:34-160`.
- Suggested fix: extend `StepRunState` with `mcpDispatch?: { server: string, tool: string, tenantId: string }` and render a short line under the step name when present.

#### F13 [P1] — Demo banner says "showing sample data. Sign in to see your own workspace" — the camera will catch it

`/skills` and `/workflows` both render the demo banner (visible in the snapshot as *"Demo mode — showing sample data. Sign in to see your own workspace."*). For a POC investor demo the banner reads as "this isn't real" — function-adjacent because Scene 4 + 5 explicitly demo skills + workflow runs. The banner's copy implies "what you see is fake."

- Reproduction: load `/skills` or `/workflows` with `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`.
- File: `web/lib/skycot/demo-mode.ts` + the AppShell DemoBanner consumer.
- Suggested fix: for the recording, set `NEXT_PUBLIC_SKYCOT_DEMO_MODE=0` and seed the live gateway with real demo data (one-time `argo demo seed --tenant skycot-demo` per the runbook precondition). The banner is dressing-side but its **copy** undermines the function-side narrative.

#### F14 [P2] — Workflow-detail "Open builder" + "Edit in chat" buttons distract from the Run beat

The detail page renders 5 buttons in a row: Open builder · Edit in chat · Version history · + Add as Schedule · Run now. The Run button (the only one Scene 5 needs) is fifth. During the camera pan to "Hit Run", the operator's eye gets pulled across four other CTAs first. P2 because the operator can still find it; the cluttered action row is just less cinematic.

- File: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:239-282`.
- Suggested fix: pre-record polish — move "Run now" leftmost or split into a primary + a secondary kebab menu.

---

### Cross-cutting

#### F15 [P1] — Audit feed has no `skill.invoked` or `workflow.step.executed` rows; Scene 4 + 5 pass-cues won't fire

`curl http://127.0.0.1:7400/api/v1/audit?limit=5` returns only `audit.replay` and `audit.detail.viewed` rows (from this audit's own browsing). Scene 4 pass-cue: *"a `SkillInvoked` audit row is written carrying the same provenance."* Scene 5 pass-cue: *"`/audit` shows two `workflow.step.executed` rows for the run, one per backend."* If F2 + F4 + F8 + F9 land but no audit row is emitted by the underlying handler, the camera-drill-in at end of Scenes 4 + 5 has nothing to show.

- File: gateway side, not web; flagged here because the web Scene-4/5 demo depends on it.
- Suggested fix: confirm `claim_4_live_install_invoke` + the workflow runtime emit the expected audit kinds and that they pass through the gateway's audit feed envelope. (Outside the web scope of this audit but the chain depends on it.)

#### F16 [P2] — Skill-detail "View source" button is a toast stub

`web/app/(app)/workers/[workerId]/skills/page.tsx:53-55`: `onViewSource` only fires an announcement toast. Scene 4 doesn't drill in on a skill from the worker page, so this is dressing-side P2 — but it's the kind of click an investor might naturally try after seeing "View source" next to a skill row, and a toast saying "Skill source viewer ships with the marketplace surface" reads as half-built.

- File: `web/app/(app)/workers/[workerId]/skills/page.tsx:53` + `web/components/skycot/worker/SkillsActiveCard.tsx:97-101`.
- Suggested fix: hide "View source" until the source viewer ships, OR navigate to `/skills/<name>` (the detail page) on click.

#### F17 [P2] — Run overlay markup uses `<style jsx global>` requiring `"use client"`; correct here, just calling it out per styled-jsx footgun

Per memory `feedback_styled_jsx_requires_use_client`, components with `<style jsx>` must declare `"use client"`. `RunOverlay.tsx:1` does. No bug; flagging because the file is fragile to future "convert to server component" passes. Skip the fix.

- File: `web/app/(app)/workflows/[verticalId]/[name]/build/RunOverlay.tsx:1, 53`.

---

## Resolved since last audit

`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md` exists from yesterday. A diff against that audit is out-of-scope for this run (the operator asked for a fresh walk), but quick spot-check: the skills page hero + search + grid + worker-skills 3-card layout (Active / Library / Learned) all still render — no obvious regression in the surfaces I touched today.

## Orientation score

**38 / 100.**

The visible surfaces (marketplace grid, workflow catalog, builder canvas) **look** like a real product — A1 marketplace register and brand consistency hold up well. But the **function** the runbook narrates is not wired. The five P0s are all "the thing the camera does next has no UI to land on." Per the function-vs-dressing rule, that's the demo-blocker category. Two of the five (F1, F10) are 5-minute seed-data fixes; the other three (F2 Install flow, F8 mcp_call/computer_use step kinds, F9 Run-button gating) are real engineering work and the long pole on getting Scenes 4 + 5 recordable.

The dressing-side surface is solid — F4, F6, F13 are real but won't kill the demo if the function-side is fixed first. The "ship 5 P0s, accept the P1s, batch the P2s" path puts these scenes recordable in roughly one workday-equivalent of focused brief work, assuming a backend runtime that actually emits `skill.invoked` + `workflow.step.executed` audit rows (F15, which this audit can't verify from the web).
