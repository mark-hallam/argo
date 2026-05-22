# SKYCOT — comprehensive user-journey map (living document)

**Date authored**: 2026-05-23 (evaluator round 6)
**Status**: living — next eval rounds extend and tick rows here.

Each journey is structured as:

- **Journey ID** — `j-<area>-<n>`
- **Persona** — who's doing it
- **Pre-state** — what must be true before the journey starts
- **Steps** — ordered atomic UI interactions
- **Pass criteria** — what makes the journey "passing"
- **Anti-theater check** — how to prove the steps aren't faked (real DB row, real network call, real audit event)
- **Status** — `🟢 PASS · 🔴 FAIL · 🟡 PARTIAL · ⏳ BLOCKED · ⚪ NOT-RUN`

This document is the COMPLETE testable surface. The evaluator runs as many journeys as it can per session and marks each row. Over time the doc trends to "🟢 PASS" everywhere.

---

## Persona A — First-time visitor (prospect)

### j-mkt-1 · Land on /landing
- **Pre-state**: clean browser, no session.
- **Steps**: navigate to `http://localhost:3000/landing`.
- **Pass**: page returns HTTP 200, renders the brand-loud hero ("Don't buy another tool. Hire a teammate."), shows 4 worker bubbles, "Hire your first worker" CTA visible.
- **Anti-theater**: screenshot diff against `docs/skycot/design-explorations/2026-05-22/landing/` lock; no Turbopack dev-error overlay.

### j-mkt-2 · Browse /compare
- **Steps**: nav to `/compare`.
- **Pass**: 13-row comparison matrix renders (SKYCOT vs Zapier, n8n, Crew.ai, Lindy), competitor logos load from `web/public/logos/`.

### j-mkt-3 · Browse /pricing
- **Steps**: nav to `/pricing`.
- **Pass**: pricing tiers render with monthly + annual toggle.

### j-mkt-4 · Browse /customers
- **Steps**: nav to `/customers`.
- **Pass**: customer logos load, case-study cards render.

### j-mkt-5 · Read /docs
- **Steps**: nav to `/docs`.
- **Pass**: docs index renders.

### j-mkt-6 · Read /changelog
- **Steps**: nav to `/changelog`.
- **Pass**: chronological list renders.

### j-mkt-7..10 · Legal pages (/security /privacy /terms /sla)
- **Steps**: nav to each.
- **Pass**: each renders without 404 or 500.

### j-mkt-11 · /help center
- **Steps**: nav to `/help`.
- **Pass**: renders.

### j-mkt-12..17 · /for/<segment> persona pages
- Segments: finance, marketing, ops, recruiting, sales, support.
- **Pass**: each renders with persona-specific copy + CTA.

### j-mkt-18 · Dark-mode toggle on all marketing routes
- **Steps**: open each route, set `prefers-color-scheme: dark` via Chrome devtools or CSS class.
- **Pass**: no white-on-white text, no missing background tokens, brand gradients still visible.

### j-mkt-19 · Mobile viewport on marketing
- **Steps**: resize to 375×667 (iPhone SE) and 768×1024 (iPad).
- **Pass**: layout adapts, no horizontal scroll, hero hamburger nav appears below 900px (per `project_personal_site_design_locked_A1_whisper.md`).

### j-mkt-20 · Click "Hire your first worker" CTA from /landing
- **Steps**: click CTA on /landing.
- **Pass**: lands on `/auth/signup`.

---

## Persona B — Signup → first-hire (5-minute new-user activation)

### j-auth-1 · Visit /auth/signup
- **Steps**: nav to `/auth/signup`.
- **Pass**: magic-link signup form visible, no passwords, brand-medium register.

### j-auth-2 · Submit magic-link signup
- **Steps**: type `demo@skycot.com`, click submit.
- **Pass**: confirmation message ("we sent you a link"), no error toast.
- **Anti-theater**: `POST /api/v1/auth/signup` network call fires; gateway responds 200.

### j-auth-3 · Magic-link verify
- **Steps**: simulate `/auth/verify?token=<jwt>`.
- **Pass**: session cookie set (`__Host-session`), redirect to `/onboarding` or `/`.
- **Anti-theater**: cookie present, JWT contains `tnt` + `role` claims.

### j-auth-4 · /auth/signin (returning user)
- **Steps**: nav to `/auth/signin` (no session).
- **Pass**: magic-link signin form, brand-medium.

