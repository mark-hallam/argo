# Dreams + runtime-swarm investigation (tick 78)

Two questions. Two verdicts. Both with brief proposals
attached so the conductor can pick them up if approved.

---

## 1. Executive summary

- **Claude Dreams is a real, shipped, beta API** that takes a memory
  store + up to 100 session transcripts and produces a *new*
  reorganized memory store: deduped, contradictions resolved, stale
  entries replaced, novel insights surfaced. It does NOT modify the
  input — it is non-destructive curation behind an explicit review
  point.
- **Verdict on Dreams: SHIP**, but as a **v2.0 self-improvement
  feature**, not a v1.x capability. It is the cleanest path we have
  to "workers actually get better over time" (claim 4) and the
  natural successor to the existing pending-review skill flow. Cost
  is small; safety story matches our existing approval gate; the
  beta API surface is two tools (create dream + poll dream) plus a
  store-swap. Defer until v1.1 reliability lands.
- **Verdict on runtime conductor/worker swarms inside SKYCOT
  workers: DEFER + REFRAME**. The git-ledger pattern that builds
  Argo does NOT translate directly. A SKYCOT worker doing real-world
  work has no PRs, no main branch, no stop condition gated by CI.
  What translates is a narrower piece — *task decomposition with a
  ledger* — and we already have most of the primitives (Session,
  Handoff, EscalationChain, Workflow-DAG). The right move is not to
  graft the build-pattern onto runtime workers; it is to extend the
  primitives we have with a **planner / sub-task ledger** for
  in-worker decomposition.
- **Cross-pollination** is the most interesting product idea.
  Dreams that consume *swarm trajectories* (not just chat
  transcripts) become "the worker reflects on its own multi-step
  delegation patterns" — which IS the missing piece behind both
  claim 4 (self-learning skills) and the multi-worker handoff
  story. That pattern is genuinely new.
- **Mission impact**: Dreams advances claims 1, 4, 5; runtime
  swarms (the narrower planner version) advances claims 1 and 2.
  Neither expands the six load-bearing claims; both make existing
  claims more production-real.

---

## 2. What Claude Dreams actually is

Distilled from the [Anthropic docs page][dreams-docs] (the canonical
reference) and the [TheNewStack writeup][newstack] / [community
guide][claudefast].

**Architecturally**, a "dream" is an asynchronous job:

- **Inputs**: one existing `memory_store` (required) + up to 100
  `session` transcripts (optional, but the interesting case).
- **Output**: a *new* `memory_store` with the same shape, deduped,
  reorganized, and enriched with patterns Claude found across the
  transcripts. The input store is read-only; the output is
  separate. Either one can be attached to future sessions.
- **Optional `instructions` field** (≤4096 chars) lets you steer the
  curation: "Focus on coding-style preferences; ignore one-off
  debugging notes," etc.
- **Models**: `claude-opus-4-7` or `claude-sonnet-4-6` during
  preview. Standard token billing — no special pricing.
- **Lifecycle**: `pending → running → completed | failed | canceled`.
  Runs minutes to tens of minutes. Polled via
  `GET /v1/dreams/{id}`, cancellable via
  `POST /v1/dreams/{id}/cancel`.
- **API surface**: ~5 endpoints (`create`, `retrieve`, `list`,
  `cancel`, `archive`). Beta header
  `dreaming-2026-04-21` required, plus the existing
  `managed-agents-2026-04-01`.
- **Limits**: 100 sessions per dream, 4096-char instructions, the
  two preview models. Default rate limits — no published per-tenant
  quota.
- **Safety model**: non-destructive (input untouched), the resulting
  store is just another memory store you can attach, archive, or
  delete. There is **no auto-merge** — the operator chooses to
  attach the dream output to future sessions or discards it. This
  matches the SKYCOT pending-review pattern precisely.
- **What it is not**: it is not an autonomous learning loop, not an
  RL fine-tune, not a fact-extraction pipeline that exposes raw
  outputs to the model mid-conversation. It is a periodic offline
  curation pass.

The auto-trigger story (`/dream` slash command, 24h+5-session
threshold) described in [the community guide][claudefast] applies to
**Claude Code's own** memory consolidation, NOT to the Managed
Agents API. The API itself is fired by your code; you decide when.

So in the language of our six claims: Dreams is the official
Anthropic answer to memory entropy in long-lived agents. It is
exactly the thing the SKYCOT worker needs after running for weeks
and accumulating contradictory notes.

