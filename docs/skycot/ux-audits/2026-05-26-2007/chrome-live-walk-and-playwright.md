# Chrome live-walk + playwright e2e — T20:07 MEL (2026-05-26T10:07Z)

**Source**: live operator-driven Chrome walk via Claude-in-Chrome MCP + full playwright e2e suite run.

**Operator directive (mid-tick)**: "use the chrome browser and e2e test all the tests".

**Scope**: Authoritative ground-truth pass against the running dev server (port 3000, PID 95164 after playwright restart). Gateway alive at :7400 (schema v83, /health = ok).

---

## TL;DR

- **Playwright e2e**: 22 pass / **53 fail** (71% failure rate). 38 spec files exercised.
- **Live Chrome walk**: 6 distinct P0 findings across landing → /workers → /approvals → /audit → /skills → /workflows → /onboarding (wizard).
- **Headline**: The investor demo flow is **NOT walkthroughable end-to-end right now**. Onboarding wizard step 2 leaks raw `X-Argo-Principal` gateway error. Audit drill-in returns 500. Skills marketplace shows 0 skills. Approvals inbox is empty while 4 worker cards say "Waiting on you".

---

## Playwright failure clusters (53 failed)

| Cluster | Count | Specs |
|---|---|---|
| **Hire-flow / onboarding** (load-bearing!) | 8 | hire-flow-shell × 2, hire-flow-step2 × 2, hire-flow-step3 × 3, hire-flow-step4 × 2, onboarding.spec |
| **Workflow builder / save / publish / rollback** | 13 | builder-add-step, builder-controlled-state, builder-delete, builder-duplicate × 2, builder-shortcuts, workflow-concurrent-edit, workflow-publish-real × 3, workflow-rollback × 1, workflow-rollback-button × 2, workflow-save-real × 2 |
| **Vocabulary: Duty → Schedule** | 6 | no-duty-copy (settings / workflows / cohorts pages still say "Duty"; settings / workflows / cohorts pages do NOT yet say "Schedule") |
| **Auth** | 4 | auth-logout × 3, auth-signup-centering |
| **Dark-mode white-on-white** | 4 | /pricing, /compare, /docs, /for/marketplace |
| **Approvals** | 3 | approvals-filter, approvals-inbox, approvals-live |
| **Team page** | 3 | team-filter-search, team-page-shows-real-workers × 2 |
| **Audit** | 2 | audit-cross-system-links, audit-filter-popover |
| **Hire celebration** | 2 | hire-celebration-banner × 2 |
| **Misc** | 8 | cohorts-project-ops, hire-flow-step2-listen redirect, hire-flow-step3-drafting redirect, hire-flow-step4-introduce × 2 redirect, orchestration-queue, skill-canvas-editor, trust-promotion |

Full run.log under `playwright/run.log` in this directory.

---

## Live Chrome walk — findings

Dev server: port 3000. Gateway: port 7400, schema v83. Viewport: 1440 × 900.

### F1 — P0 — Approvals empty inbox while 4 worker cards say "Waiting on you"

**Pages**: `/team` (a.k.a. `/workers` — see F1.1) vs `/approvals`.

**Evidence**:
- `/team` worker deck shows 4 of 5 cards with **"Waiting on you"** chip: Mariah, Peta, Priya, plus implicitly all but Jamal+Nori (Jamal = Working, Nori = idle).
- `/approvals` shows empty-state card: **"No approvals waiting"** with copy "When a worker hits a decision that needs sign-off... it lands in your inbox at home."
- Both pages are running off the same demo-fallback sample data; the two surfaces disagree about pending approvals.

**Demo impact**: Scene 6 (Claim 6 — multi-tenant trust surface) cannot be demoed. The "press one button → see your inbox of decisions waiting" moment shows an empty mailbox.

**Root cause hypothesis**: the worker-card "Waiting on you" chip is derived from a different (stub) sample dataset than the approvals inbox query. Likely two separate sample fixtures that have drifted.

**Page also missing**: per `project_approvals_page_design_locked.md` the E-ops-dashboard design specifies a 5-KPI strip, 3 severity tiers, $ at risk + confidence donuts, 7-day approval rate ring. **None of that renders** — the page short-circuits to the empty state. So the locked design is not just under-populated; it appears not adopted in this code path.

### F1.1 — P2 — `/workers` redirects to `/team` (and the sidebar shows both)

The URL `/workers` 302s to `/team`. The left sidebar lists **both** "Team" and "Workers" as separate entries — only one of them is the actual page. Brand-and-IA inconsistency. Either drop one entry or make them genuinely different surfaces.

### F2 — P0 — Audit drill-in returns 500

