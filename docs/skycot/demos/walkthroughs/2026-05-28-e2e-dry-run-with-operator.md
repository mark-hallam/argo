# SKYCOT v1.0 demo — e2e walkthrough notes

**Date**: 2026-05-28T11:30 AEST (Melbourne)
**Driver**: Claude (conductor) via claude-in-chrome MCP
**Observer**: Mark Hallam (operator)
**Mode**: Live SPA on `:3000` against live gateway on `:7400` (schema v84, build `9e19589`)
**Tenant**: `019e5057-1d3f-7fa2-9c1b-c34ef01cc21e` (workspace tenant for `~/code/argo`, "argo")
**Roster**: 9 workers — Mariah · Peta · Priya · Jamal · Nori (canonical 5) + Lex · Devon · Fin · Pitch (dept-fillers)
**Pre-warm**: audit drill-in 1.85s cold → 0.14s warm; all onboarding routes <1s; workflows/build 1.5s; skills 1.75s

**Severity legend**
- 🔴 **P0** — demo-blocker, MP4 cannot record until fixed
- 🟡 **P1** — record-degrader, MP4 records but with visible compromise
- 🟢 **P2** — polish, ship the MP4 first, fix after

---

## Scene 1 — Teammate, not tool (Claim 1)

**Mission claim**: "An AI hire shows up as a colleague, not a chatbot."
**Regression pin**: `claim_1_teammate_not_tool` (argo-audit / claim-1 LIVE)
**Route flow**: `/` (marketing) → `/auth/signup` → `/onboarding/welcome` → `/workers`
**What the camera should see**: marketing hero positions AI as a teammate; signup is magic-link not password; landing in the app shows a roster of named people with departments, not a list of features.

### Observations

- **`/` lands in the in-app Approvals view, not the marketing page.** In demo-mode+auth-bypass the SPA puts you in the signed-in app shell. A demo-banner header is shown ("Demo mode — showing sample data. Sign in to see your own workspace.") but the camera will see the app, not marketing.
- **`/landing` is the marketing page** — on-brand and well-built. Hero: "Don't buy another tool. Hire a **teammate**. (Seriously.)" with Audiowide display font and a purple-to-pink gradient on "teammate". Sub-copy: "SKYCOT compiles plain-English job descriptions into AI teammates that live in Slack, learn from your feedback, and ask for approval before they act. Audit-first, single-binary, enterprise-grade."
- **Right-side activity cards** on the hero show the 5 canonical workers in action: Mariah "3 listings live, 2 buyers waiting on you" / Peta "standup digest drafted, 3 action items chased" / Priya "8 candidates screened today" / Jamal "12 tickets resolved, median reply 4m" / Nori "Q2 update draft ready for review."
- **Dual CTAs**: gradient "Hire your first teammate →" + outline "See how it works".
- **Trust signals row** under hero: NO CREDIT CARD · SOC 2-READY AUDIT TRAIL · ISO 27001 CONTROLS SUPPORTED · GDPR · HIPAA WORKFLOWS · EARLY ACCESS · APPLY.
- **HOW IT WORKS section**: "From plain English to a working teammate in four minutes." — 4 numbered steps (Describe the job / Meet your new teammate / Approve their first decisions / Watch the audit chain).
- **WHY SKYCOT section**: 6 reason cards — Named teammates, not tools / Asks before it acts / Edit workflows in chat / Versioned + reversible / Multi-channel native (Slack, iMessage, email, web inbox) / Audit-first, enterprise-grade.
- **TRUSTED BY section**: 5 customer logos — Northwind Commerce / Beacon Health Ops / Maple Finance / Apex Recruiting / Skyline Engineering.
- **Testimonials**: 3 — Priya Subramaniam (VP SUPPORT · NORTHWIND COMMERCE), Dr Mei Lin (CCO · BEACON HEALTH OPS), Liam Chen (FOUNDER · …).
- **In-app shell** (when /auth/bypass loaded): SKYCOT logo + "AI TEAMMATES" tagline in sidebar; left nav order Approvals / Team / Workers / Genesis / Cohorts / Skills / Workflows / Audit; footer "SELF-HOSTED · SINGLE BINARY" pill; account selector "Demo Operator · Acme Co · Owner"; theme toggle (light/dark); "Demo mode" pill bottom-left.
- **In-app Approvals view** (after a few seconds load): KPI strip with PENDING (0 active human decisions) / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS — all blank because fresh-tenant ⇒ no approvals exercised yet. Empty-state card "All clear · Your teammates have nothing waiting on you. Get back to work — they've got the rest." (warm, on-brand copy.)

### Deltas

- 🟢 **P2** — `/` in demo-mode lands in app shell, not marketing. Acceptable for the recording start-frame because we'll just open `/landing` directly as the cold-open URL. Not a blocker, not a degrader.
- 🟢 **P2** — "Install SKYCOT" PWA prompt persistently covers the bottom-right (~330×100px). It overlays the last testimonial card (Liam Chen) on mid-scroll. **Recording impact**: visible in the camera unless dismissed before recording. Worth a "demo-mode hides PWA install prompt" brief if it's easy to gate.
- 🟢 **P2** — Bottom-left floating "N" button is the **Naveed / notifications widget** — same widget on landing AND app shell. Verify intended on a public marketing page (might be a Skycot internal dev affordance leaking into prod).
- 🟢 **P2** — **Approvals empty-state copy is on-brand but the KPI strip is dead** (all 5 cards show em-dashes). For the recording, this looks correct for a brand-new tenant. Could be friendlier ("first approval coming · spinner") but not blocking.
- 🟡 **P1 — Open question for operator**: the in-app **Approvals page** shows "**0 waiting**" even though the seed claimed `approval_requests: 8`. Either the seed wrote 8 in a state that doesn't surface ("pending"?), or the team-list endpoint's tenant scoping is hiding them. Worth checking before Scene 6.