### j-auth-5 · Logout
- **Pre-state**: signed in.
- **Steps**: click profile menu → Sign out.
- **Pass**: `POST /api/v1/auth/logout` fires, redirect to `/auth/signin`, cookie cleared.
- **Anti-theater**: cookie revoked in the revoked-tokens table (per #1968).

### j-onb-1 · Onboarding wizard step 1 (welcome)
- **Steps**: nav to `/onboarding` after signup.
- **Pass**: F1 editorial-center hero "Welcome" renders (per `project_onboarding_design_locked.md`).
- **Note**: as of 2026-05-22 audit, /onboarding returns 404 — F1 not yet adopted.

### j-onb-2..5 · Onboarding steps 2-5 (workspace / provider / vertical / hire)
- **Steps**: progress through each.
- **Pass**: each step persists state, back button works, validation gates.

### j-onb-6 · Onboarding completes → first worker created → land on Approvals
- **Anti-theater**: 1 row in `worker` table with tenant matching signup; audit row kind=`worker.hired`.

---

## Persona C — Hire wizard (in-app, not via onboarding)

### j-hire-1 · /hire landing (step 1)
- **Steps**: nav to `/hire`.
- **Pass**: editorial-center "Tell us what you need" hero.

### j-hire-2..5 · Hire wizard steps 2-5
- Step 2 (Listen): operator describes role free-form.
- Step 3 (Drafting): Genesis returns a WorkerDraft.
- Step 4 (Introduce): operator reviews + edits draft.
- Step 5 (Connect): channels + duties confirmation.
- **Anti-theater**: `POST /api/v1/genesis/draft-worker` returns real persona (NOT 5 Mariah clones — see round 1 finding).

### j-hire-6 · Genesis falls to stub → loud failure banner (per #2002)
- **Steps**: Step 3 with `?force_genesis_500=1` query.
- **Pass**: loud red banner appears, NOT a silent stub fallback in strict-mode.

### j-hire-7 · Idempotency-key duplicate submit
- **Steps**: submit step 5 twice with the same key.
- **Pass**: HTTP 200 second response with `"idempotent": true`, same worker_id.
- **Anti-theater check verified in round-1 cargo test** — this is the SPA-driven re-verification.

### j-hire-8 · 5 distinct workers, anti-Mariah-clone
- **Steps**: complete the wizard 5 times with 5 different problem statements.
- **Pass**: 5 distinct names, 5 distinct charters, 5 distinct verticals.
- **Anti-theater**: query `GET /api/v1/team` returns the 5 new workers with the 5 names submitted.

### j-hire-9 · Celebration banner after hire
- **Pre-state**: just hired a worker.
- **Steps**: redirect from `/hire/done` to `/`.
- **Pass**: celebration banner visible with worker name, dismissable.

---

## Persona D — Operator daily journey

### j-app-1 · Approvals inbox (/)
- **Pass**: E-ops-dashboard register: 5-KPI strip (PENDING / RESOLVED-WK / AVG RESPONSE / $ AT RISK / APPROVAL RATE), 7-day donut, empty state OR pending-approval cards.
- **Anti-theater**: KPIs come from `GET /api/v1/approvals` / `dashboards/error-rate` / `dashboards/sla`, not hardcoded.

### j-app-2 · Drill into one approval
- **Steps**: click first approval card.
- **Pass**: detail panel slides in, YES/NO branches visible, audit-trail visible.

### j-app-3 · Approve one
- **Steps**: click Approve.
- **Pass**: card disappears from queue, toast success, audit row appended.
- **Anti-theater**: `POST /api/v1/approvals/<id>/approve` fires real call; row in `audit_record` with kind=`ApprovalGranted`.

### j-app-4 · Decline one with reason
- **Pass**: reason captured in audit payload.

### j-app-5 · Filter by worker
- **Steps**: click filter chip, pick a worker.
- **Pass**: only that worker's approvals visible, chip rail shows active filters.

### j-app-6 · Search approvals
- **Steps**: type in search box.
- **Pass**: client-side debounce, results filter without page reload.

### j-app-7 · Multi-dim filter popover
- **Steps**: open popover, select 2 dims.
- **Pass**: chip rail reflects both, clear-all works.

### j-app-8 · Decider-role chip on timeline (per #2001)
- **Pass**: each approval audit-row shows the decider's role (owner / admin / user).

### j-app-9 · WS approval.expired live-removal
- **Pre-state**: 1 approval pending.
- **Steps**: emit `approval.expired` WS event for that id.
- **Pass**: card surgically removed without full re-fetch.

### j-team-1 · /team page
- **Pass**: brand-loud hero "X teammates, one team", grid of worker cards with name + persona + status + KPI row.

### j-team-2 · Filter team by status
- **Steps**: click filter chip (working/idle/blocked/paused).
- **Pass**: chips toggle, results filter.

### j-team-3 · Search team by name
- **Pass**: client-side text-match.

### j-team-4 · Click into a worker
- **Pass**: navigates to `/workers/<id>/overview`.

### j-wkr-1 · Worker detail — Overview tab
- **Pass**: personality card (LinkedIn-style — name, traits, great-at, escalate-when, signature behaviors per `project_worker_detail_design_locked.md`), responsibilities, today's KPIs.

### j-wkr-2 · Worker detail — Activity tab
- **Pass**: per-worker audit timeline (per #J9), filter by date range, drill-in to single event.

### j-wkr-3 · Worker detail — Memory tab
- **Pass**: facts list + voice profile + namespace id.

### j-wkr-4 · Worker detail — Skills tab
- **Pass**: 3 sections (active / learning / library) + per-skill history endpoint.

### j-wkr-5 · Worker detail — Settings tab
- **Pass**: channels / quiet hours / budget / autonomy + operator-only advanced block.

### j-wkr-6 · Worker detail — Trust grade
- **Pass**: trust grade chip + recompute affordance + disclaimer (per #1979).

### j-wkr-7 · Worker detail — Policies
- **Pass**: per-worker auto-approval policies index.

### j-wkr-8 · Worker detail — Retros
- **Pass**: per-worker retrospective entries.

### j-wkr-9 · Edit charter
- **Steps**: click Edit on charter.
- **Pass**: textarea opens, save persists.

### j-wkr-10 · Worker freeze
- **Pass**: freeze button → confirm → worker shows "Frozen" status, duties pause.

### j-wkr-11 · Cross-tenant probe (worker exists in another tenant)
- **Steps**: nav to `/workers/<known-other-tenant-id>`.
- **Pass**: 404 — NOT a 403 distinguishable from 404 (per round-4 P2 finding).

---

## Persona E — Skills marketplace

### j-skl-1 · /skills page
- **Pass**: brand-loud marketplace + hero-search pattern (28px gradient icon + 18px input + ⌘K + count pill + try-pills per `project_skills_page_design_locked.md`).

### j-skl-2 · Hero search
- **Steps**: type in hero search.
- **Pass**: results filter as you type.

### j-skl-3 · Try-pill suggested searches
- **Steps**: click a try-pill.
- **Pass**: search auto-fills + results update.

### j-skl-4 · Open skill detail
- **Pass**: skill name, version, provenance, trust grade chip, workers using.

### j-skl-5 · Skill IR canvas viewer
- **Pass**: visual graph of the skill's IR renders (per L4-gw `routes_skill_ir`).

### j-skl-6 · Skill quality metrics
- **Pass**: quality / canary / A-B / dreams sub-pages.

### j-skl-7 · Skill rejection flow
- **Pre-state**: 1 pending skill.
- **Steps**: human reviewer rejects.
- **Pass**: SkillRejected audit row + skill removed from offer list.

### j-skl-8 · Cross-tenant skill leak negative (cargo-side already verified in round 2, this is SPA-side)
- **Pre-state**: two tenants, each with one skill.
- **Pass**: tenant_b doesn't see tenant_a's skill.

---

## Persona F — Audit chain

### j-aud-1 · /audit page
- **Pass**: brand-restrained register (Etherscan-style, Inter throughout, per `project_audit_page_design_locked.md`), 5-KPI strip + chronological event list + 7-filter popover.

### j-aud-2 · Filter by worker / channel / action_type / status / approver / cost_band / classification
- **Pass**: each dim filter toggles + chip rail.

### j-aud-3 · Date range presets (1h / today / yesterday / 7d / 30d / qtd / lq)
- **Pass**: range chip updates results.

### j-aud-4 · Substring search
- **Pass**: `?q=` filters.

### j-aud-5 · Drill into one event
- **Pass**: pop-out modal with full payload, hash chain visible, screenshots if any.

### j-aud-6 · Cross-system link
- **Steps**: click jump link on detail.
- **Pass**: opens new tab to the foreign system.

### j-aud-7 · Hash-chain integrity
- **Pass**: each row's `self_hash` is the SHA256 of the prior row's hash + this row's content.

### j-aud-8 · Cross-tenant audit_detail (verified cargo-side round 4)
- **Pre-state**: known audit_id from tenant_a.
- **Steps**: GET as tenant_b.
- **Pass**: 404.

### j-aud-9 · Audit export bundle
- **Steps**: click Export, pick window.
- **Pass**: bundle downloads, contents are tenant-scoped (per #2051).

### j-aud-10 · Platform events visible (P0-D from round 1 — currently OPEN)
- **Pre-state**: hire 1 worker.
- **Pass**: WorkerHired event visible in audit feed within 1s.

---

## Persona G — Workflow builder

### j-flow-1 · /workflows catalog
- **Pass**: brand-loud marketplace with Audiowide "How your teammates actually think." hero + hero-search + try-pills + workflow cards.

### j-flow-2 · Search workflows
- **Pass**: client-side filter.

### j-flow-3 · Browse 50+ templates
- **Pass**: templates view renders.

### j-flow-4 · Click into workflow detail
- **Pass**: detail page renders steps + version + author + Open Builder CTA.

### j-flow-5 · Open builder canvas (⏳ BLOCKED-BY-P0-E1)
- **Currently fails** — layout.ts naming bug → blank page.
- **Pass after fix**: React Flow canvas with nodes, edges, palette, toolbar.

### j-flow-6 · Pan + zoom + mini-map (BLOCKED-BY-P0-E1)
- **Pass**: smooth interactions.

### j-flow-7 · Drag a node from palette (BLOCKED)
- **Pass**: node appears at drop location.

### j-flow-8 · Connect two nodes (BLOCKED)
- **Pass**: edge appears between them.

### j-flow-9 · Right-click node → context menu (BLOCKED)
- **Pass**: Duplicate / Disconnect / Delete options.

### j-flow-10 · Delete a step (BLOCKED)
- **Pass**: node + incident edges removed.

### j-flow-11 · Backspace as alias for Delete (BLOCKED)
- **Pass**: same as above.

### j-flow-12 · Trigger node refuses deletion (BLOCKED)
- **Pass**: Delete + Backspace are no-ops on trigger.

### j-flow-13 · Save draft (BLOCKED)
- **Pass**: `POST /api/v1/verticals/<v>/workflows/<n>` real call, version toast "Saved (v3)".

### j-flow-14 · Publish (BLOCKED)
- **Pass**: `PUT /api/v1/verticals/<v>/workflows/<n>/head`, version flip, redirect to viewer.

### j-flow-15 · Concurrent edit conflict (BLOCKED)
- **Pre-state**: two tabs at same workflow.
- **Steps**: edit + save in tab A, then in tab B.
- **Pass**: tab B sees 409 with conflict banner + refresh/diff CTAs.

### j-flow-16 · Rollback to previous version (BLOCKED)
- **Pass**: confirm modal → success toast → viewer reflects rolled-back version → audit row kind=`WorkflowRolledBack`.

### j-flow-17 · Test run (BLOCKED)
- **Pass**: NDJSON stream of step events.

### j-flow-18 · Sub-workflow node (BLOCKED)
- **Pass**: sub-workflow + Add Step.

### j-flow-19 · Sticky notes (BLOCKED)
- **Pass**: per-step input-data pinning.

### j-flow-20 · Mobile gate (BLOCKED — at canvas, not at detail)
- **Steps**: resize viewport <1024px on `/workflows/<v>/<n>/build`.
- **Pass**: MobileBuilderGate shows, full canvas hidden.

### j-flow-21 · Test run history panel (per #2016)
- **Pass**: panel lists past runs.

### j-flow-22 · NL workflow editing (Propose new + Propose-toml)
- **Steps**: type "add a step that checks ICP" → Propose.
- **Pass**: diff preview, approve to ship.

---

## Persona H — Cohorts

### j-coh-1 · /cohorts page
- **Pass**: D3 magazine tiles + 7 dept tabs + lead/stacked/compact (per `project_cohorts_page_design_locked.md`).

### j-coh-2 · Switch dept tabs
- **Pass**: tiles re-flow per dept.

### j-coh-3 · Cohort detail
- **Pass**: drawer with members + activity + invite CTA.

### j-coh-4 · Cohort skill assignment
- **Pass**: skills attached to cohort visible.

---

## Persona I — Settings

### j-set-1 · /settings landing
- **Pass**: top-tabs + sub-tabs + 2-col content with aside cards (per `project_settings_page_design_locked.md`).

### j-set-2 · Members tab
- **Pass**: list + invitations + invite CTA.

### j-set-3 · Billing tab
- **Pass**: tier + monthly usage view.

### j-set-4 · Knowledge tab (BLOCKED until mission-workplace-knowledge-gateway MERGES)
- **Pass**: markdown editor + version history + 5-category nav.

### j-set-5 · Tokens / security
- **Pass**: API token list + create + revoke.

### j-set-6 · Workspace
- **Pass**: workspace settings (tenant name, branding).

### j-set-7 · Danger zone
- **Pass**: right-aligned in tab row; destructive actions gated.

---

## Persona J — Orchestration

### j-orc-1 · /orchestration page (if mounted)
- **Pass**: queue of runs, filter by worker.

### j-orc-2 · Filter orchestration by worker
- **Pass**: client-side filter.

---

## Persona K — Edge cases + dev hygiene

### j-edge-1 · /not-found route
- **Steps**: nav to `/does-not-exist`.
- **Pass**: branded 404 page (not Next.js default).

### j-edge-2 · /maintenance
- **Pass**: branded maintenance message.

### j-edge-3 · /offline
- **Pass**: branded offline message + retry button.

### j-edge-4 · Console-error sweep
- **Steps**: visit every route, collect console errors.
- **Pass**: ZERO console errors (or only known-benign chrome-extension async errors).

### j-edge-5 · Network panel — no 4xx/5xx unexpected
- **Steps**: visit every route, watch network.
- **Pass**: no unexpected 4xx/5xx; expected ones (mock-fallback 404s) are recognised.

### j-edge-6 · Dark-mode token coverage
- **Steps**: every page in dark mode.
- **Pass**: no white-on-white text per `dark-mode-token-coverage.spec.ts`.

### j-edge-7 · Keyboard-only navigation
- **Steps**: Tab through every interactive element.
- **Pass**: focus visible, no traps, Enter activates.

### j-edge-8 · Multi-tab routing
- **Steps**: ⌘+click links.
- **Pass**: opens in new tab, state isolated.

### j-edge-9 · Page-load performance
- **Pass**: each page first contentful paint < 2s on local dev.

---

## Persona L — Approvals via inbound channels (mission claim 6)

### j-ch-1 · Email approval request
- **Pre-state**: a worker has email channel configured.
- **Steps**: worker triggers action gated by approval.
- **Pass**: email sent with HMAC-signed approval token.

### j-ch-2 · Email approval reply (inbound)
- **Steps**: reply YES to the approval email.
- **Pass**: action proceeds, audit row appended.

### j-ch-3..N · Slack / SMS / Push / Voice approvals
- Same shape as email, per `NotifyChannel` impls.

---

## Tally template (filled in per session)

| Phase | Total | 🟢 | 🟡 | 🔴 | ⏳ Blocked | ⚪ Not-run |
|---|---|---|---|---|---|---|
| Marketing (A) | 20 | _ | _ | _ | _ | _ |
| Auth/Onboarding (B) | 11 | _ | _ | _ | _ | _ |
| Hire wizard (C) | 9 | _ | _ | _ | _ | _ |
| Operator daily (D) | 20 | _ | _ | _ | _ | _ |
| Skills (E) | 8 | _ | _ | _ | _ | _ |
| Audit (F) | 10 | _ | _ | _ | _ | _ |
| Workflow builder (G) | 22 | _ | _ | _ | _ | _ |
| Cohorts (H) | 4 | _ | _ | _ | _ | _ |
| Settings (I) | 7 | _ | _ | _ | _ | _ |
| Orchestration (J) | 2 | _ | _ | _ | _ | _ |
| Edge cases (K) | 9 | _ | _ | _ | _ | _ |
| Channels (L) | N | _ | _ | _ | _ | _ |
| **Total** | **~135** | | | | | |

Each session updates this tally + appends a per-journey results table to `journey-results-<session-utc>.md`.
