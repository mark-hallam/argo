# Product Roadmap — post-MVP path to every mission claim

**Authored against**: origin/main `70e4b020` on `2026-05-11`.
**Status**: operative. Supersedes the v1.4-and-beyond sections of
`BUILD_PLAN_V11.md` for the product-surface program; backend tracks
in V11 (v1.1 production hardening, v1.3 multi-worker, v2.0 self-
improvement) remain authoritative for their scopes.

> Fresh Claude sessions: this document is the operative plan for
> the product-surface phase. Read this **after** `BUILD_STATUS.md`
> header and `MISSION.md`, **before** picking up any UI / UX /
> persona / journey work.

## What this document is

The path from v1 MVP (mock-mode smoke green, all mission backend
wires merged, no polished UX) to **every mission claim landed in
a felt user experience.** Captures seven sequenced tracks (A–G),
their dependencies, their status, and which mission claim each
closes.

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
| B | Capability fill-ins (Windows native CU + real-LLM journey test) | swarm | **QUEUED** 2026-05-11 — 2 briefs in [streams/](streams/) | mission claim 5 (Windows leg) |
| C | Design language — stack pick, brand tokens, component primitives | operator + 1 focused thread | **IN PROGRESS** 2026-05-11 — ADR + scaffold + 8 primitives + theming layer landed in [web/](../../web/); typecheck + build + 6/6 unit tests green | foundation for D + E |
| D | Wireframes for the 5 surfaces (hire / team / worker profile / approvals inbox / audit timeline) | operator + claude, validated against real users | **DONE** 2026-05-11 — README + 5 surface specs in [skycot/ux-research/wireframes/](../skycot/ux-research/wireframes/); each wireframe carries ASCII layouts, exact strings, mobile reflow, and 5-10 Track-E brief slicing hints | implementation brief for E |
| E | Frontend implementation, sliced thin (approvals inbox → hire → team → worker profile → audit) | swarm, after C+D land | **IN PROGRESS** 2026-05-11 — E0 (foundation CONTRACT PR) authored in [streams/](streams/); ~40 slice briefs scheduled in /loop | mission claims 1, 2, 3, 6 |
| F | Backend fill-ins for claim 4 — 6 outstanding skill/marketplace TODOs | swarm, parallel to E | **NOT STARTED** | mission claim 4 |
| G | Mission-5 finish: Windows dispatch wiring + Wayland Linux + real-LLM-test CI integration | swarm, parallel to E | **NOT STARTED** | mission claim 5 (full) |

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

Plus `README.md` — index, conventions, validation plan.

**Validation status**: all docs carry `(assumed — needs
validation)` tags. Per operator preference (memory
`feedback_build_then_validate`), interviews are deferred until
there's working software to test against — Track E's
approvals-inbox slice is the first such surface. The intent is
to update these docs in-place with what real-user feedback
reveals rather than gate further work on validation.

## Track B — capability fill-ins (QUEUED)

Two briefs authored 2026-05-11, queued for swarm pickup:

- [streams/mission-cu-native-windows-poc.md](streams/mission-cu-native-windows-poc.md)
  — NEW crate `argo-executor-computer-use-windows`, PowerShell-
  built-in subprocess primitives, mirrors macOS #941 + Linux POC.
  Closes the third leg of mission claim 5.
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

**Status update 2026-05-11:** ADR locked
([streams/adr-frontend-design-language.md](streams/adr-frontend-design-language.md)),
`web/` Next.js 15 scaffold landed, all 8 primitives + the load-
bearing `ApprovalCard` shipped, multi-tenant theming layer
(SKYCOT light/dark + Acme tenant white-label demo + OKLCH palette
synthesizer for arbitrary brand hues) wired end-to-end. Production
build green (29.5KB demo page, 131KB First Load JS); typecheck
clean; 6/6 ApprovalCard unit tests passing.

**What's left for DONE:**
- Storybook stories per primitive (TODO — next thin slice).
- Playwright e2e smoke (TODO).
- Real gateway wire-up against `/inbox` endpoint to replace the
  static demo page (handover point to Track E).
- Operator validation pass: open `pnpm dev` at
  `http://localhost:3000`, hit the page, react. This is the
  build-then-validate moment.

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

## Track D — wireframes (NOT STARTED)

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

## Track E — frontend implementation (NOT STARTED)

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

## Track F — backend fill-ins for mission claim 4 (NOT STARTED)

Six outstanding TODO mission rows in the ledger that close the
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

## Track G — mission claim 5 finish (NOT STARTED)

Follow-ups after Track B lands:

- Windows dispatch wiring (`mission-cu-native-windows-executor-
  dispatch`, mirrors mac #955) — depends on Track B's POC merged.
- Wayland Linux CU primitives (`mission-cu-native-linux-wayland`,
  flagged TODO in the existing Linux POC brief).
- Real-LLM journey test → CI integration (deferred follow-up
  noted in the Track B brief).

Sizing: ~1 week of swarm work.

## Mission-claim coverage map

| Claim | Tracks that close it | Status |
|---|---|---|
| 1. Teammate, not tool | A (design) → C → D → E (team page + worker profile) | A done; rest not started |
| 2. Delegation is the unit | A (design) → C → D → E (hire flow + delegate-ongoing) | A done; rest not started |
| 3. Platform builds workers | Backend MERGED; A → D → E (hire-flow UI) | A done; UI not started |
| 4. Skills shared and learned | F (backend) + E (skill library UI, deferred slice) | not started |
| 5. MCP + computer-use | B (Windows POC) → G (dispatch + Wayland) | B queued; G not started |
| 6. Multi-channel approval gates | All channels wired; A → D → E (approvals inbox) | A done; UI not started |

## Sequencing rules

- **A blocks D.** Do not wireframe surfaces against unvalidated
  personas. Run user interviews before locking D.
- **C blocks E.** Do not implement frontend slices against an
  unlocked design language. Locking C is irreversible — get it
  right with the operator in the loop.
- **D blocks E for the slice it covers.** A slice cannot ship
  without its wireframe; do not improvise the surface from the
  persona doc alone.
- **F and G parallelize with E.** Both are backend; both are
  swarm-friendly; no UI dependency.
- **B is independent.** Already queued; do not block any other
  track on it.

## Status tracking

The roadmap document is updated when a track flips state.
Status values per track: `NOT STARTED`, `IN PROGRESS`, `BLOCKED`,
`DONE`. Per-brief status continues to live in
[BUILD_STATUS.md](BUILD_STATUS.md) ledger rows.

When a fresh session picks this up, the way to read it is:

1. Confirm the freshness anchor at the top against
   `git rev-parse --short origin/main`. If the SHA is > 30 days
   old, the operator should re-validate the track statuses
   before queuing new work.
2. Read the table at the top — that's the operative state.
3. Drill into the section for the track you're working on.
4. Cross-reference [BUILD_STATUS.md](BUILD_STATUS.md) for the
   live brief-level status inside the relevant track.

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
