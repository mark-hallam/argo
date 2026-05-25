# Product Roadmap — post-MVP path to every mission claim

**Authored against**: origin/main `70e4b020` on `2026-05-11`.
**Current-state refresh**: `2026-05-24`.
**Status**: roadmap index and historical track map. The live queue,
current claim state, and operator-locked frontend instructions live in
[`BUILD_STATUS.md`](BUILD_STATUS.md). Supersedes the old v1.4-and-
beyond sections of `BUILD_PLAN_V11.md` only as product-surface context;
do not treat the 2026-05-11 status text below as the current brief queue.

> Fresh web/UX sessions: read the [`BUILD_STATUS.md`](BUILD_STATUS.md)
> header first. If the ledger header or row text says frontend work is
> operator-locked, do not brainstorm new screens from this roadmap; pick
> up the named `web-*`, `web-port-*`, or polish stream row and follow its
> current brief.

## What this document is

The path from v1 MVP (mock-mode smoke green, all mission backend
wires merged, no polished UX) to **every mission claim landed in
a felt user experience.** Captures seven sequenced tracks (A-G),
their dependencies, their historical status, and which mission claim
each was meant to close. The tracks are still useful for orientation,
but the repo has since moved through design-system, demo SWOT, and
web-port passes. Current implementation authority is the live ledger
row plus its stream brief.

Authored 2026-05-11 after coverage audit of
[BUILD_PLAN_V11.md](BUILD_PLAN_V11.md) + [MISSION_BACKLOG.md](MISSION_BACKLOG.md)
+ 567 stream-file inventory revealed: 4 of 14 product gaps not
covered by any existing brief; 9 partially covered. The frontend
program is structurally thin — `v1.4` in V11 was declared SHIPPED
against minimal TUI/HTML slices, not against a real product
surface.

## The seven tracks