---

## 3. Question 1 deep-dive — does Dreams fit SKYCOT?

### What we have today (relevant pieces)

`argo-memory` already ships a 3-layer store: `Recent` (last 30
days, verbatim) → `Indexed` (older, hybrid retrieval) → `Summary`
(monthly worker rollup). The `summary.rs` job is a placeholder
nightly batch that already approximates one third of what Dreams
does. The `argo-skill::learn` module ships `should_propose` +
`SkillCandidate::from_cu_trajectory` + `propose_for_review` — a
human approves or rejects a learned skill before it's promoted out
of `LearnedUnreviewed`.

The shape is therefore *already aligned* with Dreams' shape: a
non-destructive proposal that flows through human review before
being promoted. Dreams is the missing automation that turns "we
hand-wrote a summary every month" into "an LLM curates the store
on a schedule."

### What integrating Dreams would look like

Three integration points, in increasing ambition:

**Integration A — Drop-in replacement for the nightly summary
job.** Today `summary.rs` is ~placeholder rollup-by-worker-month.
Replace its core with a Dreams call: read all `Recent` + `Indexed`
rows for the worker's `MemoryNs`, emit them as session-shaped
inputs, fire a dream, swap the output store into the `Summary`
layer once a human approves. Implementation surface: a new
`argo-memory::dreams` module (~200 LOC) + a CLI subcommand
`argo memory dream <worker-id>` to fire it on demand. Cost: minor
— the per-worker summary job already runs nightly; we're just
swapping the implementation. Phase: **v2.0 self-improvement**
(early in the phase — this is the cheapest dream win).