**Page**: `/audit/action/019e63c2cd537671847bbc84c12cbbd5` (first row in `/audit` list).

**Evidence**: Click row → server-rendered 500 error page: "We couldn't load this audit action. Something went wrong fetching the action's details. Retrying usually fixes it; otherwise the audit feed is always available." with Try-again and Back-to-audit CTAs.

**Demo impact**: Scene 6 trust-surface "click row → see what really happened → hash-chain verify" is the climax of Claim 6. It's a hard 500. Demo blocker.

**Root cause hypothesis**: the audit-action detail route either:
- doesn't have a demo-mode fallback wired (gateway request 4xx/5xx with no sample-data path), or
- the rendered ID format `019e63c2cd...` doesn't match the action-id schema the detail route expects.

### F3 — P1 — Audit log is homogeneous demo-spam

**Page**: `/audit`.

**Evidence**: 13 visible rows in the "TODAY" section. **Every single row** is action type `http.request` from actor `UN` (unknown) with status `Succeeded`. No `skill.invoked`, no `approval.granted`, no `worker.hired`, no `delegation.assigned`, no `mcp.tool.call`, no `cu.action.taken`. The "5 teammates doing real work" narrative cannot be told from this log.

**Demo impact**: Even if F2 (drill-in 500) is fixed, the surface that gets reached is a wall of identical `http.request` rows. Reads as log spam, not as "audit-first AI workforce" — directly contradicts the landing-page promise.

**Fix**: seed sample data needs variety of action kinds — at minimum one each of skill.invoked / approval.granted / approval.escalated / delegation.assigned / worker.hired / cu.action.taken / mcp.tool.call.

### F4 — P0 — Skills marketplace shows 0 skills

**Page**: `/skills`.

**Evidence**:
- Hero search bar count pill shows **"0 skills · 0 shown"** (cosmetic — but accurate)
- "FEATURED SKILLS" panel shows **"Loading skills from the marketplace..."** indefinitely (still loading at 4s, still loading at 8s in subsequent recheck)

**Demo impact**: Scene 4 (Claim 4 — skills shared + learned) cannot be demoed. The search→install→worker-uses-skill flow has nothing to search for.

**Root cause hypothesis**: `/skills` page calls gateway `/v1/skills` (or similar); in demo mode this either returns empty or never resolves. PR #1682 wired the runtime side of skill-library loading, but the marketplace listing endpoint may not have a demo-mode fixture.

### F5 — P0 — Onboarding wizard Step 2 → Continue surfaces raw gateway error

**Page**: `/onboarding/workspace` (step 02 of 05).

**Evidence**: With workspace name "Acmeco Growth" and company name "Acmeco Inc.", clicking "Continue" displays inside the form card, in red:

> X-Argo-Principal header must carry a verified `human:<email>` value

**Demo impact**: Cannot advance past Step 2 of the 5-step wizard in demo mode. **The investor narrative cannot be walked end-to-end.** This contradicts the landing page promise ("From plain English to a working teammate in four minutes" — the whole point is that anyone can do it without signing up).

**Two sub-issues**:
- **F5a**: Demo mode is supposed to bypass gateway auth (the top banner says "Demo mode — showing sample data. Sign in to see your own workspace.") — but Continue on Step 2 still fires a gateway call requiring `X-Argo-Principal: human:<email>`. The bypass isn't wired through to this step.
- **F5b**: Even if demo bypass were intended to fail closed here, the error copy leaks internal protocol detail (`X-Argo-Principal`, the literal header name). User-facing copy should be "We can't save this workspace in demo mode — sign in to continue" with a Sign-in CTA.

### F6 — P2 — Workflows page: "Loading workflows..." label persists after data loads

**Page**: `/workflows`.

**Evidence**: 7 workflow cards render correctly (post_listing, screen_candidate, approve_refund, qualify_lead, triage_ticket, morning_pipeline_digest, etc.) with proper author chips (BY MARIAH/PRIYA/JAMAL), version chips (v3/v5/v2/v4/v3), step-count, classification chips (APPROVAL REQUIRED / AUTONOMOUS / SALES / SUPPORT / GTM-ENGINEER / PEOPLE-OPS), state chips (HEADLESS · IDLE). The count pill correctly says "7 workflows". **But the text "Loading workflows..." remains visible directly above the card grid** — a stale loading state that should have been replaced by the rendered cards.

**Demo impact**: Mid. The cards render fine, but the persistent loading label undermines the surface's polish.

### F7 — P2 — Demo-mode "MOCK-OFFLINE" chip is visible inside Team header

**Page**: `/team`.