| Track | Scope | Who | Status | Closes |
|---|---|---|---|---|
| A | UX research — personas + journey maps | operator + claude | **DONE** 2026-05-11 | design brief for B/C/D/E |
| B | Capability fill-ins (Windows native CU + real-LLM journey test) | swarm | **DONE** 2026-05-12 — both briefs MERGED (mission-cu-native-windows-poc #1332, mission-real-llm-operator-journey-test #1333) | mission claim 5 (Windows leg) |
| C | Design language — stack pick, brand tokens, component primitives | operator + focused threads | **CURRENT HANDOFF** 2026-05-24 — design-system foundation is in [web/](../../web/); remaining coverage is tracked by live rows such as `web-storybook-core-ui-coverage`, not by this historical table | foundation for D + E |
| D | Wireframes and locked visual directions | operator + claude, validated against product passes | **DONE / SUPERSEDED BY LOCKED MOCKUPS** 2026-05-24 — original wireframes remain context, but current web-port source material is under [design-explorations/2026-05-18](../skycot/design-explorations/2026-05-18/) and named in each ledger row | implementation brief for E |
| E | Frontend implementation and polish | swarm, after C+D land | **LIVE LEDGER** 2026-05-24 — post-25-audit and demo SWOT work is carried by `web-*`, `web-port-*`, and fix-request rows in [`BUILD_STATUS.md`](BUILD_STATUS.md); do not infer "not started" from the 2026-05-11 slice list | mission claims 1, 2, 3, 6 |
| F | Backend fill-ins for claim 4 — skill/marketplace/workplace-knowledge TODOs | swarm, parallel to E | **LIVE LEDGER / GAP-MANIFEST DRIVEN** 2026-05-24 — use live `mission-*` rows and `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`, not the old 14-row advisory snapshot | mission claim 4 |
| G | Mission-5 finish: Windows dispatch wiring + Wayland Linux + real-LLM-test CI integration | swarm, parallel to E | **IN PROGRESS** 2026-05-12 — 3 briefs authored in [streams/](streams/); advisory queued ([G+J](conductor-advisory-G-J.md)). Flips to DONE only after merges land. | mission claim 5 (full) |
| J | Gateway endpoints sibling to Track E — JSON APIs that the frontend briefs assume (replace mock fallbacks with real backends) | swarm, parallel to E | **IN PROGRESS** 2026-05-12 — all 18 backend briefs authored in [streams/](streams/); advisory queued ([G+J](conductor-advisory-G-J.md)). Flips to DONE only after merges land. | mission claims 1-6 backend completeness |

## Current Handoff — 2026-05-24

Use this section before interpreting any older track notes:

- [`BUILD_STATUS.md`](BUILD_STATUS.md) is the live queue. Its header
  carries current operator directives, including when design-language or
  demo-SWOT work is operator-locked.
- Current web/UX implementation is mostly expressed as `web-*`,
  `web-port-*`, and fix-request rows. Examples at this refresh include
  `web-port-approvals-e-ops-dashboard`, `web-skills-mobile-and-search-dark-mode-polish`,
  and `web-storybook-core-ui-coverage`.
- Operator-locked mockups and visual source material live under
  [design-explorations/2026-05-18](../skycot/design-explorations/2026-05-18/).
  A brief row may cite a narrower source path; if the cited path is stale,
  use the real on-disk design-explorations path and document the deviation.
- Demo SWOT and cold-walk follow-ups are first-class product work when they
  appear as live ledger rows. Treat the row status and stream brief as the
  operative source, not this roadmap's 2026-05-11 estimates.
- Backend product-surface gaps now flow from live `mission-*` stream rows
  and the mission-vs-code gap manifest. Old advisory files are historical
  unless a current ledger row explicitly points at one.

## Track A — UX research (DONE)

Deliverables under [../skycot/ux-research/](../skycot/ux-research/):

All 6 personas + 5 journey maps written in full as of 2026-05-11
(initial 3-primary-personas + 2-primary-journeys landed
earlier, remaining 3 personas + 3 journeys expanded later same
day).

Personas:
- `delegator.md` — primary daily user (SMB owner / dept head).
- `approver.md` — highest-frequency touchpoint (mobile, 10s decisions).
- `operator.md` — technical admin / B2B buyer (RBAC, compliance).
- `end-user.md` — external counterparty (candidate, buyer,
  customer, patient). The reputation we're defending.
- `auditor.md` — compliance / SOC2 / GDPR / HIPAA. Determines
  regulated-industry deployability.
- `worker.md` — the AI worker itself as a user of internal APIs.

Journeys:
- `journey-hire-a-teammate.md` — 10-minute conversational onboarding.
- `journey-approve-on-mobile.md` — 10s decision card across channels.
- `journey-delegate-ongoing-work.md` — recurring delegation, post-hire.
- `journey-audit-replay.md` — forensic + compliance flow.
- `journey-retrain-a-worker.md` — plain-English correction → durable change.
- Operator reference: [marketplace tag policy](../skycot/operator/marketplace-tag-policy.md).

Plus `README.md` — index, conventions, validation plan.

**Validation status**: all docs carry `(assumed — needs
validation)` tags. Per operator preference (memory
`feedback_build_then_validate`), interviews are deferred until
there's working software to test against — Track E's
approvals-inbox slice is the first such surface. The intent is
to update these docs in-place with what real-user feedback
reveals rather than gate further work on validation.

## Track B — capability fill-ins (DONE 2026-05-12)

Two briefs authored 2026-05-11, queued for swarm pickup:

- [streams/mission-cu-native-windows-poc.md](streams/mission-cu-native-windows-poc.md)
  — NEW crate `argo-executor-computer-use-windows`, PowerShell-
  built-in subprocess primitives, mirrors macOS #941 + Linux POC.
  Closes the third leg of mission claim 5. Operator update
  2026-05-14: Windows stays shipped but is DEFERRED-v1.x; macOS is
  the v1 primary demo path.
- [streams/mission-real-llm-operator-journey-test.md](streams/mission-real-llm-operator-journey-test.md)
  — NEW `scripts/smoke-skycot-real-llm.sh`, mock-mode-equivalent
  but against real provider. Coherence + cost + 90s budget
  assertions. Closes the verifiability gap for mission claim 1.

**Watch for**: Windows POC may sit TODO until a Windows worker
joins the swarm — the acceptance criterion requires a real Windows
session for the integration assertion. If it sits TODO > 1 week,
split into cross-platform compile-clean (any worker) + Windows
integration (Windows-only).

## Track C — design language (IN PROGRESS)

**Current handoff 2026-05-24:** the design-system substrate exists in
`web/`, and the active design-system work should be read from
[`BUILD_STATUS.md`](BUILD_STATUS.md). At this refresh the live queue
contains `web-storybook-core-ui-coverage`; claim that stream row rather
than opening an ad hoc Storybook or primitive sweep from this section.

**Status update 2026-05-11:** ADR locked
([streams/adr-frontend-design-language.md](streams/adr-frontend-design-language.md)),
`web/` Next.js 15 scaffold landed, all 8 primitives + the load-
bearing `ApprovalCard` shipped, multi-tenant theming layer
(SKYCOT light/dark + Acme tenant white-label demo + OKLCH palette
synthesizer for arbitrary brand hues) wired end-to-end. Production
build green (29.5KB demo page, 131KB First Load JS); typecheck
clean; 6/6 ApprovalCard unit tests passing.

**Historical "what's left" note:** Storybook, Playwright smoke, and
gateway wire-up were the 2026-05-11 next slices. Do not use this list
as an open queue; use the current ledger header and rows.

Single irreversible-decision phase. Owner-driven. Defaults:

- **Stack**: Next.js 14+ App Router + TypeScript + Tailwind CSS +
  shadcn/ui (Radix primitives). Hosted gateway-adjacent. Server
  components where they fit; client islands for the approval-card
  and live-update surfaces.
- **State**: Tanstack Query for server state; URL state for
  filters; minimal client store (Zustand or none) for UI-only.
- **Auth**: NextAuth with the existing `ARGO_GATEWAY_BEARER_TOKEN`
  precedent as the bridge. Per-Principal session derived from
  the gateway.
- **Brand tokens**: type scale, color palette, density,
  iconography, motion. Locked in Storybook + a single ADR.
- **Primitive components** (~5–8): button, card, avatar, status
  pill, approval card, side-nav drawer, toast, data table cell.

Deliverable: a Storybook + an ADR brief that captures the
locked decisions and lives at `docs/build-plan/streams/adr-
frontend-design-language.md`. After this lands, Track E can
parallelize cleanly.

Sizing: ~1 week of focused operator work plus one supporting
claude thread.

## Track D — wireframes (DONE 2026-05-11)

**Current handoff 2026-05-24:** the low-fi wireframes remain useful
research context. For current web-port work, prefer the operator-locked
mockups in [design-explorations/2026-05-18](../skycot/design-explorations/2026-05-18/)
and the exact source path named in the live row. Known directories
include `approvals-inbox`, `audit-page`, `cohorts-page`, `skills-page`,
`worker-detail`, `workers-page`, `settings-page`, `onboarding-wizard`,
and `auth-pages`.

Five surfaces, low-fi in Figma or paper, validated against the
Track A personas with real users before locking. Order matters —
each surface is a single Figma file linked from this section:

1. **Approvals inbox** (highest leverage; closes daily UX for
   claim 6; the surface every Approver lives in).
2. **Hire flow** (10-minute conversational onboarding;
   closes felt-experience of claim 3).
3. **Team page** (the "meet your AI teammates as faces" roster).
4. **Worker profile** (persona / role / KPIs / memory / skills;
   the deep view).
5. **Audit timeline** (forensic / compliance view; doubles for
   Auditor persona).

Sizing: ~1–2 weeks. Operator + claude in a focused thread, with
2–3 user-validation passes per surface.

## Track E — frontend implementation (IN PROGRESS — E1 closed, E3 closed, E2/E4/E5 in flight)

**Current handoff 2026-05-24:** this section is historical slice
planning. The active frontend queue is the live ledger's `web-*`,
`web-port-*`, and demo SWOT/fix-request rows. Examples at this refresh:
`web-port-approvals-e-ops-dashboard` is in review, `web-skills-mobile-and-search-dark-mode-polish`
is in review, and `web-storybook-core-ui-coverage` remains TODO. Web/UX
workers should check the ledger header before starting new frontend work
because current design-language and roadmap decisions may be explicitly
operator-driven.

Sliced thin by surface, in the order above. Each slice is
5–10 swarmable briefs once the C+D foundation is locked.
Approximate brief inventory per surface:

- **Approvals inbox** (~6 briefs): inbox list view, approval-card
  component, decision-submit + undo, per-channel parity test,
  filter UI, mobile-first responsive pass.
- **Hire flow** (~10 briefs): landing screen, Genesis-driven
  conversation surface, example-replies preview, hire-with-
  defaults submit path, OAuth connect surface, post-hire
  summary, Day-1 wrap email/notification, error recovery,
  edit-before-hire loop, analytics instrumentation.
- **Team page** (~5 briefs): roster grid, worker status pills,
  per-worker today's-headline, quick-actions menu, empty state.
- **Worker profile** (~8 briefs): persona header, charter editor,
  duties list, KPI charts, memory browser, skill list, activity
  feed, retrain entry-point.
- **Audit timeline** (~6 briefs): cross-worker timeline, drill-
  in view, replay launch, export-bundle UI, redaction layer,
  search filters.

Sizing: ~6–10 weeks of swarm work. Conductor manages dependency
ordering inside each slice; slices ship sequentially but briefs
within a slice parallelize.

## Track F — backend fill-ins for mission claim 4 (substantively MERGED 2026-05-12)

**Current handoff 2026-05-24:** this row list is historical. The
operative backend fill-in source is the live `mission-*` ledger queue
plus `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md`.
Current product-surface backend work includes claim-4 skill and
marketplace follow-ups, workplace-knowledge rows already shipped through
Wave A, and tenant-pinning rows such as `mission-skill-query-tenant-pinned`
when they appear in [`BUILD_STATUS.md`](BUILD_STATUS.md). Do not revive
an old row below unless the current ledger says it is TODO.

Historical six outstanding TODO mission rows that closed the
"skills shared and learned" claim:

- `mission-skill-publish-verified-ingest`
- `mission-skill-store-available-for-worker`
- `mission-skill-version-migration-path`
- `mission-skill-ratings-publisher`
- `mission-skill-min-trust-grade-column`
- `mission-skill-select-variant-runtime-wire`
- `mission-skill-session-boot-wire`

Plus marketplace-side:

- `mission-marketplace-browser-e2b-image`
- `mission-marketplace-oci-registry-scan`

Sizing: ~2–3 weeks of swarm work. Independent of Track E;
runnable in parallel.

## Track G — mission claim 5 finish (DONE 2026-05-12 — G1/G2/G3 MERGED)

Operator update 2026-05-14: v1 hardening targets
`argo-executor-computer-use-macos` as the primary computer-use path.
E2B, Linux, and Windows executors stay in the workspace and continue
to ship, but they are DEFERRED-v1.x and do not block the v1 demo.

Follow-ups after Track B lands:

- Windows dispatch wiring (`mission-cu-native-windows-executor-
  dispatch`, mirrors mac #955) — depends on Track B's POC merged.
- Wayland Linux CU primitives (`mission-cu-native-linux-wayland`,
  flagged TODO in the existing Linux POC brief).
- Real-LLM journey test → CI integration (deferred follow-up
  noted in the Track B brief).

Sizing: ~1 week of swarm work.

## Track J — gateway endpoints sibling to Track E (LIVE LEDGER)

**Current handoff 2026-05-24:** the 2026-05-12 G+J advisory is
historical. Backend endpoints that unblock product surfaces now appear
as named `mission-*`, `web-gateway-endpoints-*`, or tenant-bind rows in
[`BUILD_STATUS.md`](BUILD_STATUS.md), with stream files under
[`streams/`](streams/). For new product-surface backend fill-in work,
start from the live ledger row and the mission-vs-code gap manifest,
then verify the handler and client paths on disk before claiming.


## Track L — Skill Canvas (fine-tuner) (IN PROGRESS — phase 1 briefs landing 2026-05-14)

**Status update 2026-05-14:** Operator pulled Track L into v1.0 scope. L0 overview (`streams/L0-skill-canvas-track-overview.md`) is the authoritative spec. Phase 1 briefs (L1-L6 + L4-gw) authored this tick.

**What this delivers**: a **canvas as a fine-tuner, not a builder**. Workers learn through natural interaction; the canvas gives operators a structured view of what was learned — a graph to inspect, refine, version, and AB-promote.

**Phasing (per L0)**:
- Phase 1 (L1-L6, L4-gw): foundation + viewer + Ghostwriter. Read-only canvas demo + gen-from-prompt. ~3500 LOC.
- Phase 2 (L7-L10): editor + diff. Operator can refine + AB-promote. ~3000 LOC.
- Phase 3 (L11-L13): observation extraction. CU recording → skill graph. ~2800 LOC.
- Phase 4 (L14-L17): composition + library + e2e. ~2000 LOC.

Total: ~17 briefs, ~11,300 LOC.

**Design decisions locked in L0** (2026-05-12):
- Canvas renderer: React Flow (`@xyflow/react`).
- Sub-skill versioning: pinned by default; "follow latest" toggle per call site.
- Ghostwriter LLM: strongest available regardless of worker runtime provider.
- Canonical sub-skills (L15): `auth-flow`, `document-extraction`, `escalation-pattern`, `approval-with-timeout`, `csv-to-records`, `retry-with-backoff` — all wrap existing primitives.

## Track M — Enterprise Parity (closing UiPath gaps) (PLANNED — briefs being authored, gated on Track L Phase 1)

**Status update 2026-05-14:** Operator pulled Track M into v1.0 scope. M0 overview (`streams/M0-enterprise-parity-track-overview.md`) is the authoritative spec. Phase 1 briefs (M1-M8) to be authored next, after Track L Phase 1 lands.

**Sub-tracks (per M0)**:
- **M-Doc** — Document Understanding (Claude vision extractor + operator UI)
- **M-Orch** — Orchestration maturity (queue/SLA/retry surface UI)
- **M-Eval** — Regression eval (operator UI + self-healing)
- **M-Code** — Pro-code skill mode (TS skill editor)
- **M-Tmpl** — Vertical template packs

Total: ~20 briefs, ~9000 LOC. Runs PARALLEL to Track L Phases 3-5.


## Mission-claim coverage map

> **Refreshed 2026-05-24** — this table is orientation only. Mission
> definitions and LIVE criteria live in [`../skycot/MISSION.md`](../skycot/MISSION.md)
> and the claim dashboards under `docs/skycot/evals/scheduled/`.
> Product and backend work state lives in [`BUILD_STATUS.md`](BUILD_STATUS.md)
> rows and the mission-vs-code gap manifest.

| Claim | Tracks that close it | Status |
|---|---|---|
| 1. Teammate, not tool | A (research) + C/D/E product surface | Check live `web-*` rows plus claim-dashboard evidence; do not infer from old E3/E4 labels. |
| 2. Delegation is the unit | A + C/D/E hire/delegation surface | Check live hire/workflow rows and current demo-runbook gates. |
| 3. Platform builds workers | Backend + hire/workflow product surface | Check current `mission-*`, workflow, onboarding, and demo-SWOT rows. |
| 4. Skills shared and learned | F backend + Skills/workplace product surface | Check live skill/workplace rows and the mission-vs-code gap manifest. |
| 5. MCP + computer-use | B/G plus current macOS-first demo path | MISSION.md says macOS is v1 primary; Linux/Windows/E2B CU remain DEFERRED-v1.x unless a current row says otherwise. |
| 6. Multi-channel approval gates | Channel backend + approvals inbox surface | Check live channel/approval rows; `/approvals` design parity is now tracked by web-port rows such as `web-port-approvals-e-ops-dashboard`. |

## Sequencing rules

- **Ledger header wins.** Check it before starting new frontend work;
  it may mark design-language or roadmap decisions as operator-driven.
- **Do not improvise locked UI.** For web-port work, use the row's
  stream brief plus the on-disk design-explorations source.
- **Backend fill-ins are row-driven.** Use current `mission-*` and
  gap-manifest rows, not the old advisory snapshots in this roadmap.
- **Historical sequencing remains context.** A/C/D still explain why
  the product slices exist, but they are not a standalone task queue.

## Status tracking

The roadmap document is updated only for strategic drift. Per-brief
status lives in [BUILD_STATUS.md](BUILD_STATUS.md), and its status values
(`TODO`, `CLAIMED`, `IN_REVIEW`, `MERGED`, `BLOCKED`) are the source of
truth for worker action.

When a fresh session picks this up, the way to read it is:

1. Read the [`BUILD_STATUS.md`](BUILD_STATUS.md) header and the row you
   intend to claim.
2. Read the stream brief named by that row.
3. Use this roadmap only to understand which product track the row
   belongs to.
4. If this document conflicts with `MISSION.md`, the ledger header, or
   an operator-locked design brief, follow the current source and note
   the discrepancy in your PR.

## What this document REPLACES

For the product-surface phase (v1.4 onward), this supersedes:

- `BUILD_PLAN_V11.md` §"Phase v1.4 — Operator UX" — the bullet
  list there was declared SHIPPED against minimal TUI/HTML
  slices and does not capture the polished SPA program.
- `MISSION_BACKLOG.md` Section 5 entries #170–#175 — they remain
  ledger rows for the thin MVP UI slices already merged but do
  not constitute the operative product-surface plan.

The original v0.2 build plan and V11 backend phases (v1.1, v1.2,
v1.3, v2.0, v2.1, v2.2) remain authoritative for their scopes.

## Operator validation week (post-Track-A immediate next step)

The single highest-ROI action before queuing more conductor
work is operator-driven user interviews against the Track A
personas. Plan:

- 3 × Delegator interviews (~45 min each): walk through the
  hire-flow journey paper-prototype, capture friction.
- 2 × Approver shadow sessions (~half-day each): observe real
  approval cadence on existing tools (Slack, iMessage, SMS),
  test the card-mockup on their phone.
- 1 × Operator interview (~60 min): walk through the operator-
  console concept, capture compliance/audit needs.

Output: a `validation-findings-2026-05-XX.md` doc under
[../skycot/ux-research/](../skycot/ux-research/) that updates the
load-bearing persona docs in-place and drops `(assumed)` tags
where validated.

## Open questions for the operator

- Do you want to formally split CLAUDE.md instructions per
  track, or keep one shared CLAUDE.md? (Recommendation: keep
  one, reference this roadmap.)
- Do you want a separate brand identity for SKYCOT before or
  after Track C? The rename is documented in CLAUDE.md as
  deferred; the brand could be deferred with it, or locked
  early in Track C. (Recommendation: lock early so Track E
  is brand-consistent.)
- Do you want a hosted demo environment (skycot.com) standing
  up alongside Track E for user-validation rounds, or only
  after E ships a working surface? (Recommendation: stand it
  up after the approvals-inbox slice lands — first viewable
  surface.)