**Integration B — Dreams-on-skill-learn-trajectory.** Today a
single CU trajectory above the success threshold becomes a
`SkillCandidate`. But the *truly* interesting skills are patterns
across N successful trajectories ("Maya always opens marketplace,
then waits 2s, then clicks the Sell button"). Wire dreams to fire
on a window of N successful trajectories per `(worker_id,
action_type)` pair. The dream's instructions field carries the
"distill a reusable skill from these trajectories" prompt. The
output store becomes the `SkillCandidate` body. Cost: mid — it's
N tokens per cohort instead of one. Phase: **v2.0
self-improvement**, after Integration A.

**Integration C — Dream-as-EscalationStep** (speculative). When
a worker is stuck (3 consecutive low-success runs, an anomaly
hit), trigger a dream with "identify what's going wrong" as the
instruction. Genuinely new product behaviour; needs its own
brief + safety review. Phase: **v2.1** or later.

### Mapping to mission claims

| Claim | Does Dreams advance it? | How directly |
|---|---|---|
| 1. Real teammates | Yes | A teammate that doesn't accumulate contradictions over months IS more teammate-feeling. Direct. |
| 2. Delegation as the unit | Indirect | Dreams curate memory; delegation is unaffected. |
| 3. Platform-built workers | No | Genesis owns vertical generation; dreams operate on the worker AT RUNTIME. |
| 4. Skills shared and learned | **Yes — strongly** | Integration B IS the missing piece behind "novel patterns become reusable skills." This is the highest-leverage hook. |
| 5. Real-world execution | Indirect | Better memory → better task execution. Tertiary. |
| 6. Multi-channel approvals | No | Approval gate unchanged. |

So Dreams is a **claim 4 amplifier**, with secondary value on
claims 1 and 5. It does not expand the six load-bearing claims; it
deepens claim 4 from "we ship the proposal pipeline" to "the
proposals come from a periodic LLM-curated cohort, not a
single-trajectory threshold."

### Cost / quota / safety

- **Tokens**: standard model rates. A typical dream (100KB store
  + 20 × 50KB sessions ≈ 250K input tokens on sonnet-4-6) ≈
  ~$0.75. One dream per worker per week is rounding error.
- **Quota**: default rate limits on dream creation; no published
  per-tenant cap. Manageable.
- **Safety**: input is read-only; output is reviewable. Same
  pattern as the existing skill-candidate flow. Risk: a dream
  surfaces a *wrong* insight ("Maya prefers nighttime posting")
  that gets attached and biases future runs. Mitigation: gate the
  swap-in behind operator approval, like skill candidates.
- **Beta**: requires `dreaming-2026-04-21` header; Anthropic may
  change the surface. Wrap behind an adapter module — one-file
  fix on API change.

### Recommended phase placement

**v2.0 self-improvement.** The plan slot
`claude-skill-versioning-ab-test` is currently a sibling — Dreams
fits next to it as `claude-dreams-summary-job` (Integration A)
+ `claude-dreams-skill-cohort` (Integration B). Both are claude
briefs in the 500-1500 LOC range.

### Brief proposals (for the conductor to accept)

```
claude-dreams-summary-job
  Depends on: argo-memory v1, argo-identity v1
  Scope: Replace summary.rs nightly rollup with a Dreams call.
    Emit memory_store + recent sessions; persist output store
    into Summary layer after operator approval.
  May touch: argo-memory/src/dreams.rs (new),
    argo-memory/src/summary.rs, argo-cli/src/memory.rs
  Acceptance: end-to-end against fixture; CLI subcommand
    "argo memory dream <worker>"; manual-approval gate on swap.

claude-dreams-skill-cohort
  Depends on: claude-dreams-summary-job
  Scope: When N (default 5) successful CU trajectories for the
    same action_type accumulate, fire a Dreams job to distill
    them into a SKILL.md body. Replace per-trajectory body
    generation in argo-skill::learn with the cohort output.
  May touch: argo-skill/src/learn.rs, argo-skill/src/cohort.rs
    (new), argo-memory/src/dreams.rs
  Acceptance: cohort fixture passes; SkillCandidate body
    references the dream output; propose_for_review unchanged.
```

### Honest read

**Ship it. Defer to v2.0.** ~700 LOC across two briefs. The
existing pending-review pattern absorbs the safety story. Defer
Integration C — recovery-primitive use is new product behaviour
and should land after v1.1 reliability is bedrock.

The temptation will be to do this earlier because it's a shiny
Anthropic feature. Resist. v1.1 reliability is what unlocks every
self-improvement loop — workers that go off-rails poison their
own memory, and Dreams curating a poisoned store turns a transient
bug into a confident-and-wrong worker.

---

## 4. Question 2 deep-dive — should runtime workers use conductor/worker swarms?

This is meta-recursion: the platform-build pattern, applied
inside the platform-built workers. Tempting because of the
elegance. The honest answer is: **the analogy breaks where it
matters most.**

### Where the build-pattern translates

- **Lock-free git ledger.** Translates *partially*. A runtime
  worker has SQLite, not git. We could simulate first-push-wins
  with a transactional row-update — but that's just a DB lock,
  not the audit log + branchable history.
- **Brief-as-spec.** Translates well. A "task" delegated to a
  runtime worker is brief-shaped: scope, constraints,
  acceptance. Genesis already emits action manifests like this.
- **First-push-wins.** Doesn't translate. No main branch, no PR.
  Runtime workers produce *actions*, not diffs.
- **Asymmetric conductor/worker.** Translates partially —
  `EscalationChain` walks workers in priority order. But we do
  NOT have a conductor that *decomposes* a delegated role into
  sub-tasks. That's the gap.

### The actual question

Reframe: **does a runtime worker need a planner?** When the
operator says "Maya, promote this product launch," does Maya
internally decompose into research-audience → draft-copy →
design-post → schedule → monitor, and orchestrate child workers
(or her own action loop) across the steps?

Today: Maya is a single-agent loop. The vertical's action
vocabulary lists what she can do; her main loop picks the next
one each tick. There is no plan, no "run sub-task on worker B
and await result."

The gap is three things:
1. **No planner** — no primitive that decomposes "promote
   product launch" into a DAG of sub-tasks.
2. **No sub-task ledger** — a worker can't claim and release a
   sub-task the way a worker-tab claims a brief row. `Handoff`
   is closer to role-transfer than sub-task delegation.
3. **No first-class child-worker spawn** — a worker can
   `Handoff` along an `EscalationChain`, but can't "spawn a
   transient sub-worker, give it this scope, await, fold."

### What the existing primitives already cover

We're not at zero. `argo-orchestrator` ships Session, Handoff,
EscalationChain. `argo-workflow` (just landed) ships a
declarative DAG runner with `dag.rs` + `executor.rs` + `expr.rs`.

- **Session** — thread of work joining workers + humans.
- **Handoff** — pass control with payload + expected outcome.
- **EscalationChain** — priority-ordered walker for stuck tasks.
- **Workflow-DAG** — declarative DAG; "Maya's launch recipe."

The missing piece is **dynamic in-flight planning** — a worker
that, mid-task, decides it needs sub-task X and Y. Workflow-DAG
is *static*: author the DAG offline, the worker runs it.

### The cleanest bridge

Don't graft the build-pattern onto runtime workers. Extend the
primitives we have with one new piece: the **planner trait**.

```
trait Planner {
    // Given a delegated task + the worker's current memory,
    // emit a DAG of sub-tasks. Each sub-task is a Task with
    // scope, constraints, acceptance criteria — exactly the
    // shape Workflow-DAG already accepts.
    fn plan(&self, task: &Task, memory: &MemoryView) -> ArgoResult<TaskDag>;
}
```

Two implementations:

- **StaticPlanner** — wraps a hand-authored Workflow-DAG. What
  Genesis emits today, lifted into the trait. Default for v1.
- **LlmPlanner** — Claude (or whichever model the worker uses)
  decomposes the delegated task into a DAG. Output is the same
  `TaskDag` shape; the worker runs it via the existing Workflow-DAG
  executor.

This matches the build-pattern's *brief-author* role (the conductor
decomposes mission → briefs), not its *coordination* role (the
git ledger). Coordination inside a worker's plan is the existing
DAG executor — events on edges, parallelism allowed but not
required.