**Evidence**: Inside the heading "Team" the badge "MOCK-OFFLINE" renders as a sibling chip. This is fine for a developer working locally but should be gated behind a `?demo=1` query param or only render when the user is not signed in (which is detectable via the existing demo-mode top banner).

For investor recording: this chip will appear on screen for the entire team page. The "Demo mode — showing sample data" top banner already communicates the state; the duplicate "MOCK-OFFLINE" chip in the header reads as broken software.

### F8 — observation — PWA install prompt overlays interactive areas

**Pages**: `/team`, `/audit`, `/onboarding/workspace`, others. The "Install SKYCOT — Get it on your homescreen for one-tap access. [Not now] [Install]" toast sits bottom-right and overlaps the help bubble. Acceptable for the operator's daily use but in a screen-record context it pollutes every screenshot. Add `?hideInstallPrompt=1` or a session flag, or simply suppress the prompt for `localhost` origins.

---

## 6-MISSION-claim coverage (rollup from SPA observations)

| Claim | Status block | What the SPA actually shows | Verdict |
|---|---|---|---|
| 1 — Teammate not tool | 🟢 LIVE | `/team` 5-card C2 deck renders with personality (traits, level, today/tomorrow narrative). Strong surface. | 🟢 demo-walkable |
| 2 — Delegation is the unit | 🟢 LIVE | Cards show "Waiting on you" badges + "Open profile" links — but Approvals inbox (F1) is empty, so the delegation→approval loop cannot be observed. | 🟡 visible but broken loop |
| 3 — Platform builds workers | 🟢 LIVE | Onboarding wizard step 1 looks correct (F1 editorial-center). Step 2 fails (F5). | 🔴 demo-blocked |
| 4 — Skills shared + learned | 🟢 LIVE | `/skills` page shows hero correctly but "0 skills · 0 shown" + perpetual "Loading skills…" (F4). | 🔴 demo-blocked |
| 5 — Any-PC execution | 🟡 LIKELY-LIVE-pending-perms | Not directly observable from SPA in demo mode; depends on `/workflows/build` + computer-use substrate which need real macOS perms to demo. | 🟡 unchanged from prior |
| 6 — Multi-channel + multi-tenant | 🟢 LIVE | `/audit` lists rows (B3-Etherscan visual works) but drill-in 500s (F2) and rows are homogeneous demo-spam (F3). `/approvals` is empty (F1). | 🔴 demo-blocked |

**Net**: 3 of 6 claims are **demo-blocked** at the SPA tier even though the runtime-tier status block calls them 🟢 LIVE. The runtime claim and the SPA reality have drifted apart.

---

## Highest-leverage fix targets (root-cause clustering hint)

1. **Demo-mode bypass propagation** (closes F4, F5, plausibly F1 and F2): the SPA's demo-mode flag short-circuits to bundled sample data for **some** routes (team page, audit list, workflows list) but not others (skills, audit-action-detail, onboarding wizard continue, approvals inbox). The fix is a single demo-mode-aware gateway adapter that returns the right sample fixture for every endpoint the SPA calls.
2. **Sample data variety + cross-fixture consistency** (closes F1, F3): one fixture pack that produces (a) workers with N "Waiting on you" badges → (b) N approval rows in the approvals inbox → (c) audit rows with action kinds matching those approvals. End the per-page-fixture drift.
3. **Hire-flow / onboarding test infra collapse** (closes 8 playwright fails + F5): the step2/3/4 redirect failures + happy-path 30s timeouts cluster around a single broken state-machine wire. Pick this up as one brief.
4. **Workflow builder regression** (closes 13 playwright fails): builder save/publish/rollback/concurrent-edit/duplicate/delete all failing — one brief covers them as a single regression.
5. **Vocabulary "Schedule" rollout completion** (closes 6 playwright fails): operator-locked 2026-05-23 vocabulary memory says rename "Duty" → "Schedule" in UI copy on settings, workflows, cohorts. Not yet done on three pages. One mechanical brief.
6. **Dark-mode token coverage** (closes 4 playwright fails): /pricing, /compare, /docs, /for/marketplace white-on-white in dark mode. One CSS token sweep brief.

---

## Method

- Approach: live operator-driven Chrome walk via Claude-in-Chrome MCP at 1440×900 viewport. Pages visited in order: `/landing`, `/workers`→`/team` (redirect), `/approvals`, `/audit` (list + drill-in), `/skills`, `/workflows`, `/onboarding/welcome`, `/onboarding/workspace`. Plus the full playwright e2e suite (38 spec files) run in chromium-default project under the local lane.
- Read-only against codebase. No edits to any source file outside this report directory.
- Findings reported with file:line citations where applicable; primary evidence is screenshots taken during the walk + the playwright `run.log` in this directory.
