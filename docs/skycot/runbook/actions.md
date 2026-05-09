# SKYCOT Operator Runbook — Actions, skills, and handoffs

How to drive an action through its state transitions, how the skill
learning loop produces SKILL.md proposals, and how multiple workers
hand off mid-task. For the full table of contents, see
[RUNBOOK.md](../RUNBOOK.md).

---

## 6. Drive the first action

### 6.1 Emit an action

The cleanest path is via the agent loop, but for the runbook
we'll emit one manually so you can see the state transitions:

```bash
argo work add \
  --vertical marketing-intern-maya \
  --action research_topic \
  --payload '{"topic":"dev tools trending in 2026"}'
```

### 6.2 Observe the state

```bash
argo worker actions list
# action_id  state    requires  approved_by  action_type
# act-aB3c   pending  false     —            research_topic

argo worker actions show act-aB3c --json
```

Because `requires_approval = false`, this row would normally
auto-execute. But our worker's `dry_run = true` (set by
Genesis 24h ago), so it stays `pending`.

### 6.3 Approve manually (or let the gate auto-approve)

```bash
argo worker actions approve act-aB3c \
  --note "approved during runbook walkthrough"
```

If the workspace policy is configured + the action is
within budget, you can skip the manual approve and just
execute — the gate handles it:

```bash
argo worker actions execute act-aB3c
# pending → approved (auto:policy:risk=low_within_caps) → executing
# bundle:       .skycot/outputs/<job>/actions/act-aB3c/
```

Inspect the bundle:

```bash
ls .skycot/outputs/<job>/actions/act-aB3c/
# action.json    evidence.md    instructions.md
```

`instructions.md` is the operator-facing brief — the action's
payload, evidence, and a "next step" footer pointing back at
`argo worker actions complete`.

### 6.4 Mark complete

```bash
argo worker actions complete act-aB3c \
  --note "shared the trend report in #marketing channel" \
  --result-json '{"shared_url": "https://slack.com/archives/C123/p456"}'
```

Now the row's state is `completed`, the result is in the
audit chain, and any `[on_complete_emit]` follow-ups in the
manifest fire automatically.

### 6.5 Failure handling

If something blocks the action:

```bash
argo worker actions fail act-aB3c --reason "couldn't access trends.dev"
```

Or if the action is approval-gated and you want to refuse:

```bash
argo worker actions block act-aB3c --reason "campaign on hold this week"
```

Both are **terminal** for that action — to retry, emit a new
action that references the failed one in `evidence`.

```bash
argo worker actions retry <failed-id>
```

The retry chain is enforced by the per-action `[retry]` policy
in the manifest; `--force` bypasses the cap with a stderr
WARN breadcrumb.

---

## 7. Skill learning loop

When the executor (especially `ComputerUseExecutor`) completes
a successful trajectory above the success threshold, the
runtime drafts a `SkillCandidate` and lands it in the
`pending_review` lifecycle.

### 7.1 The structured success metric

Every CU completion produces a `SuccessMetric`:

```rust
SuccessMetric {
    confidence: 1.0,            // 1.0 = stop_success, 0.0 = handoff
    trajectory_length: 5,       // CU steps taken
    duration_seconds: 12.5,     // walltime
    final_state_score: Some(0.92), // optional fixture match
}
```

The default `ProposalThreshold`:

- `min_confidence`: 0.8
- `max_trajectory_length`: 25
- `max_duration_seconds`: 600.0
- `min_final_state_score`: None (skipped unless set)

A metric must clear **every** floor to propose. Quick wins
that finish in 5 steps in 12 seconds with high confidence
become candidates; 30-step grinds with low confidence don't.

### 7.2 List pending proposals

```bash
argo skill review list
# id                   name                       lifecycle      vertical
# sk-xY7p              post_listing_session-abc   pending_review marketplace_browser
```

### 7.3 Inspect

```bash
argo skill review show sk-xY7p
# (full SKILL.md body, including the trajectory)
```

The body's `provenance.source` is `learned:<session-short>`
which the H-4a TrustGrade ladder grades as
`LearnedUnreviewed` — the strictest tier. This means the
runtime won't surface this skill to high-risk actions until
you approve it.

### 7.4 Approve / refine / reject

```bash
# Approve as-is
argo skill review approve sk-xY7p

# Refine the body before approving (opens $EDITOR)
argo skill review edit sk-xY7p
# (edit the SKILL.md body)
argo skill review approve sk-xY7p

# Refuse
argo skill review reject sk-xY7p --reason "didn't actually solve the task"
```

After approval, the skill's lifecycle flips to `active` and
its `from_source` grade promotes to `Tenant` (workspace
trust). The runtime will start surfacing it for matching
actions.

---

## 8. Multi-worker handoff

The orchestrator's `Session` + `Handoff` types let one worker
hand off mid-task to another. v1 ships the primitives; the cli
surface for full session start/list/show is part of an
in-flight follow-up (`H-orchestrator-cli`). Today you can
exercise it through the gateway HTTP endpoints (also in
flight: `H-orchestrator-routes`).

### 8.1 Start a session (programmatically, until the cli ships)

```rust
use argo_orchestrator::{Session, Handoff};

let session = Session::start("marketplace-launch", tenant_id);
let handoff = Handoff::new(maya_id, sam_id, "qualify the leads I drafted");
session.record_handoff(&handoff)?;
```

When `sam` (the SDR worker) ticks next, the handoff is in
`shared_team` memory; the worker picks it up and starts work
without re-asking the operator any of the questions `maya`
already answered.

> **Memory hydration (v1, workspace-shared scope)**: every
> agent turn now injects a `## Memory` block at the bottom of
> the system prompt containing the most recent `shared_team`
> rows for the session's tenant (top-N controlled by
> `ARGO_MEMORY_COMPOSE_TOP_N`, default 8). This is the v1
> wire from `mission-wire-memory-compose-context` — it gives
> every worker continuity across sessions without requiring
> a tool call to read memory. Per-worker scope (each worker
> sees only its own private history) ships in a follow-up
> brief once the session schema gains a `worker_id` column.
> If the memory table is unreachable on a stale DB, the wire
> soft-fails — a `tracing::warn!` is emitted and the prompt
> proceeds without the memory block.

### 8.2 Escalation chain

The role declares an escalation chain in `vertical.toml`:

```toml
[role]
system_prompt = "prompts/system.md"
persona = "prompts/persona.md"
escalation_chain = ["sam", "robin", "human:you@example.com"]
```

If `maya` runs out of retry budget, the handoff walker (which
ticks periodically — `claude-orchestrator-handoff-walker` is
in flight) escalates to `sam`. If `sam` doesn't respond
within the deadline, escalates to `robin`. Last resort: `human`.

### 8.3 Audit the chain

```bash
argo audit query --kind handoff --since-minutes 60
# at                   from   to     reason
# 2026-05-06T15:00Z    maya   sam    qualify the leads I drafted
# 2026-05-06T15:30Z    sam    robin  sam unreachable, escalating
```

Every handoff lands an audit row with a stable session id, so
you can reconstruct the chain end-to-end.

---

Next: [Audit and observability](observability.md).