### Comparison to alternatives

| Pattern | Fits SKYCOT runtime? | Why / why not |
|---|---|---|
| Single-agent loop (today) | Floor; works for simple tasks | Can't decompose. Blows up on complex roles like "marketing intern." |
| Hierarchical role (CrewAI) | Mismatched | Roles are typed (PM/Engineer/QA). SKYCOT roles are emergent — Maya isn't a typed role, she's an instance with persona + duties. |
| Typed graph (LangGraph) | Closest analog | LangGraph IS what Workflow-DAG resembles. Difference: LangGraph's typed shared state vs SKYCOT's session-scoped memory. We already do most of this. |
| Message-bus (AutoGen) | Misfit | Workers don't gossip; they hand off. Bus-style chatter doesn't match the delegation model. |
| Build-pattern (lock-free git) | Doesn't translate | No PRs, no main, no CI. The lock primitive doesn't have a runtime analog. |

The conclusion is that SKYCOT runtime is closest to **LangGraph
shape** with the planner being the missing piece.

### Mapping to mission claims

| Claim | Does runtime-swarm advance it? | How |
|---|---|---|
| 1. Real teammates | Yes | A teammate that decomposes "promote a launch" into a real plan IS more teammate-like. Direct. |
| 2. Delegation as the unit | **Yes — strongly** | The planner is the missing primitive that turns "delegate a role" into "execute a plan derived from the role's duties." This is the highest-leverage hook for claim 2. |
| 3. Platform-built workers | Indirect | Genesis would emit StaticPlanner DAGs by default; LlmPlanner is opt-in. |
| 4. Skills shared and learned | Indirect | Sub-tasks become candidates for skill-learning. |
| 5. Real-world execution | Indirect | Decomposed tasks → more PC/Mac coverage per delegation. |
| 6. Multi-channel approvals | No | Approval gate unchanged. |

So runtime-swarm-as-planner is a **claim 2 amplifier**, with
secondary value on claim 1.

### Recommended phase placement

**v1.3 multi-worker orchestration.** The phase is named for
exactly this. Currently the phase brief list includes
`claude-workflow-dag` (just landed), `claude-worker-roster`,
`claude-task-distribution`, `claude-checkpoint-resume`,
`claude-org-awareness`. The planner trait fits between
workflow-dag and worker-roster:

```
claude-planner-trait
  Depends on: claude-workflow-dag
  Scope: Introduce trait Planner with StaticPlanner +
    LlmPlanner. Static wraps hand-authored DAGs; Llm emits a
    DAG via a model call. Worker main loop becomes "ask the
    planner for a DAG, run it, ingest results."
  May touch: argo-orchestrator/src/planner.rs (new),
    argo-orchestrator/src/lib.rs, argo-workflow/src/lib.rs
    (re-export TaskDag), argo-agent-runner/src/main_loop.rs
  Acceptance: Static unit fixtures pass; Llm integration test
    against stubbed client; main loop e2e walks 3-node DAG.

claude-subtask-ledger
  Depends on: claude-planner-trait
  Scope: Persist each planner-emitted sub-task as a row with
    status todo|claimed|completed. CLI "argo workers tasks
    <worker>" + GET /workers/{id}/tasks.
  May touch: argo-orchestrator/src/subtask.rs (new),
    argo-gateway/src/tasks.rs (new endpoint),
    argo-cli/src/workers.rs
  Acceptance: ledger persists across restarts; status
    transitions append-only; gateway returns JSON sub-task
    list per worker.
```