### Step 2 — `/auth/signup`

- **Hero**: "Hire your first **teammate**." Audiowide, gradient on "teammate". Eyebrow: "START YOUR FREE TRIAL".
- **Sub-copy**: "In **four minutes**, you'll set up your workspace, link a provider, and have a teammate working alongside you. **No credit card** required."
- **3 SSO options** stacked: Continue with Google / GitHub / Microsoft (each with brand logo).
- **"OR WITH EMAIL"** divider.
- **WORK EMAIL** label + input `you@yourcompany.com` placeholder.
- **"Send magic link →"** button (purple gradient, looks disabled until email entered).
- Reassurance copy: "We'll email you a one-tap sign-in link. No password to remember."
- **"Or try a demo workspace · no email required"** outline button below — demo-mode fast-path.
- Trust signals row at bottom: SOC 2-READY AUDIT TRAIL · GDPR · SSO READY.
- Top-right: "Already have an account? Sign in".

**Verdict so far**: ✅ matches the operator's locked auth-pages design (A editorial-center) and the magic-link-not-password vocabulary. Brand restraint is right for trust-building. PWA prompt still overlays the bottom-right.

### Step 3 — `/onboarding/welcome` (Step 01 of 05)

- **Top chrome**: demo-mode banner ("Demo mode — first-hire creates a staged teammate, not a real provisioning. Provider calls are simulated."); 5-segment progress pip with first segment filled (gradient); STEP 01 OF 05 label; WELCOME tag; "Help" link + "SAVE & EXIT" button on the right.
- **Hero**: "Hire your **first** **teammate**." Audiowide, gradient on "first" and "teammate". Eyebrow: "STEP ONE · WELCOME".
- **Sub-copy**: "In **four minutes** you'll onboard your first AI teammate — with your role, playbook, and guardrails ready before they start."
- **3 value-prop cards** below hero:
  1. **AUDIT-FIRST** — "Every action signed and replayable — SOC 2-ready audit trail."
  2. **GENESIS · ONBOARDING STUDIO** (middle, the operator's flagship pillar) — "Describe the role in plain English; Skycot synthesises charter, action vocabulary, memory schema, and eval suites — a real teammate ready to start work in minutes."
  3. **ENTERPRISE-GRADE** — "Multi-tenant, capability-based RBAC, GDPR Article 30 trails."
- **2 CTAs**: "Get started ↵" (purple gradient primary) + "Open Genesis studio →" (outline secondary).
- **Trust signals**: NO CREDIT CARD · SOC 2-READY AUDIT TRAIL · GDPR ARTICLE 30 · FOUNDING 100 · EARLY ACCESS COHORT.

**Verdict**: ✅ matches the operator's locked F1 onboarding design (editorial-center). Genesis is positioned as the flagship pillar (mid-card, the eye lands here first). Step-pip semantics fix from PR #2595 is visible in the progress UI.

### Step 4 — `/onboarding/workspace` (Step 02 of 05)

- Hero: "Name your **workspace**." Audiowide, gradient on "workspace".
- Sub-copy: "Workers live in workspaces. **You can rename them later** — this is just so we can stop calling it 'your workspace.'"
- Two form fields pre-filled (good for demo): **WORKSPACE NAME** "Acme Co" + helpful tip about multi-workspace naming (Acme Co · sales / Acme Co · ops); **COMPANY NAME** "Acme Inc."
- ← Back outline + Continue ↵ primary gradient.

**Verdict**: ✅ clean, friendly. Pre-filled values mean the demo flow doesn't stall on typing.

### Step 5 — `/onboarding/connect-provider` (Step 03 of 05)

- Hero: "Connect **Anthropic**." Audiowide, gradient on "Anthropic".
- Honest demo-mode sub-copy: "Workers run with a **mock provider** (simulated responses). Type any sk-ant- key to continue; no Anthropic call is made."
- ANTHROPIC API KEY field (placeholder `sk-ant-…`); "Get a key from the Anthropic console ↗" link below; reassurance "Your key stays in this browser tab. Closing the tab clears it."
- Yellow warning card: "**Mock provider** — workers run but responses are simulated. Skipping uses a mock provider until you add a real key in Settings · Providers."
- Three buttons: ← Back / "Skip for now" / "Validate & continue" (gates on input; activates on type — PR #2578 fix visible).

**Verdict**: ✅ honest about mocking. Three-button layout (back/skip/validate) is unusually rich for an onboarding step but legible.

### Step 6 — `/onboarding/pick-vertical` (Step 04 of 05)

- Hero: "Pick a starting **template**." Audiowide, gradient on "template".
- Demo-mode sub-copy: "Demo mode — pick a starting template. We'll install the real **skills, memory schema, and approval gates** once you create a real workspace."
- **5 templates only** despite 32 verticals seeded in the gateway: Accounts Receivable (5 skills) / GTM Engineer (8) / Customer Support (5) / Marketing Intern (5) / Sales SDR (5). Each card shows name, one-line description, 3 representative skill chips, total skill count.
- Footer: "5 starting templates"
- I picked **Customer Support** (maps to canonical Jamal).

### Step 7 — `/onboarding/first-hire` (Step 05 of 05) — **the Genesis input**

- Step pip fully filled (5/5).
- Hero: "Describe your **first worker**." Audiowide, gradient on "first worker".
- Sub-copy: "Plain English is fine. Give them a **name and a role**. You can set their schedule after they're hired."
- TELL US ABOUT THEM textarea, placeholder "A marketing intern named Maya who drafts blog posts and asks before publishing."
- Example caption same as placeholder.
- "← Back" + "✨ Hire them ↵" primary gradient (activates on type).
- I typed: *"A customer support teammate named Casey who triages inbound tickets, drafts polite FAQ replies for common questions, and escalates anything that needs a human decision. She asks for approval before sending any reply that quotes a price or makes a commitment."*

### Step 8 — clicking "Hire them" routes to `/workers/<canonical-Jamal-uuid>/overview`

The page that loads:
- Demo-mode banner switches to **"Demo mode — showing sample data. Sign in to see your own workspace."**
- Header reads "**Jamal** · Working", "handles customer support tickets", "Customer Success · Customer Experience", "12 tickets resolved".
- Worker tabs: Overview / Schedules / Activity / Memory / Skills / Trust / Escalation / Settings.
- "❤ HOW JAMAL WORKS" personality block (matches locked C2 design): trait chips **Calm under pressure / Empathic / Quick / Honest**; `✅ Great at` list (classifying severity <2m, pulling KB articles, follow-ups in customer voice, spotting same issue across tickets); `⚠ Escalate when` list (churn mention, 3+ tickets same customer/24h, billing >$200, VIP tier); SIGNATURE BEHAVIORS (always names human owner on escalation, sends "still investigating" at 4h, never closes without a one-line outcome).
- "What she does" card with "Adjust" pencil button — body uses **HE** pronouns ("Jamal helps… He resolves… escalates…") despite the **SHE** label in the card heading.
- "Responsibilities" toggle list (Resolve tier-1 tickets · Triage incoming tickets · Escalate billing disputes + churn risks · Send CSAT survey after resolution) with cadence chips (as tickets arrive / every 5 min / …).

### Deltas

- 🔴 **P0 — DEMO BLOCKER** — **first-hire ignores the typed name and routes to a canonical sample**. Operator typed "Casey", landed on **Jamal** (worker id `018f67d0-0000-7000-8000-000000000003`). This is the exact theater the operator's NO-THEATER directive forbids — the camera literally sees the lie. **Fix options**: (A) actually create the named worker from the description ("real Genesis"); (B) change the UI so the operator picks from a canonical roster (no name input); (C) hide the typed name and use it as memory_seed only while showing the canonical assignment as "we matched you to Jamal — your support specialist". (C) is fastest, (A) is what the brand promises.
- 🟡 **P1** — **vocab inconsistency on step 5**: eyebrow says "STEP 05 · YOUR TEAMMATE", hero says "Describe your first **worker**", example says "marketing intern named Maya". User-facing vocabulary (per operator-locked memory) should be **teammate** everywhere. Fix: change "first worker" → "first teammate" on this hero.
- 🟡 **P1** — **pronoun mismatch in Jamal "What she does" card**: header label uses "she", body uses "he". Either he or she, not both.
- 🟢 **P2** — **example placeholder in step 5 doesn't adapt to the picked vertical**. After picking Customer Support, the example still references "marketing intern named Maya". Should be a support-themed example.
- 🟢 **P2** — **only 5 templates exposed in onboarding picker** (vs 32 in the gateway). Confirm whether 5-curation is intentional or a bug. If intentional, add an "All templates" link for after-onboarding browsing.
- 🟢 **P2** — **PWA install prompt persistent through all 5 steps** — covers ~330×100px of the bottom-right. Should be suppressed in `NEXT_PUBLIC_SKYCOT_DEMO_LIVE=1` mode.

---

## Scene 2 — Delegation is the unit (Claim 2)

**Mission claim**: "Work happens by delegation between workers, with a chain-of-custody audit."
**Regression pin**: `claim_2_delegation_is_the_unit`
**Route flow**: `/workers/<Mariah-id>` → activity tab → trigger a `delegate-task` → `/audit` shows handoff_id chain.

### Observations

- **Worker Activity tab (`/workers/<id>/activity`)** loads fine. Filter chips: TIME RANGE (Last 7 days) · ACTION TYPE (All) · STATUS (All). Two events showing for Jamal: "resolved tier-1 ticket #SUP-29382 with KB article SK-413" (9m ago, Succeeded, $0.004) and "escalated refund request to support lead" (2h ago, Succeeded). "Open in Audit timeline →" link at the bottom.
- **Action IDs in the Activity tab are synthetic fixtures** — format `act_<worker-uuid>_NNN` (e.g. `act_018f67d0-0000-7000-8000-000000000003_001`). These do NOT exist in the real audit store. They are mock fixture data leaking through demo-live mode.
- **Clicking the "escalated refund request" row** routes to `/audit/action/act_018f67d0-...-001` which returns the **"Action not found"** empty state with text "No audit record matches `act_<…>_001`. The action may have been redacted, hard-deleted by a retention policy, or the id is incorrect." This is the **delegation moment Scene 2 promises** — and the camera lands on a 404.
- **Real `/audit` page populated**: filter "Today (default)", header "Audit · Every action, hash-chained and replayable." Search + Export. Rows show real action_ids (32-hex like `019e6cd2073978219924ce38a5777547`), real self_hash (truncated), real worker avatars (NO/JA/PR/PE/MA for the canonical 5), all status=Succeeded. Pagination scrolls smoothly.
- **All visible audit rows are `worker.kpi_rolled`** events from the demo seed's wrap-up step. NO `delegate-task`, NO `handoff`, NO escalation-flavored events anywhere in the real feed. The seed has no actual delegation activity baked in.
- **Drilling into ANY real audit row returns "Action not found"** too. Verified via direct gateway curl: `GET /api/v1/audit?limit=5` returns 3 valid items (worker.kpi_rolled for Nori, Jamal, Priya); `GET /api/v1/audit/<that-very-id>` returns `{"error":{"code":"not_found","message":"audit row not found"}}`. PR #2589 (merged today, "align gateway audit list/detail tenant resolver via shared extract_tenant") was supposed to fix this — either the fix incomplete or there's a second tenant-scope drift not covered. Path variants `/api/v1/audit/<id>`, `/api/v1/audit/action/<id>`, `/api/v1/audit/actions/<id>` all 404.
- **Tab title regresses to raw UUID** when on the Activity tab — chrome shows "018f67d0-0000-7000-8000-000000000003 · Activity" instead of "Jamal · Activity". (Overview tab title was correct.)

### Deltas

- 🔴 **P0 — DEMO BLOCKER** — **Worker Activity tab serves synthetic fixture IDs that 404 on drill-in.** This kills Scene 2's headline beat: the camera clicks the delegation event and lands on "Action not found." Fix: replace the fixture data path with the real audit-feed query filtered by `worker_id` (the gateway already has the `/api/v1/audit?worker=<id>` endpoint per the audit page wiring). May need a new SPA fetcher or a real audit-event row for this worker in the seed.
- 🔴 **P0 — DEMO BLOCKER** — **EVERY drill-in on `/audit/<id>` returns 404 — even with real IDs from the list endpoint.** This is the list/detail tenant resolver drift PR #2589 was meant to fix, but the symptom is still present on the v84 gateway built from `9e19589`. Either the fix didn't land cleanly or there's a second-order tenant scope bug. Without a working drill-in, Scenes 2 and 6 both lose their chain-of-custody payoff.
- 🔴 **P0 — Demo seed has no actual delegation events.** The 150 dressing rows + 12 Pass-A scaffolding rows + KPI rolls all read as single-worker activity, no handoff_id. To satisfy Claim 2's "delegation is the unit", the seed needs at least one realistic A→B handoff chain (e.g. Jamal escalates SUP-29382 to a human approver, with a real `handoff.created` audit row whose `parent_id` links to Jamal's tier-1 resolve attempt). Add to seed profile `v1-demo-canonical`.
- 🟡 **P1** — **Worker detail tab title falls back to UUID on non-Overview tabs.** chrome shows "018f67d0-0000-7000-8000-000000000003 · Activity" instead of "Jamal · Activity". Worker name should resolve consistently across all tabs.

---

## Scene 3 — Platform builds workers / Genesis (Claim 3)

**Mission claim**: "The platform builds workers — operator writes a role, Genesis turns it into a vertical."
**Regression pin**: `claim_3_platform_builds_workers`
**Route flow**: `/genesis` "Onboarding studio" page (sidebar nav).

### Observations

- **`/genesis` page exists and is brand-loud**: gradient violet/coral hero banner, eyebrow "GENESIS · ONBOARDING STUDIO", hero "Onboard a new **teammate**." (Audiowide, gradient on "teammate"), sub-copy "Describe the role, upload your SOPs, wiki pages, Slack threads or voice notes, then scope their access. Genesis trains the teammate and checks comprehension before they start work."
- **Only the textarea is built**: no SOP/wiki/Slack/voice upload affordance, no access scoping UI, no comprehension verify UI. The hero promises features that don't ship in v1.0.
- **"Generate worker" button** below — outline gradient (disabled until input), helper text "We'll draft a real worker — persona, charter, and skills — in about ten seconds."
- **Clicking Generate returns "Failed to fetch"** error card with "Try again" + "Modify description" buttons.
- **The real problem is a field-name mismatch**: gateway endpoint `POST /api/v1/genesis` expects `role_description`, SPA sends `description`. Gateway returns 400 `{"error":{"code":"bad_request","message":"parse body: missing field \`role_description\` at line 1 column 60"}}`. The SPA's error handler renders the generic "Failed to fetch" instead of surfacing the gateway's `error.message`.
- **HUGE: Genesis is REAL on the backend**. With the correct field name, `POST /api/v1/genesis` returns a **Server-Sent Events stream** with named phases:
  ```
  event: progress data: {"message":"Resolving persona slug","percent":10,"phase":"drafting_persona"}
  event: progress data: {"message":"Compiled archetype: lead_researcher","percent":35,"phase":"compiling_charter"}
  event: progress data: {"message":"Writing manifest, prompts, eval seeds","percent":65,"phase":"generating_skill_bundle"}
  ```
  The gateway already does archetype matching, charter compilation, manifest + prompt + eval-seed generation, and skill bundling. The operator's "Phase 2 Genesis = 4 weeks" estimate is wrong — most of Phase 2 is BUILT. The SPA just needs (a) field-name fix, (b) SSE consumer to render progress, (c) final-state rendering.

### Deltas

- 🔴 **P0 — DEMO BLOCKER if Scene 3 visits `/genesis`** — clicking "Generate worker" produces "Failed to fetch". Root cause: SPA sends `description`, gateway expects `role_description`. Two paths: (A) hide /genesis from the demo recording (skip sidebar link) + fall back to /onboarding/first-hire as the Claim-3 surface; (B) ship the field-rename + SSE consumer + final-state rendering in a small brief (probably <1 day given backend is built).
- 🟡 **P1** — **hero copy over-promises**: SOPs, wiki pages, Slack threads, voice notes, access scoping, comprehension verify — none of these UIs exist. Until they ship, the hero copy should match what the UI actually does (text-only description → worker draft). Operator NO-THEATER rule applies.
- 🟡 **P1** — **error envelope not surfaced**: SPA shows "Failed to fetch" when the gateway returned a perfectly readable 400 with `{"error":{"code":"bad_request","message":"..."}}`. The error display should read `.error.message` and render it, so future debugging is one step shorter.
- 🟢 **P2** — **vocab drift in button**: "Generate **worker**" (not teammate). Same operator-locked vocab rule. Change to "Generate teammate" or "Onboard them".

### Major operator finding from this scene

**Backend Genesis is already real.** The endpoint produces personas, archetypes, charters, manifests, prompts, eval seeds, and skill bundles via streaming progress events. The "v1.1 Genesis Phase 2" timeline should be re-estimated: the bottleneck is now the SPA consumer + final-state rendering, not the AI/orchestration layer. Probably 1–3 days of SPA work to ship a real "describe → draft → verify → hire" flow that delivers what the marketing landing page promises today.

---

## Scene 4 — Skills shared + learned (Claim 4)

**Mission claim**: "Skills are shared between workers and improve over time from real use."
**Regression pin**: `claim_4_live_install_invoke`
**Route flow**: `/skills` marketplace → click skill → see provenance/learning-loop/workers-using; OR `/workers/<id>/skills` → "Try it" on a library skill → it moves to Active.

### Observations

- **`/skills` marketplace** is brand-loud and ships well: gradient hero "Every skill your team **will ever need**.", hero search bar with ⌘K + "7 skills · 7 shown" count + TRY-pill chips (approval / invoice / candidate / deal / support / lead / "Skills my workers learned this month"). 6 featured cards in 3-col + 1 below: Pricing competitor scrape · Approval route · Invoice extract · Candidate screen · Deal qualify · Support triage · Lead enrich. Each card: gradient icon, name, one-line description, category chip, VERSION + VERIFIED status, skill-id.
- **Clicking "Support triage" → `/skills/support.triage`**: SKILL DETAIL eyebrow + Audiowide skill-id hero. Three cards: **PROVENANCE** (Learned by Mariah · 2026-05-23 + Trust · D), **LEARNING LOOP · TRUST GRADE B** ("No runs yet — `support.triage` ships at Trust B and earns A through real workers."), **WORKERS USING THIS SKILL** (Mariah · Learned 4 days ago · "Used by 1 worker across the SKYCOT marketplace.").
- **`/workers/<jamal>/skills` (the install/use surface)**: three sections — **Active** (tier1-ticket-resolver v6.0.2 · trust 95% · used 412× last 7d, ticket-routing-classifier v2.3.0 · trust 88% · used 188× last 7d) · **Library** (refund-eligibility-checker v1.1.0 · trust 84% · Workspace, csat-followup-scheduler v1.0.0 · trust 72% · Shared) · **Learned** (auto-pause-on-hold-tickets · 6 successes · 1 fail · Promote / Discard). Each Active row has Disable / View source / Audit usage links.
- **"Try it" works end-to-end**: clicking on refund-eligibility-checker moved it Library → Active instantly, showing "trust: 84% / used 0× last 7d". Library section shrank. Clean install moment.
- **"Audit usage" click** routes to `/audit?worker=<jamal-uuid>&skill=sk-jamal-resolve` (raw UUID in URL params). Page renders the audit shell but body says "No actions match the current filters. Try removing the worker filter or widening the time range." This is the same tenant-scope drift from Scene 2 — list/detail mismatch. Worker filter chip displays the raw UUID, not "Jamal".

### Deltas

- 🟢 **P2 — Scene 4 ships strong as-is.** The Active/Library/Learned three-rail layout on `/workers/<id>/skills` is the canonical Claim-4 surface. "Try it" works. "Promote/Discard" on Learned is the operator's eureka moment ("the worker discovered a new skill — promote it"). Recommend Scene 4 dwells here, NOT on the /skills marketplace (which is a brochure page without an install affordance).
- 🟡 **P1** — **two parallel skill catalogues**: /skills marketplace shows 7 verified skills (Pricing competitor scrape / Approval route / Invoice extract / Candidate screen / Deal qualify / Support triage / Lead enrich); worker-skills page shows 5 different ones (tier1-ticket-resolver / ticket-routing-classifier / refund-eligibility-checker / csat-followup-scheduler / auto-pause-on-hold-tickets). They're keyed differently (`support.triage` vs `support-triage` vs `tier1-ticket-resolver`). Both sets are valid for a demo as long as they don't appear on the same scene — but eventually they need reconciling.
- 🟡 **P1** — **Skill detail page has an internal trust-grade contradiction**: PROVENANCE card says "Trust · D" and the LEARNING LOOP card heading right below says "TRUST GRADE B". Pick one.
- 🟡 **P1** — **Provenance "Learned by Mariah" mismatch**: support.triage is the support-vertical skill, Mariah is the marketplace-lister persona. Seed should assign support.triage's "learned by" to Jamal.
- 🟡 **P1** — **Audit-usage drill-down empty state for Jamal+skill** (same root cause as Scene 2's drill-in P0 — list/detail tenant scope drift). Worker filter chip also leaks the raw UUID.
- 🟡 **P1** — **Worker detail tab title still UUID on `/skills`** ("018f67d0-... · Skills" not "Jamal · Skills").
- 🟢 **P2** — empty-state audit drill-in shows "Load more — page 2" button under "No actions match" empty state. Inconsistent UI state.

---

## Scene 5 — Any-PC execution (Claim 5)

**Mission claim**: "Workers execute on any PC via macOS computer-use or remote substrate."
**Regression pin**: `claim_5_live_macos_cu_smoke_test` (+ MCP cap audit + pool routing pins)
**Route flow**: `/workflows/<customer_research_followup>` → Run now → `cu.run_macro` opens macOS Notes → audit captures the action.
**Tooling note**: I did NOT click "Run now" during this walkthrough because that would open macOS Notes (operator-permissioned binary, real side-effect). The recording-time Run is for the operator to drive.

### Observations

- **`/workflows` index** is brand-loud and on-message: gradient hero "How your teammates **actually** **think**.", sub-copy positioning workflows as plain-English playbooks. Search bar + "1 workflows · Browse 53+ templates · Propose new" buttons. TRY chips approve / qualify / digest / triage. One workflow card visible: `customer_research_followup` (MARKETPLACE · AUTONOMOUS · 3 steps · v1 · MACOS · IDLE · by argo-demo-seed). Card is the lead-tile from the operator's locked workflow-goldens design.
- **`/workflows/marketplace/customer_research_followup`** is rich and demo-ready:
  - Header band: MARKETPLACE · AUTONOMOUS · v1 · 🖥 MACOS · IDLE · WORKFLOW · workflow-id hero.
  - Five action buttons: ▶ **Run now** (purple gradient) / 📝 Open builder / 💬 Edit in chat / 🔀 Version history / ➕ Add as Schedule.
  - STEPS · 3 TOTAL: (1) SKILL · `fetch_thread` (2) SKILL · `summarize_question` (3) TOOL CALL · `drop_note` · 🖥 MACOS · IDLE — with edit pencils per step.
  - RUN HISTORY · LAST 7 DAYS card: **172 RUNS / 87% SUCCESS RATE / 4.3s AVG DURATION / LAST RUN 2026-05-20** — 4-stat strip in big numbers.
  - Filter chips: Last 24h / Last 7d (active) / Last 30d / Custom / Success / Failed / Running / Queued.
  - RECENT RUNS table (6 visible rows): RUN ID `wfr_01HX…` / STARTED / DURATION / STATUS (SUCCEEDED green, FAILED red).
- **Substrate badge `MACOS · IDLE`** appears in 4 places on this page (header band, step 3, the workflow card on the index) — visually consistent.

### Deltas

- 🟢 **P2 — Scene 5 surface ships well as a STATIC display.** The page communicates everything Claim 5 needs: 3-step recipe, macOS substrate label, recent-run history with real-looking stats. The recording will work if the Run-now click triggers cu.run_macro cleanly. **The unknown is the click**: I deferred testing it to avoid interrupting the operator's macOS state, so we need a dry-run of Run-now before the camera rolls — ideally with the recording terminal already foregrounded on a clean macOS desktop.
- 🟡 **P1** — **172 RUNS / 87% SUCCESS / 4.3s AVG / LAST RUN 2026-05-20 is seed-fixture data**. The dates (2026-05-19, 2026-05-20) come from the seed and won't update — on recording day (presumably ≥2026-05-28) "LAST RUN 2026-05-20" reads as "8 days ago" which is fine. But all the success/failure rows have static IDs and won't reflect a real Run-now click during the demo. Either (A) wire the run-history card to query the live audit feed filtered by workflow_id, or (B) accept the cosmetic mismatch (recommended for demo MP4).
- 🟡 **P1** — **"Browse 53+ templates" CTA** — clicking it goes where? If marketplace is empty (only 1 workflow visible), the "53+" promise is theater.
- 🟢 **P2** — **Run IDs in table look like ULIDs (`wfr_01HX…`) but aren't truncated to a link**. Drilling into a past run isn't reachable from this view. Worth a brief.
- 🟢 **P2** — **Workflow vertical-id is "marketplace" but the workflow's content (Zendesk thread, customer support follow-up) is support-themed**. Seed mismatch — should be vertical-id "support".

---

## Scene 6 — Multi-channel + multi-tenant (Claim 6)

**Mission claim**: "Approvals route through real channels (Slack, SMS, etc.) and never leak across tenants."
**Regression pin**: `claim_6_cross_tenant_approval_select_no_existence_leak`
**Route flow**: `/approvals` (channel chip per row) → `/audit/action/<id>` drill-in shows tenant scope on every payload field.

### Observations

- **`/approvals` page is on-brand and gracefully designed**: gradient hero banner "INBOX · Approvals." with sub-copy "Every decision your AI teammates queue for a human shows up here — refunds, hires, contracts, anything that needs a sign-off." 5 KPI cards (PENDING / $ AT RISK / MEDIAN TIME / APPROVAL RATE / ACTIVE REVIEWERS) — all empty.
- **3 severity tiers each with proper empty-state**: 🔴 P0 HIGH "Over SLA or high-risk decisions" / 🟡 P1 MEDIUM "Elevated work still inside SLA" / 🟢 P2 LOW "Routine confirmations" — each says "No <X> severity approvals waiting." Tier semantics + empty-state copy is well-considered.
- **Right rail** has 3 explainer cards: `$ at risk` (0 pending with monetary exposure), `Confidence` (0% conf · "Mean worker confidence across pending rows" — gauge donut), `Approval rate` (— · "No resolved approvals yet").
- **THE PAGE IS EMPTY despite the seed creating 8 approval_requests.** Direct sqlite verification: `SELECT tenant_id, COUNT(*) FROM approval_request GROUP BY tenant_id` shows the workspace tenant has 8 rows. The gateway `/api/v1/approvals` returns `{items: [], count: 0}`. Even adding explicit `X-Argo-Tenant-Id` + `X-Argo-Principal` headers returns 0. Tenant-scope drift in `routes_approvals_json::list_response`.
- **`/api/v1/approvals/pending` returns 404** — endpoint not mounted; only `/api/v1/approvals` exists.

### Deltas

- 🔴 **P0 — DEMO BLOCKER** — **Approvals page is empty.** Scene 6's whole story is "look at this pending refund · see the Slack channel chip · click drill-in → see the audit chain proves tenant isolation." With 0 visible approvals, none of that ships. Root cause is the same tenant-scope drift family as Scene 2's drill-in 404 — the gateway's approvals list query doesn't resolve the workspace tenant the same way the seed wrote to. Fix candidates: (A) align `routes_approvals_json::list_response` to use the same `cwd_write_tenant` / `ensure_workspace` path the seed uses; (B) re-run the seed with explicit tenant=read-resolved-tenant; (C) update the seed to write to whatever tenant the read-side resolves.
- 🔴 **P0 — DEMO BLOCKER** — **Multi-channel proof requires a visible row with a Slack channel chip.** Once approvals surface, we still need to verify the row shows a "🟣 Slack" channel pill (or webinbox/email/sms/push) per row — PR #2569 "render channel chip on every row" was one of today's drain-wave merges; it's on tip but unverifiable until rows surface.
- 🟢 **P2 — Empty-state design is good.** When this DOES populate, the severity-tiered shell will be camera-ready. The 5-KPI strip, 3 right-rail explainers, and 3 severity sections give the camera plenty to dwell on per row.
- 🟢 **P2 — Cross-tenant negative check (Scene 6's invisible payoff)**: the seed already creates `foreign_workers: 1`, `foreign_skills: 1`, `foreign_audit_rows: 1` in a separate tenant per the dem seed output. The regression test `claim_6_cross_tenant_approval_select_no_existence_leak` proves they don't leak. But the camera can't *see* that — it's a property test. Recording-time presentation: narrate it ("if I were on a different tenant, none of this would be visible — Claim 6 is enforced at the SQL layer with this regression test") + show the test passing in a terminal split. Don't try to demonstrate it interactively.

---

## Punch-list (synthesized from deltas)

### 🔴 P0 — must close before MP4 recording

| # | Scene | Title | Brief slug (proposed) |
|---|---|---|---|
| 1 | 1 | first-hire textarea is theater — replace with canonical-roster card | **DONE in 718fb9f6d** — `fix-onboarding-first-hire-canonical-picker-not-textarea` |
| 2 | 2 | worker Activity tab serves synthetic fixture IDs that 404 on drill-in | `fix-worker-activity-tab-real-audit-rows-not-fixture-ids` |
| 3 | 2 | EVERY `/audit/<id>` drill-in returns 404 — list/detail tenant scope drift (PR #2589's fix incomplete) | `fix-gateway-audit-detail-tenant-scope-drift-not-fully-closed-by-2589` |
| 4 | 2 | demo seed has no real delegation/handoff events — only `worker.kpi_rolled` rows | `fix-demo-seed-add-real-delegation-handoff-chain-for-claim-2` |
| 5 | 3 | `/genesis` "Generate worker" 400s on field-name mismatch (SPA `description` vs gateway `role_description`) + no SSE consumer | `fix-genesis-spa-field-rename-and-sse-consumer-for-progress` |
| 6 | 6 | `/approvals` page empty despite 8 seeded approval_requests — list-side tenant scope drift | `fix-gateway-approvals-list-tenant-scope-not-resolving-seed-tenant` |
| 7 | 6 | Slack channel chip on approvals row unverifiable until #6 closes | (rolls up into #6 — verify post-fix) |

### 🟡 P1 — fix-and-still-ship (post-MP4 OK, but degrades the demo)

| # | Scene | Title | Brief slug (proposed) |
|---|---|---|---|
| 8 | 1 | "first worker" hero → "first teammate" vocab | **DONE in 718fb9f6d** |
| 9 | 1 | Jamal "What she does" he/she pronoun mismatch | **DONE in 718fb9f6d** (heading → "What they do") |
| 10 | 2 | worker detail tab title leaks raw UUID on non-Overview tabs ("018f67d0-... · Activity") | `fix-worker-detail-tab-title-resolve-name-not-uuid-across-tabs` |
| 11 | 3 | `/genesis` hero over-promises (SOP upload, Slack threads, voice notes, access scoping, comprehension verify) — none exist | `fix-genesis-hero-copy-match-actual-text-only-ui` |
| 12 | 3 | SPA error envelope not surfaced (gateway returns `error.message`, SPA shows "Failed to fetch") | `fix-genesis-and-onboarding-surface-gateway-error-envelope` |
| 13 | 4 | two parallel skill catalogues — /skills marketplace (7 skills) vs /workers/<id>/skills (5 different skills) — keyed differently | `fix-skill-catalogue-reconcile-marketplace-vs-worker-skills` |
| 14 | 4 | skill detail page contradicts itself — PROVENANCE "Trust · D" vs LEARNING LOOP "TRUST GRADE B" | `fix-skill-detail-page-trust-grade-internal-contradiction` |
| 15 | 4 | provenance "Learned by Mariah" on support.triage (vertical mismatch — should be Jamal) | `fix-demo-seed-skill-provenance-match-persona-vertical` |
| 16 | 4 | Audit-usage drill-down from worker-skills shows empty + raw UUID in filter chip | (rolls up into #3 — tenant scope) |
| 17 | 5 | workflow recent-runs table is seed-fixture data (172 runs / 87% / 2026-05-20 LAST RUN) — won't update on a live Run-now click | `fix-workflow-detail-recent-runs-wire-live-audit-feed` |
| 18 | 5 | "Browse 53+ templates" CTA on workflows page — marketplace appears empty | `fix-workflows-browse-templates-cta-points-to-empty-marketplace` |
| 19 | 5 | workflow `customer_research_followup` has vertical-id "marketplace" but content is support-themed | `fix-demo-seed-customer-research-followup-vertical-id-marketplace-to-support` |
| 20 | 1 | only 5 templates in onboarding picker vs 32 verticals in gateway | (operator-decisional — intentional curation or bug?) |

### 🟢 P2 — polish

| # | Scene | Title |
|---|---|---|
| 21 | 1 | `/` in demo-mode lands in app shell, not marketing (cold-open URL should be `/landing`) |
| 22 | 1 | PWA install prompt persistent through all 5 onboarding steps + most pages — should be suppressed in DEMO_LIVE mode |
| 23 | 1 | example placeholder in step 5 doesn't adapt to picked vertical (still says "marketing intern named Maya") |
| 24 | 3 | Genesis "Generate worker" button — "worker" vocab again |
| 25 | 4 | empty-state audit drill-in shows "Load more — page 2" button inconsistently |
| 26 | 5 | workflow Run IDs in recent-runs table aren't drill-in links |

---

## Ship decision — preliminary read

**Status**: **🔴 NO-GO for MP4 recording today**. The 6 P0 items above gate the recording.

Breakdown by claim:
- **Claim 1** — ✅ shippable after the Scene-1 fixes that landed in `718fb9f6d`
- **Claim 2** — ❌ blocked by audit drill-in 404 + no real delegation in seed (3 P0s)
- **Claim 3** — ❌ blocked by Genesis field-name + SSE (1 P0 OR hide /genesis from the recording)
- **Claim 4** — ✅ Scene 4 ships strong as-is on `/workers/<id>/skills`; recording avoids /skills marketplace detail page where the trust-grade contradiction lives
- **Claim 5** — ⚠️ surface is ready, but the Run-now click hasn't been verified — needs a pre-record dry-run with macOS Notes
- **Claim 6** — ❌ blocked by `/approvals` empty state (1 P0)

### Fastest path to GO

Three of the six P0s (#3, #6, plus the audit-usage in #16) trace back to a **single root cause: gateway tenant-scope drift on list/detail/scoped queries**. PR #2589 was meant to fix this for audit list/detail; it apparently only fixed part of it. **One focused brief that audits every `list_response` and `detail_response` handler against `cwd_write_tenant` / `ensure_workspace`** could close 3 P0s + 1 P1 in one PR.

That leaves the demo-seed P0s (#2, #4) and the Genesis SPA P0 (#5):
- **#2** — replace worker Activity-tab fixture data with real `/api/v1/audit?worker=<id>` queries. Web-only.
- **#4** — add a real delegation handoff row pair to the demo seed (Jamal → Priya escalation with `handoff_id`). Rust + seed.
- **#5** — SPA Genesis field-rename + SSE consumer + final-state page. Web-only.

**Realistic timeline**: 1–2 days of focused work to ship the 6 P0s and re-walk all 6 scenes. THEN tag `v1.0-mp4-recording-ready` and record.

The Scene 4 finding that **backend Genesis is already real** (SSE stream with archetype/charter/manifest phases) re-prices the operator's "v1.1 Genesis Phase 2 = weeks" estimate down to "days of SPA work". That's a strategically large finding — worth a separate conversation post-MP4.

