# SKYCOT Operator Runbook

**Status**: v1 customer-launch — 28 PRs merged, 4-day swarm
delivered the full Worker OS stack, e2e gate (claude-e2e-integration-test #810) passes
offline.

A walkthrough from `cargo install` to your first auto-approved
action and a live audit trail. Written for an operator who has
never seen SKYCOT — by the end you'll know what to install,
how to drive it, and where to look when something goes wrong.

If you want the *why*, read [MISSION.md](MISSION.md). This
document is the *how*.

---

## Table of contents

1. [What you're getting](#1-what-youre-getting)
2. [Install](#2-install)
3. [Bootstrap](#3-bootstrap)
4. [Generate your first worker](#4-generate-your-first-worker)
5. [Configure approvals](#5-configure-approvals)
6. [Drive the first action](#6-drive-the-first-action)
7. [Skill learning loop](#7-skill-learning-loop)
8. [Multi-worker handoff](#8-multi-worker-handoff)
9. [Audit and observability](#9-audit-and-observability)
10. [Where to next](#10-where-to-next)
11. [Security](#11-security)

---

## 1. What you're getting

> The mission verbatim, from MISSION.md:
>
> AI workers that humans can bring into their team as a real
> teammate, that can be delegated a responsibility / task / work /
> role; the platform can build an AI worker for it, draw upon a
> shared skills library, or even learn and build its own new
> skills; it has memory; it works autonomously and alongside
> humans; AI workers can fully autonomously do real-world work;
> that work is any task a human can do on a PC/Mac; approval
> requests can flow via many different communication methods.

In v1 SKYCOT ships:

- **A vertical generator** (`argo genesis create`) that turns a
  one-line charter into a complete worker template — system
  prompt, action vocabulary, memory schema, eval seeds. 12
  archetypes ship out of the box (Sales SDR, Marketing Intern,
  Customer Support, etc.). Add `--refine` for an LLM-driven
  refinement pass on top of the deterministic floor.
- **A safety judge** built into Genesis: every action whose
  name contains a sensitive verb (`send`, `post`, `pay`,
  `publish`, `delete`, `share`, `message`) is automatically
  marked `requires_approval = true`, regardless of what the LLM
  proposes. The hard rule cannot be bypassed.
- **Two execution backends**:
  - `HumanExecutor` for human-carry-out actions (artifact
    bundles + `complete <id>` workflow).
  - `ComputerUseExecutor` driving an E2B Firecracker microVM
    + Anthropic computer-use loop for novel GUI work.
- **Auto-approval policy** with per-action / daily USD caps,
  time windows (with cross-midnight wrap), wake rules for
  urgent action types. Per-worker overrides via
  `argo worker update --auto-approval-json`.
- **Multi-channel notify** for approval gates: email
  (Postmark), Slack (Block Kit buttons), SMS (Twilio), push
  (Expo APNs/FCM), web inbox.
- **A worker-instance identity layer** — workers persist
  across sessions with stable ids, persona, role, memory
  namespace, channel preferences.
- **A 3-layer memory store** (recent / indexed / summary) with
  sqlite-vec + Voyage-3 embeddings.
- **A skill library** with trust grades (`builtin → tenant →
  published → learned-unreviewed`) and a learning loop that
  drafts SKILL.md proposals from successful CU trajectories.
- **An orchestrator** with Session + Handoff + EscalationChain
  primitives for multi-worker coordination.

This runbook walks you through using all of it.

---

## 2. Install

### 2.1 Prerequisites

| Requirement | Why |
|---|---|
| Rust 1.75+ | The CLI builds against the workspace's MSRV. |
| SQLite 3.40+ | The session store ships its own SQLite via `sqlx`; system version only matters if you want to inspect the DB with `sqlite3 .skycot/argo.db`. |
| `git` | Genesis writes a vertical tree — most operators commit it to a repo. |
| (optional) `ANTHROPIC_API_KEY` | Required for `argo chat`, `argo genesis create --refine`, and the Anthropic CU loop. |
| (optional) `OPENAI_API_KEY` | Alternate provider for chat. |
| (optional) `E2B_API_KEY` | Required for `ComputerUseExecutor` actions. |

### 2.2 From source

```bash
git clone https://github.com/mark-hallam/Argo-Agent skycot
cd skycot
cargo install --path crates/argo-cli
argo --version
```

Cargo installs the `argo` binary into `~/.cargo/bin/`. Verify it's on PATH:

```bash
which argo
# /Users/you/.cargo/bin/argo
```

### 2.3 Recommended directory layout

A workspace is just a directory with a `.skycot/` subdir. The
CLI auto-creates it on first run, but if you want a clean
layout up front:

```
my-workers/
├── .skycot/
│   ├── argo.db                 # Session store + identity + audit
│   ├── auto_approval.toml      # Workspace-level approval policy
│   └── outputs/                # Per-action artifact bundles
│       └── <job_id>/
│           └── actions/<action_id>/
├── verticals/
│   ├── marketing-intern-maya/
│   │   ├── vertical.toml
│   │   ├── worker.toml
│   │   ├── prompts/system.md
│   │   ├── prompts/persona.md
│   │   ├── memory/schema.toml
│   │   └── eval/seed.jsonl
│   └── sales-sdr-sam/
│       └── ...
└── manifests/
    └── executors/
        └── computer_use.toml   # CU profile registry
```

Genesis writes the per-vertical tree under `verticals/`.
Everything else under `.skycot/` is the runtime store.

### 2.4 Required env vars

The minimum viable workspace is **zero env vars** — the CLI's
read-side surface, Genesis (without `--refine`), and the human
executor all run offline.

For each capability you want:

| Capability | Env vars |
|---|---|
| `argo chat` (Anthropic) | `ANTHROPIC_API_KEY` |
| `argo chat` (OpenAI) | `OPENAI_API_KEY` |
| `argo genesis create --refine` | `ANTHROPIC_API_KEY` |
| `ComputerUseExecutor` | `E2B_API_KEY`, `ANTHROPIC_API_KEY` |
| Email approval channel | `POSTMARK_SERVER_TOKEN` |
| Slack approval channel | `SLACK_BOT_TOKEN`, `SLACK_SIGNING_SECRET` |
| SMS approval channel | `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, `TWILIO_FROM_NUMBER` |
| Push approval channel | `EXPO_ACCESS_TOKEN` |

Missing env vars surface at the **first call site that needs
them**, never at startup. So you can install + bootstrap
without any keys, then add them as you wire up channels.

---

## 3. Bootstrap

```bash
cd my-workers
argo doctor
```

`argo doctor` is the first thing to run in a fresh workspace.
It walks every loaded vertical, checks the action ledger
schema, verifies memory layers, and reports any per-vertical
load errors.

Sample output:

```
argo doctor — workspace report

verticals (0 loaded):
  (no verticals — run `argo genesis create` to make one)

session store: ok (.skycot/argo.db, 41 KB, schema v15)
identity store: ok (0 workers)
memory layers: ok (0 entries across recent/indexed/summary)
audit log: ok (0 rows)

🟢 ready
```

If you see `🔴 errors`, the report names the offending vertical
or schema column. Common fixes:

| Symptom | Fix |
|---|---|
| `vertical 'X' failed to load: schema mismatch` | Bump the vertical's `schema_version` to match `argo --version`'s. |
| `no [actions] section` | The vertical's `vertical.toml` is missing the `[actions]` table — add at least one action declaration. |
| `migration v15 not applied` | Older workspace; run any `argo` command and the runtime auto-migrates on first connect. |

### 3.1 Verifying providers

```bash
argo provider list
argo doctor --json | jq '.providers'
```

If you've set `ANTHROPIC_API_KEY` or `OPENAI_API_KEY`, they
show up here. If not, the list is empty (which is fine — you
just can't use chat / refine until you set one).

### 3.2 Verifying channels

Channels are registered via `manifests/channels/<name>.toml`
files in your workspace. The four shipped impls:

```bash
argo gateway routes --grep approve
```

This lists the gateway's HTTP endpoints that the channels
post decision callbacks to. If a channel has been configured
(toml + env vars), you'll see its decision route here.

---

## 4. Generate your first worker

```bash
argo genesis create \
  --charter "be my marketing intern, run weekly campaigns to cold leads" \
  --persona maya \
  --output verticals/marketing-intern-maya
```

What just happened:

1. **Archetype classifier** scored the charter against 12
   archetypes and picked `MarketingIntern`.
2. **Manifest writer** assembled the seed action vocabulary
   (`research_topic`, `draft_campaign`, `post_campaign`,
   `send_followup`).
3. **Safety judge** auto-fixed any sensitive-verb actions
   (`post_campaign`, `send_followup`) to `requires_approval =
   true`.
4. **Prompt writer** generated `prompts/system.md` +
   `prompts/persona.md` + `memory/schema.toml` +
   `eval/seed.jsonl`.
5. **Worker config** stamped `worker.toml` with `dry_run =
   true` and `dry_run_until = now + 24h` (the brief calls this
   the "training-wheels" window — every action lands `pending`
   for the first 24h regardless of policy).
6. **Round-trip validation** loaded the tree via
   `argo_vertical::load_dir` and confirmed it parses cleanly.

Inspect what it wrote:

```bash
tree verticals/marketing-intern-maya/
```

```
verticals/marketing-intern-maya/
├── vertical.toml
├── worker.toml
├── prompts/
│   ├── persona.md
│   └── system.md
├── memory/
│   └── schema.toml
└── eval/
    └── seed.jsonl
```

### 4.1 Concrete examples for each archetype

```bash
# Sales SDR
argo genesis create \
  --charter "be my outbound SDR, prospect 50 accounts a week, qualify on BANT" \
  --persona sam \
  --output verticals/sales-sdr-sam

# Customer Support Agent
argo genesis create \
  --charter "triage support tickets, draft responses, escalate P0s to me" \
  --persona ari \
  --output verticals/support-agent-ari

# Marketing Intern (above) — already covered

# Recruiter Screener
argo genesis create \
  --charter "screen inbound resumes, draft screening questions, schedule interviews with promising candidates" \
  --persona robin \
  --output verticals/recruiter-screener-robin

# Ops On-Call
argo genesis create \
  --charter "ack incidents from PagerDuty, draft status updates, page secondary if I'm unreachable" \
  --persona oscar \
  --output verticals/ops-on-call-oscar
```

### 4.2 With LLM refinement

If you have `ANTHROPIC_API_KEY` set, the deterministic floor
can be sharpened:

```bash
argo genesis create \
  --charter "be my marketing intern, run weekly campaigns to cold leads" \
  --persona maya \
  --output verticals/marketing-intern-maya \
  --refine
```

The `--refine` pass:

1. Runs the deterministic pipeline first (so you always get a
   known-good draft).
2. Calls Claude Sonnet with a strict JSON schema asking for:
   - Confirmation/revision of the archetype.
   - Up to 3 additional actions the seed vocab missed.
   - 5–10 lines of charter-specific context for the system
     prompt.
3. Re-runs the safety judge on every action the model proposes.
4. Falls back silently with a `tracing::warn!` if the model's
   response doesn't validate (so a transient LLM failure can't
   fail the whole `genesis create`).

Missing `ANTHROPIC_API_KEY` with `--refine` fails at startup
**before** the deterministic pipeline runs, so you don't end
up with a half-baked vertical:

```
$ argo genesis create --charter "..." --persona maya --output ./out --refine
argo: refinement validation failed at refine.config: ANTHROPIC_API_KEY must be set when --refine is passed
```

### 4.3 Walking the generated tree

#### `vertical.toml`

```toml
schema_version = "1"

[vertical]
id = "marketing-intern-maya"
name = "Marketing Intern (maya)"
version = "0.0.1"
description = "Generated by Genesis from charter: be my marketing intern, run weekly campaigns to cold leads"

[role]
system_prompt = "prompts/system.md"
persona = "prompts/persona.md"

[actions.research_topic]
risk = "low"
requires_approval = false
description = "Research a campaign topic"
executor = "human"

[actions.draft_campaign]
risk = "medium"
requires_approval = false
description = "Draft a campaign brief"
executor = "human"

[actions.post_campaign]
risk = "high"
requires_approval = true   # ← auto-fixed by safety judge
description = "Publish the campaign"
executor = "human"

[actions.send_followup]
risk = "high"
requires_approval = true   # ← auto-fixed by safety judge
description = "Send a follow-up message"
executor = "human"

[success_metrics]
deflection_target_pct = 70
median_response_seconds = 60
csat_threshold = 0.85
required_audit_chain_intact = true
```

#### `worker.toml`

```toml
worker_id = "marketing-intern-maya"
persona   = "maya"
dry_run   = true
dry_run_until = "2026-05-07T12:34:56+00:00"
dry_run_reason = "auto-set by Genesis at generation time; clears after 24h"
```

While `dry_run = true`, every action lands `pending` regardless
of the auto-approval policy. Flip the flag manually after you
review the manifest, or wait for the 24h timer.

#### `prompts/system.md` (excerpt)

```markdown
# System prompt — Marketing Intern worker `maya`

You are maya, a Marketing Intern working alongside the operator.

## Charter (operator's words)
be my marketing intern, run weekly campaigns to cold leads

## How to behave

- Plan before you act. Surface your plan to the operator before
  dispatching any action that touches the outside world.
- Every action you emit must match an entry in the manifest's
  `[actions]` vocabulary. Undeclared action_types will land
  `blocked` rather than executing.
- Sensitive actions (send, post, pay, publish, delete, share,
  message) ALWAYS require operator approval. Do not attempt to
  autonomously dispatch them.
- Cite evidence for every claim. The OS's evidence gate
  rejects emissions that lack a verifiable source.
```

#### `eval/seed.jsonl`

```jsonl
{"id":"marketing_intern_draft_then_post","prompt":"Plan a launch tweet for our new feature","expect_action_type":"draft_campaign"}
{"id":"marketing_intern_research_first","prompt":"Research what's trending in the dev-tools space","expect_action_type":"research_topic"}
```

The seed is intentionally minimal — operators expand the suite
once the worker has been live for a week and there's real
traffic.

### 4.4 Validating the result

```bash
argo vertical list
# verticals (1 loaded):
#   marketing-intern-maya v0.0.1

argo vertical show marketing-intern-maya
# (full manifest summary)

argo vertical lint marketing-intern-maya
# 🟢 vertical loads cleanly + safety judge passes
```

If `argo vertical lint` flags an issue, the message points at
the specific line in the manifest. Most common: a typo in an
action_type, or a `requires_approval = false` on an action
the safety judge wants `true`.

---

## 5. Configure approvals

By default, every action with `requires_approval = true` falls
through to a **human Ask** — there's no auto-approval policy
loaded in a fresh workspace. To turn on auto-approval:

### 5.1 Workspace-level policy

Create `.skycot/auto_approval.toml`:

```toml
enabled = true
max_usd_micros_per_action = 500_000        # $0.50 per action
daily_budget_usd_micros   = 50_000_000     # $50.00 per day
wake_action_types         = ["page_oncall"]

[[time_windows]]
days  = ["Mon", "Tue", "Wed", "Thu", "Fri"]
start = "09:00:00"
end   = "17:00:00"
```

Now low-risk approval-gated actions during weekday business
hours auto-approve up to the per-action and daily caps.
**High-risk actions never auto-approve** — the H-1 evaluator
hard-codes that, regardless of policy.

The 7 policy gates (in evaluation order):

1. `manifest:requires_approval = false` → always auto-approve
   (autonomous-safe action; policy doesn't apply)
2. `policy:disabled` → always Ask (no auto-approval)
3. `policy:high_risk_always_asks` → high-risk actions never
   auto-approve
4. `policy:per_action_cap=$X exceeded` → Ask
5. `policy:daily_cap=$X would be exceeded` → **Block** (the
   operator didn't agree to spend more today)
6. `policy:outside_time_window` → Ask (unless the action is
   in `wake_action_types`)
7. Otherwise → AutoApprove

### 5.2 Per-worker override

If one worker should be more permissive (or stricter) than
the workspace default:

```bash
cat > /tmp/maya-policy.json <<EOF
{
  "enabled": true,
  "max_usd_micros_per_action": 1000000,
  "daily_budget_usd_micros": 100000000
}
EOF

argo worker update <worker-id> --auto-approval-json /tmp/maya-policy.json
```

The runtime resolves at gate time:

1. Worker's own policy if `enabled = true` → use it
2. Else workspace `.skycot/auto_approval.toml` → use it
3. Else disabled default → Ask everything

### 5.3 Channel notify on Ask

When the gate decides Ask, the channel-notify path fires.
Configure workspace fan-out channels in `.skycot/notify.toml`:

```toml
[[channels]]
channel = "slack"
target = "#ops"
risk_level = "high"

[[channels]]
channel = "email"
target = "you@example.com"
```

Each channel reads env vars, with credentials.toml fallback via
`argo cred set`:

| Channel | Required env vars |
|---|---|
| `push` | `EXPO_ACCESS_TOKEN` plus the push-channel device registry |
| `webinbox` | optional `ARGO_WEBINBOX_PATH`; optional `ARGO_WEBINBOX_BEARER_TOKEN` for reads |
| `slack` | `SLACK_WEBHOOK_URL` |
| `email` | `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM` |
| `sms` | `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, `TWILIO_FROM` |
| `discord` | `DISCORD_WEBHOOK_URL` |
| `chat` | `CHAT_WEBHOOK_URL` |
| `sse` | `SSE_NOTIFY_URL` |

Missing required values produce errors like
`slack not configured: set SLACK_WEBHOOK_URL` in the notify
dispatch result and logs. Notify is best-effort: a channel
failure does not change the approval gate decision.

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

## 9. Audit and observability

### 9.1 `argo audit query`

```bash
# All actions in the last hour
argo audit query --since-minutes 60

# Only auto-approval decisions
argo audit query --kind auto_approval

# One worker's full trail
argo audit query --worker-id <id> --json | jq '.[]'

# Failed actions
argo audit query --kind action_failed --since-days 7
```

Every row has:

- `kind` — what happened (action_emitted, auto_approved,
  channel_queried, executed, completed, failed, handoff,
  skill_proposed, …)
- `at` — UTC timestamp
- `worker_id`, `action_id`, `session_id` — denormalised so
  you don't have to join
- `payload_json` — the full row at that moment

### 9.2 `argo doctor`

The same command you ran in §3 works as an ongoing health
check:

```bash
argo doctor --json | jq '.summary'
```

```json
{
  "verticals_loaded": 5,
  "workers_active": 3,
  "actions_pending": 12,
  "actions_blocked": 0,
  "memory_recent_rows": 847,
  "memory_indexed_rows": 1239,
  "memory_summary_rows": 41,
  "audit_rows_today": 312,
  "auto_approval_today": 28,
  "channel_notify_today": 4
}
```

### 9.3 Prometheus

When the gateway is running:

```bash
argo gateway serve --port 7676 &
curl http://localhost:7676/metrics | grep argo_
```

Metrics shipped:

- `argo_actions_emitted_total{vertical,kind}`
- `argo_auto_approval_decisions_total{decision}`
- `argo_handoffs_total{from,to,outcome}`
- `argo_evaluation_latency_seconds` (histogram)
- `argo_workers_active`
- `argo_memory_layer_rows{layer}`

Wire to whatever Prometheus + Grafana you already use. The
metrics names are stable — `argo_*` is the namespace.

### 9.3.1 OpenTelemetry / OTLP exporter

`argo-telemetry` ships an optional OpenTelemetry Protocol (OTLP)
exporter behind the `otlp` cargo feature (default off — single-
binary distribution stays zero-deps without it). When the feature
is on, the in-loop spans (`agent.turn`, `judge.adjudicate`, …) and
counters / histograms (`routing_decisions_total`,
`judge_duration_ms`, …) are routed to whatever OTel collector you
point them at.

Build with the feature on:

```bash
cargo build --release --features argo-telemetry/otlp
```

Then on process start, call `argo_telemetry::otlp_exporter::init()`
once and hold the returned `ShutdownGuard` for the lifetime of the
process — its `Drop` flushes any pending batches synchronously.

Two environment variables tune the exporter:

- `OTEL_EXPORTER_OTLP_ENDPOINT` — collector base URL. Default
  `http://localhost:4318` (OTLP/HTTP standard port). Spans POST to
  `<endpoint>/v1/traces`, metrics POST to `<endpoint>/v1/metrics`.
- `OTEL_SERVICE_NAME` — `service.name` resource attribute.
  Default `argo`.

Point at a collector by setting the endpoint:

```bash
# Local OTel collector / Datadog Agent / any OTLP/HTTP receiver
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318 argo gateway serve

# Hosted backend that accepts OTLP/HTTP (e.g. Honeycomb)
OTEL_EXPORTER_OTLP_ENDPOINT=https://api.honeycomb.io \
OTEL_SERVICE_NAME=argo-prod \
argo gateway serve
```

For backends that need auth headers (e.g. Honeycomb's
`X-Honeycomb-Team`), front the collector with a sidecar OTel
collector that adds the headers — first-party header config is a
follow-up item.

### 9.4 Live audit stream

For real-time monitoring (live ops dashboard, on-call):

```bash
# Subscribe to the gateway's audit stream — every audit row
# pushed as a WebSocket frame.
argo gateway tail --kind auto_approval,handoff
```

(Surface still landing as `claude-realtime-audit-stream`; the
existing `argo audit query --watch` flag is the today-state
substitute that polls every second.)

### 9.5 Coverage

CI measures Rust coverage on every PR and main push with
`cargo-llvm-cov`. The job uploads `lcov.info` and
`coverage-summary.txt`, comments the summary on PRs, and uploads
`coverage.svg` as an artifact on main pushes. No threshold is
enforced yet; this is a measurement baseline. The coverage job
ignores test-run failures while collecting coverage so known
pre-existing flakes do not suppress the report; the CI `test`
job remains the correctness gate.

To run the same report locally:

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
cargo llvm-cov --workspace --html
```

Open `target/llvm-cov/html/index.html` for the annotated report.

### 9.6 Common failure modes

A condensed copy of this table — plus the cli command index from
Appendix A — is rendered to a self-contained one-screen card at
[`quickref.html`](quickref.html). Regenerate with
`bash scripts/gen-runbook-quickref.sh`.

<!-- quickref:errors:start -->
| Symptom | Cause | Fix |
|---|---|---|
| `executor 'computer_use' is not yet implemented` | `E2B_API_KEY` or `ANTHROPIC_API_KEY` not set | Set them, or declare `executor = "human"` to fall back to the artifact-bundle path |
| `unknown executor kind 'browser'` | Manifest typo — only `human \| mcp \| computer_use \| api` are valid | Fix the action's `executor` field |
| `safety judge rejected the generated manifest` | An action with a sensitive verb is `requires_approval = false` | The auto-fix should have caught this; if you see it, the manifest was hand-edited after generation. Set `requires_approval = true` on the offending action. |
| `vertical-id 'marketing_intern' must be lowercase ASCII letters, digits, or '-'` | Underscores in the id | Use dashes: `marketing-intern-maya`. Genesis converts automatically; only hand-authored manifests hit this. |
| `output dir '...' is not empty` | Re-running `argo genesis create` against an existing dir | Use a fresh path or `rm -rf` the old one — Genesis refuses to clobber. |
| `action {id}: approval required (state pending, requires_approval=true)` | Trying to execute before approval | Run `argo worker actions approve <id>` first, or set up an auto-approval policy |
<!-- quickref:errors:end -->

---

## 10. Where to next

- **The mission, in full**: [MISSION.md](MISSION.md). Read this
  to understand the six load-bearing claims SKYCOT is built
  against.
- **The 14-day swarm protocol**:
  [../build-plan/PROTOCOL.md](../build-plan/PROTOCOL.md). How
  multiple agent families (Claude Code, Codex) built this
  repo concurrently in 14 days.
- **Active follow-ups**: see
  [BUILD_STATUS.md](../build-plan/BUILD_STATUS.md) for in-flight
  briefs. The v1.x roadmap items that didn't make the swarm:
  - Federated OCI marketplace for the skill library tier
  - `argo session start/list/show` cli (in flight as
    `H-orchestrator-cli`)
  - WebSocket audit stream (`claude-realtime-audit-stream`)
  - Per-vertical proposal thresholds (manifest field)
- **Contributing**: PRs welcome. The brief format + protocol
  are documented in `docs/build-plan/PROTOCOL_BRIEF_TEMPLATE.md`
  + `PROTOCOL.md`. Start with a small, self-contained slice;
  the swarm has shown that 100–400 LOC PRs are the right size.
- **Demo recording**: TODO — once the cli for
  `argo session start` lands, the v1 demo gate (the
  marketing-intern → sales-sdr → audit walkthrough) becomes
  scriptable and we'll attach the recording here.

---

## 11. Security

### 11.1 Gateway bearer-token auth

The gateway routes (session creation, message append, run turn,
skill install, memory writes, the read-side dashboards `/sessions`,
`/audit/recent`, `/messages/recent`, `/memory/*`, …) are
anonymously reachable by default — the only protection is the
`127.0.0.1:7400` localhost bind. For any non-localhost deployment,
set a bearer token:

```sh
export ARGO_GATEWAY_BEARER_TOKEN=$(openssl rand -hex 32)
argo gateway start
```

Once the env var is set (with a non-empty value), every POST
request and every non-allowlisted GET request must carry
`Authorization: Bearer <token>` or the gateway returns
`401 Unauthorized` with body
`{"error":"unauthorized","reason":"missing or invalid bearer token"}`.

**Public read-only allowlist** (always anonymous so liveness
probes, load balancers, and catalog-discovery flows keep working):

- `GET /` — root index
- `GET /health`, `/health/live`, `/health/ready`, `/health/full`
- `GET /version`
- `GET /routes`

Everything else (`/sessions`, `/audit/*` except `stream` is gated,
`/messages/*`, `/memory/*`, `/doctor`, `/db/*`, `/metrics/json`,
`/workspace/*`, `/search*`, every per-resource GET, every POST)
is protected.

**Integration-test escape hatch**: set `ARGO_GATEWAY_AUTH_BYPASS=1`
to bypass the check even when a token is configured. Use this in
local CI fixtures that need to exercise GET + POST routes without
threading the token through every test.

The token can also live in `<workspace>/.argo/credentials.toml`
under the same key — `argo_config::cred::get` reads env first
and falls back to that file, matching the precedent for
`ARGO_WEBINBOX_BEARER_TOKEN`.

---

## Appendix A — Cli command index

Every command shown above is in v1 main. Run `argo --help` for
the full tree, or `argo <subcommand> --help` for a specific
group. The 25 top-level commands are:

<!-- quickref:commands:start -->
```
argo new           argo resume        argo chat          argo steer
argo agents        argo gateway       argo doctor        argo health
argo db            argo workspace     argo audit         argo setup
argo vertical      argo genesis       argo messages      argo cron
argo work          argo worker        argo costs         argo eval
argo forge         argo judge         argo mcp           argo memory
argo model         argo plugin        argo provider      argo route
argo session       argo skill         argo trace         argo tui
argo tools
```
<!-- quickref:commands:end -->

`argo worker actions <list|show|approve|execute|complete|fail|block|retry>`
is the hot-path family for driving the action ledger. Genesis
+ doctor + worker actions + audit are the four families an
operator touches daily.

## Appendix B — File checklist for a customer demo

Before showing this to a customer, verify:

- [ ] `argo --version` returns a v1.x string
- [ ] `argo doctor` reports 🟢 ready
- [ ] `argo genesis create --charter "..."` writes a complete
      tree
- [ ] `argo vertical lint <id>` passes
- [ ] An emitted action goes pending → approved → executing →
      completed cleanly
- [ ] `argo audit query --since-minutes 5` shows the trail
- [ ] `argo doctor --json` includes a non-zero
      `auto_approval_today` if you've configured a policy
- [ ] (Optional) `cargo test -p argo-cli --test e2e_v1_demo`
      passes — this is the e2e gate that exercises the same
      paths from above against an in-memory store

If all eight tick, the platform really does what MISSION.md
promises.
