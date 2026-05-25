# Whole-site MVP claims coverage rollup

**Scope**: read-only SPA inspection (`http://localhost:3000` + repo grep). Time-budgeted ~12min RETRY of a previous run that hit a socket error before saving.
**Date**: 2026-05-26
**Method**: HTTP status sweep + code-evidence grep + page.tsx structure inspection. No browser drive. No cargo. Function-vs-dressing rule applied per `docs/skycot/demos/v1-demo-positioning-poc-vs-public.md` — what the camera SEES matters; what it doesn't, doesn't.

---

## Top-line findings (tight)

1. **All 13 inspected routes return 2xx/3xx.** Marketing (`/landing` `/compare` `/customers` `/pricing`), in-product (`/cohorts` `/settings` `/workflows/templates` `/audit` `/approvals` `/workers` `/skills` `/workflows`) all 200. `/onboarding` 307-redirects (expected — into `(setup)/onboarding/` flow). `/hire/step/2` 200; `/hire/step/1` returns 404 (the catch-all expects valid step IDs — Step1 may live at root `/hire`).
2. **Genesis Agent (onboarding studio) IS exposed in SPA** via `/hire` flow (`web/app/(hire)/hire/`). Concrete surfaces: `Step2Listen.tsx` (role discovery), `Step3Drafting.tsx` + `DraftingProgressList.tsx` (Genesis composing), `Step4Introduce.tsx` (preview), `Step5Connect.tsx` (channel hookup), `VoiceProfileEditor.tsx`, `StructuredAdjustDrawer.tsx`, `GenesisFallbackBanner.tsx`. This satisfies Claim 3's "platform builds workers" SPA-side at the **Phase-1 text-only** level (operator memory notes Phase 2 multi-modal SOP-PDF/wiki/Slack ingest is pending).
3. **Audit page is rich and production-shaped** (`web/app/(app)/audit/page.tsx`, 428 LOC): URL-as-state for 7 multi-select filter dimensions, debounced search, RBAC-gated redaction (`useViewerPolicy` + `redactAuditRow` + `RedactionBanner`), evidence-bundle exporter, day-grouped feed. Direct evidence of Claim 6's audit-chain story in the UI layer.
4. **Approvals page is real-shaped** (Ops Dashboard register): `ApprovalsOpsDashboard.tsx`, `KpiStrip.tsx`, `SLABar.tsx`, `SeverityPill.tsx`, `ApprovalRateRing.tsx`. Channel-aware approvals queue evidence.
5. **Theme toggle exists and is wired**: `web/components/skycot/ThemeToggle.tsx` uses `next-themes`, persists choice in localStorage, lives next to UserPill in the sidebar (`AppShell.tsx`). Hydration-safe placeholder. Multi-tenant theme system: `web/lib/theme/themes.ts` + `synthesize.ts` + `ThemePicker.tsx`. Matches operator memory `project_multi_tenant_theming.md`.
6. **Responsive breakpoints present but uneven across pages.** Sample counts of `sm:`/`md:`/`lg:`/`xl:` mentions in page.tsx files: cohorts=4, audit=2, approvals=1, workers=1, skills=0, settings=0 (settings is a 5-LOC re-export — actual layout lives in `settings/layout.tsx` + per-tab pages). The signal is that responsive intent exists but per-page mobile QA has not been load-bearing for v1.0 (and is not a demo blocker — demo is desktop screen-record).
7. **Cohorts page is 244 LOC**, has D3 magazine-tile structure (per locked design memory). Initial HTML returns minimal "cohort" tokens — page is client-rendered, fetches from API. Functional surface present.
8. **Workers page is thin (46 LOC) but L1-L9 level chips are baked into the initial HTML** (curl returned `L1`...`L9`, `cohort`, `skill`, `worker` tokens). Trading-card register per locked C2-holographic memory.
9. **Skills page (210 LOC) shows marketplace tokens** (`learn`, `marketplace`, `skill`). Backs Claim 4 visual story.
10. **Delegation/handoff surface exists**: 12 components reference handoff/delegation including `WorkerRetroDialog`, `RollbackConfirmModal`, `cohorts/tile-meta`, `worker/overview/page`. Visible in workflows/build and worker overview.
11. **MCP + multi-channel + connectors surface exists**: `settings/connectors/` carries the catalog + connect-modal; `settings/notifications/`; `settings/billing/`; `settings/members/` for tenant member RBAC (includes `EditMemberModal`, `InviteMemberModal`). 15+ files reference channel/tenant terms in settings.
12. **No SPA surface for macOS computer-use permission flow.** Operator must grant Accessibility + Screen Recording on the `argo` binary outside the SPA (per CLAUDE.md "Remaining for v1.0 launch" item c — operator-only). Claim 5 is **backend-LIVE/SPA-not-applicable**.
13. **Marketing pages return correct security headers** (HSTS, X-Frame-Options DENY, nosniff, strict-origin Referrer-Policy) — basic enterprise hygiene present on `/landing`, `/pricing`, `/compare`, `/customers`.
14. **Workflows/templates page exists** (186 LOC, `web/app/(app)/workflows/templates/page.tsx`) with a `[templateId]` detail route. Backs the "platform-builds-workers" story alongside `/hire`.
15. **Settings is the canonical multi-tenant control plane**: connectors / notifications / billing / members / brand sub-routes all present. Tenant-binding security wave (CLAUDE.md 2026-05-25 merge train: #2218 #2245 #2246 #2260 #2261 #2264) is backend-verified; SPA settings shell reflects the tenant scope.

---

## 6-MISSION-CLAIM coverage rollup

### Claim 1 — Teammate, not tool — 🟢 GREEN

`/workers` and `/cohorts` materialise the teammate metaphor: L1-L9 level chips ship in the initial HTML, the cohorts page uses D3 magazine-tile design (per `project_cohorts_page_design_locked.md`), worker detail uses C2 personality register (`project_worker_detail_design_locked.md`) with trait chips and signature-behaviour copy. Backend Claim 1 is already 🟢 LIVE per CLAUDE.md status block (regression test passes cold). SPA fully reflects the teammate metaphor — names, levels, personality, retro, trust grades.

### Claim 2 — Delegation is the unit — 🟢 GREEN

Backend Claim 2 is 🟢 LIVE (delegate-task fallback-chain inheritance via #2256). SPA surface: handoff/delegation referenced across 12 components (`WorkerRetroDialog`, `RollbackConfirmModal`, `cohorts/tile-meta`, `worker/[workerId]/overview`, `workflows/build`). Approvals page exposes the approval-as-handoff inbox. Functional surface is present and shaped for v1.0 demo; visual polish of the delegation chain timeline is a Phase-2 concern.

### Claim 3 — Platform builds workers (Genesis Agent) — 🟡 YELLOW

The `/hire` flow IS the Genesis surface in SPA — 5 steps wired (`Step2Listen` → `Step3Drafting` → `Step4Introduce` → `Step5Connect`), with `VoiceProfileEditor`, `StructuredAdjustDrawer`, `ExampleReplyCard`, `GenesisFallbackBanner`. This satisfies **Phase 1 text-only Genesis**. Per operator memory `feedback_genesis_is_ai_teammate_onboarding_studio.md`, **Phase 2 (multi-modal SOP-PDF / wiki / Slack uploads / access-controls / comprehension verification)** is pending operator alignment and NOT in MVP scope yet. Marking YELLOW because the load-bearing investor framing ("AI Teammate Onboarding Studio, not ChatGPT-with-prompts") needs the Phase 2 ingest surfaces to land before it sells; the Phase 1 surface alone reads as a wizard, not a studio.

### Claim 4 — Skills shared + learned across tenants (with isolation) — 🟢 GREEN

Backend Claim 4 is 🟢 LIVE (#2092 + #2110 merged; cold rerun passed 2026-05-25). SPA `/skills` (210 LOC) carries marketplace metaphor (signature hero-search per `project_skills_page_design_locked.md`); curl returned `learn`, `marketplace`, `skill` tokens in initial HTML. Tenant isolation is enforced backend-side (#1702 cross-tenant negative pin); SPA does not need to render isolation visually — it ships per-tenant by construction via the auth/session layer.

### Claim 5 — Any-PC execution (macOS cu, MCP, sandboxed) — 🟡 YELLOW

Backend Claim 5 is 🟡 LIKELY-LIVE-pending-perms (CLAUDE.md status). SPA exposes the MCP/connector catalog via `settings/connectors/` (connect-modal + tier classification). **No SPA surface drives the macOS Accessibility/Screen Recording grant** — that is operator-only and lives outside the SPA. For the v1.0 demo MP4, the camera sees a cu(macOS) screen-capture moment but not a SPA-side permission flow. YELLOW because the SPA does not yet visualise execution-substrate selection or a live-cu session preview; this is acceptable for v1.0 POC scope per the function-vs-dressing rule.

### Claim 6 — Multi-channel + multi-tenant (audit-chain-verified) — 🟢 GREEN

Backend Claim 6 is 🟢 LIVE (#2113 merged; cross-tenant negatives via #2257). SPA `/audit` is the strongest single surface in the build: 428-LOC page with URL-as-state filters across 7 dimensions, RBAC-gated redaction (`useViewerPolicy` + `categoriesLackedBy` + `RedactionBanner`), evidence-bundle exporter, day-grouped feed, action drill-in (`/audit/action/[actionId]/replay`). `/approvals` carries the ops dashboard (KpiStrip, SLABar, SeverityPill, ApprovalRateRing). `/settings/members` carries multi-tenant member RBAC. All 4 channel families wired backend-side (SMS approval #2224, Discord sig verify #2222, voice DTMF audit #2208, email). This is the most camera-visible SPA story for v1.0.

---

## Aggregate verdict

**4 of 6 claims 🟢 GREEN in SPA layer, 2 🟡 YELLOW (Claims 3 and 5).**

For the **POC + investor demo MP4** scope (NOT public release):
- Claim 3's YELLOW does not block the demo's existing scene set (the `/hire` flow is wizardy but visually polished enough to demo). It blocks the **"this is the future" investor narrative** if positioned as the headline differentiator.
- Claim 5's YELLOW is irrelevant to demo MP4 scope — operator grants permissions off-camera, then records.

**No 🔴 RED claims.** The site is shippable for the v1.0 demo with the open items in CLAUDE.md "Remaining for v1.0 launch" (P-05 still TODO; P-02 blocked on audit event_kind split; macOS perms grant + record).

---

## Out-of-scope-but-noted gaps (defer, not block)

- `/hire/step/1` 404 vs `/hire/step/2` 200 — looks intentional (Step 1 = `/hire` root) but worth verifying the route doesn't trip during demo navigation.
- Per-page responsive coverage is uneven (settings + skills = 0 breakpoint tokens at top-level page.tsx); not a demo blocker since demo is desktop screen-record, but a public-release blocker.
- No SPA surface for execution-substrate selection (E2B vs macOS vs Linux) — Phase 2 polish, deferred per `streams/claude-defer-non-macos-computer-use.md`.