### Honest read

**Defer the full build-pattern recursion. Ship the planner
piece.** The temptation to "make Maya use the same pattern that
builds Argo" is mostly aesthetic. The actual gap — Maya can't
decompose a real role into a real plan — is solved by one trait
+ one ledger, not by recreating the conductor/worker/git
topology inside a worker. Git-as-lock is load-bearing for the
*build* because N coding agents race on one repo; inside a
runtime worker there is no race, no branch, no PR.

What I would NOT do: spin up a conductor microservice per
worker that orchestrates child workers via PRs to a private
git repo. Recursive and pretty and the wrong abstraction.

---

## 5. Cross-pollination — Dreams + runtime planner

Where the two combine cleanly:

**Dreams that consume swarm trajectories.** Once a worker has a
planner emitting DAGs and a sub-task ledger persisting their
outcomes, you have *structured* trajectory data — not just
"the worker took these N actions" but "the worker decomposed
this role into these sub-tasks, ran them in this order, and these
N succeeded / M failed." Feeding *that* into a Dreams pass yields
genuinely useful insights:

- "Maya's product-launch plans always include a draft-copy step,
  but it succeeds 95% on Tuesdays and 60% on Fridays" → encoded
  back into the planner's bias for future plans.
- "When the design-post step succeeds, the schedule step succeeds
  98% of the time; when design-post fails, schedule fails 80%" →
  surface the dependency to the planner.

This is the **claim 4 + claim 2 unified loop**: workers learn how
to *plan better*, not just how to *act better*. Single-trajectory
skill learning catches "what action to take in situation X."
Cohort-trajectory dreams catch "how to structure a plan for
situation X." The latter is the more durable artifact.

The implementation surface is small: extend
`SkillCandidate::from_cu_trajectory` with
`from_planner_cohort(plans: Vec<TaskDag>, outcomes: Vec<...>)`.
Output is a planner-template, not an action-template. New trust
grade `LearnedPlanTemplate` slots into the existing TrustGrade
ladder.

This combination is genuinely novel — I haven't seen it
documented elsewhere. It's also the clearest reason to ship both
features rather than just one.

**Phase placement**: cross-pollination brief lands in **v2.0
self-improvement**, after both `claude-dreams-skill-cohort` and
`claude-planner-trait` are in. Probably one brief:
`claude-planner-dream-cohort`.

---

## 6. Updates to BUILD_PLAN_V11

Three insertions, in the same shape as the existing plan.

**Phase v1.3 — Multi-worker orchestration.** Insert after
`claude-workflow-dag`:

```
- claude-planner-trait (Planner trait + StaticPlanner +
  LlmPlanner; main loop ingests DAG)
- claude-subtask-ledger (per-worker sub-task ledger +
  gateway endpoint + CLI subcommand)
```

**Phase v2.0 — Self-improvement.** Insert near
`claude-skill-versioning-ab-test`:

```
- claude-dreams-summary-job (replace summary.rs nightly
  rollup with a Dreams API call)
- claude-dreams-skill-cohort (cohort-distilled SkillCandidate
  body via Dreams)
- claude-planner-dream-cohort (Dreams over planner cohorts;
  emit LearnedPlanTemplate into TrustGrade ladder)
```

**Sequencing rules update.** Add to the existing rules:

- **v2.0 self-improvement requires v1.3 planner.** The
  planner-dream-cohort brief depends on the sub-task ledger
  existing.

These six briefs slot into the standard step-8 replenishment
loop. Estimated total: ~4500 LOC across the six, ~6 conductor
ticks of merge time.

---

## 7. Updates to MISSION.md

**Recommended: no changes.** Both Dreams and the planner trait
deepen existing claims rather than expand them. The six
load-bearing claims still hold.

