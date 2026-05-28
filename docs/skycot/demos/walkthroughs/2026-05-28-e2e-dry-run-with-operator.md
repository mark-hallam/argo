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

### Deltas

---

## Scene 3 — Platform builds workers / Genesis (Claim 3)

**Mission claim**: "The platform builds workers — operator writes a role, Genesis turns it into a vertical."
**Regression pin**: `claim_3_platform_builds_workers`
**Route flow**: `/onboarding/first-hire` Genesis flow — text role + vertical pick + (eventually) SOP-PDF upload + comprehension verify.

### Observations

### Deltas

---

## Scene 4 — Skills shared + learned (Claim 4)

**Mission claim**: "Skills are shared between workers and improve over time from real use."
**Regression pin**: `claim_4_live_install_invoke`
**Route flow**: `/skills` marketplace → install one → `/skills/<name>` learning-loop card shows LEARNED telemetry.

### Observations

### Deltas

---

## Scene 5 — Any-PC execution (Claim 5)

**Mission claim**: "Workers execute on any PC via macOS computer-use or remote substrate."
**Regression pin**: `claim_5_live_macos_cu_smoke_test` (+ MCP cap audit + pool routing pins)
**Route flow**: `/workflows/<customer_research_followup>` → Run now → `cu.run_macro` opens macOS Notes → audit captures the action.
**Tooling note**: Notes app is a native macOS app — outside Chrome — switch to `mcp__computer-use__*` tools for this scene.

### Observations

### Deltas

---

## Scene 6 — Multi-channel + multi-tenant (Claim 6)

**Mission claim**: "Approvals route through real channels (Slack, SMS, etc.) and never leak across tenants."
**Regression pin**: `claim_6_cross_tenant_approval_select_no_existence_leak`
**Route flow**: `/approvals` (channel chip per row) → `/audit/action/<id>` drill-in shows tenant scope on every payload field.

### Observations

### Deltas

---

## Punch-list (synthesized from deltas)

_(filled at end — each P0/P1 will get a TODO brief in `docs/build-plan/streams/`)_

| ID | Sev | Scene | Title | Brief slug |
|---|---|---|---|---|

---

## Ship decision

_(written after Scene 6: GO / GO-with-followups / NO-GO)_
