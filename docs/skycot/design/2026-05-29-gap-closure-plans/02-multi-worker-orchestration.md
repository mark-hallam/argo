# Design plan — Multi-worker orchestration (delegation beyond 1-to-1)

**Status:** ready-for-go implementation plan. Authored 2026-05-29 (overnight).
**Closes:** competitive gap #2 (multi-worker orchestration) — and shores up our
own thesis. *"Delegation is the unit"* is undercut if delegation is only ever
one teammate handing to one other.
**Effort:** programmatic engine = MEDIUM (we have the primitives) · visual
no-code canvas = HIGH (separate, high-payoff project).

---

## Plain-English summary (for the operator)

Right now one teammate can hand a task to one other teammate (Peta → Patrick).
What we *can't* yet do is have one teammate **fan a job out to several teammates
at once**, wait for them all, and combine the results — or run a small "team of
teammates" with a lead coordinating the others. Competitors (Relevance,
Beam, Dust, LangGraph) all ship some form of this; it's becoming table-stakes,
and it's literally our headline claim, so we should own it.

This plan adds real multi-worker orchestration in two layers:

- **The engine (medium effort).** A coordinator that can run several
  delegations **in parallel**, respect **dependencies** ("do B and C only after
  A finishes"), and **gather** the results — built on the handoff machinery we
  already have. After this: a lead teammate can truly run a small team.
- **The canvas (big, separate).** A drag-and-drop visual builder for these
  team workflows (like Relevance's "Workforces"). High effort, high commercial
  payoff — recommend deferring until the engine proves out.

**Recommendation:** build the **engine** now (it makes our core claim real and
is bounded), demo it as "watch Mariah delegate research to three teammates at
once and synthesize their findings," and treat the **visual canvas** as a
later, separately-funded product surface.

**One thing to explicitly DROP for v1:** the "auction/bidding" sub-idea we once
scoped (workers bid for tasks) — **no competitor does this**, it adds a lot of
complexity, and skills/SLA-based routing covers the real need. Keep it as a
someday-differentiator, not v1 scope.

---

## What exists today (verified against the repo)

- `crates/argo-orchestrator` — exists; `GET /api/v1/orchestration/runs` +
  `routes_orchestration.rs` + the `/orchestration` SPA page exist and are
  **seeded** (2 runs). But this is a *runs dashboard*, not a live DAG executor —
  it surfaces run rows, it doesn't yet schedule parallel/dependent delegations.
- **Handoff primitives exist:** delegation/handoff is wired, and the runtime
  carries the active handoff chain through `ToolContext.current_handoff_id`
  (this is what made Claim 2 LIVE). That is the foundation to build on.
- Today's shape: **flat, point-to-point** (1 delegator → 1 delegate). No
  parallel fan-out, no dependency graph, no nested lead-of-leads.

## Decisions the operator must make (the orchestration *model*)

The competitors split three ways; pick the shape that matches how you want
buyers to think about it:

| Model | What it is | Who does it | Fit for Skycot |
|---|---|---|---|
| **Supervisor** (recommended) | A lead teammate routes/parallelizes sub-tasks to named teammates, gathers results | LangGraph `create_supervisor`, Dust | Maps cleanly onto "hire a team with a lead"; demo-friendly; reuses handoff |
| **DAG / workflow graph** | Author a dependency graph of steps, each a delegation | Beam, Ema, Relevance | More powerful, but authoring it is the visual-canvas project |
| **Auction / bidding** | Workers bid for tasks | *(no competitor)* | **DROP for v1** — novelty without demand |

**Recommendation: Supervisor model.** It's the smallest thing that makes
"delegation is the unit" true at team scale, it reuses `current_handoff_id`, and
it tells the cleanest story ("a lead AI teammate running a team of AI
teammates"). The DAG/canvas is the natural follow-on once authoring matters.

## Architecture — a supervisor over the existing handoff chain

```
argo-orchestrator (extend, don't fork the runtime)
├── struct TeamPlan { lead: WorkerId, steps: Vec<DelegationStep> }
├── struct DelegationStep { to: WorkerId, task, depends_on: Vec<StepId> }
├── async fn run_team(plan, deps) -> TeamResult
│     • topological layers from depends_on
│     • each layer: spawn delegations concurrently (bounded), await the layer
│     • thread current_handoff_id so the chain + audit stay coherent
│     • gather child results → hand back to the lead for synthesis
└── persists each child delegation as an orchestration run row (existing table)
```

Invariant-safe: built **inside `argo-orchestrator`** (a vertical-agnostic core
capability), not forked into the runtime; tenant_id threaded through every
step; every delegation already audits, so a team run is a coherent audit
subtree. No new global state.

## Implementation phases

### Phase 1 — parallel fan-out + gather (MEDIUM, the core win)
- `run_team` with a single layer: lead fans N delegations out concurrently,
  awaits all, hands results back for synthesis. Bounded concurrency (reuse the
  same cap discipline as elsewhere).
- `POST /api/v1/orchestration/team-runs` to kick one off; the existing
  `/orchestration/runs` dashboard already renders the child rows.
- **Verify:** a test team-run with 3 child delegations completes, all 3 audit
  rows + the synthesis row land under the right tenant, cross-tenant isolation
  holds (the round-37 vacuous-pass guard: a different tenant sees none).

### Phase 2 — dependencies (topological layers)
- Honor `depends_on`: build layers, run each layer in parallel, barrier between
  layers. A failed step drops its dependents (record + surface, don't hang).
- **Verify:** A→{B,C}→D ordering test; a forced B-failure skips D and is
  visible as a blocked run, not a deadlock.

### Phase 3 — SPA: show the team run as a tree (demo surface)
- Extend `/orchestration` (or worker-detail) to render a team run as a
  lead-with-children tree with live status — the visible "watch the team work"
  beat. (Brand-loud register; reuse the holographic/showcase language.)
- **Verify:** browser pass — kick a team run, watch children resolve, see the
  lead's synthesis.

### Phase 4 (separate project) — visual no-code canvas
- Drag-and-drop team/DAG builder (Relevance "Workforces" class). HIGH effort;
  its own brief tree + design-exploration (optometrist method) pass. Defer.

## Risks / honesty notes
- This is the most architectural of the three plans — get the **supervisor vs
  DAG** call from the operator before authoring, or it risks building a shape
  we don't want (exactly the drift DRIFT_PREVENTION.md warns about).
- Don't fork the runtime; extend `argo-orchestrator`. Watch the <500 LOC/file +
  <80 LOC/fn ceilings (the layer scheduler wants careful splitting).
- Keep it demo-honest: until Phase 1 lands, the `/orchestration` page is a
  *dashboard of seeded runs*, not a live executor — don't narrate it as more.

## Brief decomposition (ready to author once the model is chosen)
`orchestration-supervisor-engine-fanout-gather` →
`orchestration-supervisor-dependencies-layers` →
`orchestration-team-run-gateway-route` → `web-orchestration-team-tree-view`.
Each carries the freshness header + names its consumer per DRIFT_PREVENTION.md.