One soft suggestion: the existing claim 4 ("Skills are shared and
learned") could clarify that learning includes both
*action-templates* (what we ship today) and *planning-templates*
(what cross-pollination introduces). A one-line addition to the
claim-4 paragraph:

> Skills include both **action sequences** and **planning
> structures**: workers learn not only what to do in a situation,
> but how to decompose a delegated role into a plan.

Optional. The architecture is clearer without it; adding it just
makes the planner-cohort brief easier to defend during review.

---

## 8. Verdict — should we ship this?

### Question 1 — Claude Dreams: SHIP, in v2.0

Reasons:
1. The integration matches our existing pending-review pattern —
   non-destructive, human-approved, reversible. Safety story is
   the same as today's skill candidate flow.
2. The cost is bounded (~$0.75 per dream, weekly cadence per
   worker per tenant ≈ rounding error).
3. Integration B (Dreams over skill-learn cohorts) is genuinely
   the missing piece behind claim 4. Without it, we're shipping a
   single-trajectory threshold; with it, we're shipping pattern-
   distilled skills.
4. Beta-API risk is bounded — wrap behind one adapter module.

Reasons NOT to ship now:
1. v1.1 reliability isn't done. A worker that goes off-rails
   poisons its own memory; Dreams curating a poisoned store turns
   a transient bug into a confident-and-wrong worker. Get the
   anomaly detector + circuit breaker in first.
2. v2.0 is the right phase by the existing sequencing rules.

### Question 2 — Runtime conductor/worker swarms: REFRAME, ship the planner piece in v1.3

Reasons:
1. The full build-pattern recursion is mostly aesthetic. The
   git-as-lock primitive doesn't have a runtime analog and the
   work to invent one wouldn't pay back.
2. The actual product gap (workers can't decompose a delegated
   role) is solved by one trait + one ledger, not by recreating
   the whole conductor/worker topology.
3. v1.3 is named for exactly this work and currently has slack
   in the brief list (`claude-workflow-dag` just landed; the
   sibling briefs are incomplete).
4. The planner trait composes with Dreams cleanly (cross-
   pollination above). Building one without the other leaves
   real value on the table.

Reasons NOT to ship the full swarm:
1. Recursive elegance is the wrong reason to ship a primitive.
2. The pattern was designed for asynchronous concurrent agents
   on a single git repo with CI; runtime workers don't have any
   of those properties.

### Net recommendation

Two-step shipping plan:
1. **Now**: queue `claude-planner-trait` + `claude-subtask-ledger`
   into v1.3. Total ~1500 LOC, two briefs.
2. **After v1.1 reliability lands**: queue
   `claude-dreams-summary-job` + `claude-dreams-skill-cohort` +
   `claude-planner-dream-cohort` into v2.0. Total ~3000 LOC,
   three briefs.

That gets us:
- Workers that can decompose a role into a plan (claim 2 made
  real).
- Workers that periodically curate their own memory (claim 1
  made real over weeks).
- Workers that learn planning patterns from cohorts of past
  runs (claim 4 generalised).

Five briefs. Two phases. No mission expansion. All six load-
bearing claims still hold.

---

## 9. Sources

- [Claude Dreams official docs (Anthropic)][dreams-docs] —
  the canonical reference for the API surface, lifecycle,
  limits, and billing.
- [TheNewStack — Anthropic will let its managed agents
  dream][newstack] — high-level positioning of the feature
  (subscription wall, headline only accessible).
- [Anthropic Managed Agents overview][managed-overview] —
  parent docs for the Managed Agents API stack Dreams sits on.
- [Claude Auto Dream guide (community)][claudefast] — the
  Claude-Code-side memory consolidation feature (related but
  distinct from the API).
- [Anthropic May 2026 release notes (releasebot)][releasebot]
  — confirms the dreaming-2026-04-21 beta header date.
- [SKYCOT MISSION.md][mission] — the six load-bearing claims.
- [SKYCOT BUILD_PLAN_V11.md][buildplan] — the strategic phase
  plan beyond v1.
- [SKYCOT PROTOCOL.md][protocol] — the build-pattern that
  drives the platform build.
- [SKYCOT SWARM_COMPARISON.md][swarmcompare] — recent
  comparison of SKYCOT's coordination method vs alternatives.

[dreams-docs]: https://platform.claude.com/docs/en/managed-agents/dreams
[newstack]: https://thenewstack.io/anthropic-managed-agents-dreaming-outcomes/
[managed-overview]: https://platform.claude.com/docs/en/managed-agents/overview
[claudefast]: https://claudefa.st/blog/guide/mechanics/auto-dream
[releasebot]: https://releasebot.io/updates/anthropic
[mission]: ../skycot/MISSION.md
[buildplan]: ../build-plan/BUILD_PLAN_V11.md
[protocol]: ../build-plan/PROTOCOL.md
[swarmcompare]: ../skycot/SWARM_COMPARISON.md
