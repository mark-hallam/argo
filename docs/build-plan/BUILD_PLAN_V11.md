# Build plan beyond v1 — toward the SKYCOT mission

**Status as of tick 57:** v1 demo gate is GREEN (9/9). 58 PRs
merged. The platform shows a Genesis-output vertical going
through the full stack: action emission → auto-approval gate →
channel notify → executor → audit. **But that's the demo.
The mission is bigger.**

This document plans the work between v1 (a green e2e test) and
"AI workers as teammates that humans can delegate to."

## Mission recap (from MISSION.md)

> AI workers that humans can bring into their team as a real
> teammate, that can be delegated a responsibility/task/work/
> role; the platform can build an AI worker for it, draw upon
> a shared skills library, or even learn and build its own new
> skills; it has memory; it works autonomously and alongside
> humans; AI workers can fully autonomously do real-world work;
> that work is any task a human can do on a PC/Mac; approval
> requests via many different communication methods.

Six load-bearing claims:
1. **Real teammates** — operator delegates a role, not a task
2. **Platform-built workers** — Genesis from charter
3. **Shared skills library** — cross-tenant skill marketplace
4. **Self-learning skills** — workers grow new skills from
   successful trajectories
5. **Persistent memory** — workers remember sessions + share
   across the team
6. **Autonomous + multi-channel approvals** — workers act
   independently OR ask via the appropriate channel

## What v1 delivers

| Claim | v1 status |
|---|---|
| Real teammates | ⏳ Identity primitives (Worker/Persona/Role) ✓; teammate-feel UX ✗ |
| Platform-built workers | ✅ Genesis offline pipeline + 12 archetypes + LLM refinement (`--refine`) |
| Shared skills library | ✅ TrustGrade ladder + LayeredLookup + `vertical publish/pull` (OCI) |
| Self-learning skills | ✅ `should_propose` + `SkillCandidate::from_cu_trajectory` + `propose_for_review` |
| Persistent memory | ✅ 3-layer (recent/indexed/summary) + sqlite-vec/Voyage-3 + shared-team namespace |
| Autonomous + multi-channel | ✅ AutoApprovalPolicy gate + 5 NotifyChannels (email/slack/sms/push/webinbox) + escalation walker |

**Headline:** all six claims have a v1 implementation. The
remaining work is making each one **production-real** rather
than demo-real.

## The gap

A green e2e test is necessary but not sufficient. What's
between v1 and the mission:

### Gap 1: Reliability the operator can trust

A teammate the operator delegates to is a teammate they can
TRUST. Today the platform can do the right thing in a
controlled flow; it can't yet:
- Detect when a worker is going off-rails (anomaly detection)
- Pause a worker mid-flight when something looks wrong
  (circuit breaker)
- Require N humans for a high-risk action (multi-sig)
- Show the operator exactly what the worker SAW + DID
  (trajectory replay UI)
- Auto-pause if the operator goes inactive (dead-man switch)

### Gap 2: Capability breadth

The mission says "any task a human can do on a PC/Mac". v1
ships:
- ComputerUseExecutor (browser via E2B Anthropic CU)
- 5 MCP connectors (gmail, slack, gcal, hubspot, notion)

What's missing for "any PC/Mac task":
- Filesystem operations (read/write with sandboxing)
- Code execution (Python/JS/etc in isolation)
- Document creation (PDFs, slides, structured docs)
- More connectors (Stripe, Salesforce, Jira, Asana,
  Zendesk, Airtable, Notion DB, etc — there are 100+)
- Phone/voice (workspace memory mentions this on todo)
- Vision (OCR, image-understanding for screenshots)

### Gap 3: Operator visibility + control

A teammate's operator needs to SEE what's happening. v1 has:
- TUI overlays (sessions/handoffs/budget)
- `argo audit query/replay` for after-the-fact investigation
- `GET /audit/stream` snapshot

What's missing:
- Web UI dashboard (everyone has a browser; not everyone has
  a terminal)
- Real-time activity feed (what is each worker doing RIGHT
  NOW)
- Worker portrait pages (personality, history, current
  state)
- Mobile companion app (push-driven approvals are partial)

### Gap 4: Multi-worker orchestration

A team has multiple teammates. v1 has Session/Handoff/
EscalationChain primitives but no:
- Worker rosters (groups of workers with team-level policy)
- Workflow definitions (DAG of tasks across workers)
- Skill auctions (workers bid on tasks based on skill
  availability)
- Cross-worker context sharing patterns
- Round-robin task distribution

### Gap 5: Self-improvement

A teammate gets better over time. v1 ships the
SkillCandidate proposal flow but no:
- Skill version diffing
- A/B testing of skill versions
- Performance regression detection
- Operator-feedback channel (thumbs up/down per action)
- Trajectory training-data export (for fine-tuning)

### Gap 6: Org awareness

A teammate knows the team. v1 doesn't:
- Know the org chart
- Know who reports to whom
- Address workers/humans by name
- Route escalations through the chain of command
- Apply approval matrices (different actions, different
  approvers)

### Gap 7: Demo verticals beyond marketplace-seller

v1 ships ONE demo vertical (marketplace-seller). The
platform claims any role; we should prove that with at least
3 — sales SDR, customer support, marketing intern.

## Phased plan

### Phase v1.1 — Production hardening (next)

Goal: a customer can deploy this without 24/7 babysitting.

Briefs (mix of codex + claude, 6-8 in flight at once):
- claude-anomaly-detector (worker behavior anomaly)
- codex-circuit-breaker (auto-pause on N errors)
- codex-multi-step-approval (N-of-M signing)
- claude-trajectory-replay-ui (visual replay surface)
- claude-spend-anomaly-alerts (sudden burn rate)
- codex-compliance-export-cli (SOC2/GDPR-ready audit dumps)

### Phase v1.2 — Capability breadth

Goal: "any PC/Mac task" is closer to true.

- claude-filesystem-executor (sandboxed FS ops as new
  ExecutorKind)
- claude-code-execution-executor (Python/JS sandbox)
- claude-document-creation-executor (PDF/markdown/slides)
- codex-connector-stripe
- codex-connector-jira
- codex-connector-asana
- codex-connector-salesforce (private-app token + REST)
- codex-connector-zendesk

### Phase v1.3 — Multi-worker orchestration

Goal: teams of workers, not solo workers.

- claude-workflow-dag (declarative DAG of tasks)
- claude-worker-roster (groups of workers + team policy)
- claude-task-distribution (round-robin OR skill-auction)
- claude-checkpoint-resume (long workflows survive
  restarts)
- claude-org-awareness (org chart + addressable names)

### Phase v1.4 — Operator UX

Goal: a non-terminal operator can use this.

- claude-web-dashboard (browser UI; minimal first cut)
- codex-setup-wizard-cli (`argo setup` walks through
  install + first worker)
- codex-worker-portrait-page (web URL per worker)
- codex-activity-feed-stream (real-time worker activity)
- claude-voice-cli (voice-driven commands; precursor)

### Phase v2.0 — Self-improvement

Goal: workers actually get better over time.

- claude-skill-versioning-ab-test (A/B + regression
  detection)
- claude-trajectory-training-export (fine-tuning data
  pipeline)
- claude-operator-feedback-channel (thumbs up/down per
  action → reinforcement)
- claude-skill-quality-judge (per-skill quality score
  trend)

### Phase v2.1 — Governance

Goal: enterprise + regulated industry adoption possible.

- claude-policy-as-code (declarative org policies)
- claude-approval-matrices (different actions, different
  approvers — already partially via H-multi-step-approval)
- claude-data-classification (PII/PHI/financial tags +
  per-classification handling)
- codex-gdpr-delete-cli (right-to-be-forgotten)
- codex-audit-report-cli (compliance-grade reports)

### Phase v2.2 — Demo verticals

Goal: prove the platform works for >1 use case.

- codex-vertical-customer-support (full Customer Support
  Agent vertical)
- codex-vertical-sales-sdr (full Sales SDR vertical)
- codex-vertical-marketing-intern (full Marketing Intern
  vertical)
- codex-vertical-finance-ar (Accounts Receivable vertical)

## Sequencing rules

- **v1.1 first.** Reliability is the precondition for everything
  else; without trust, capability + breadth don't matter.
- **v1.2 + v1.3 in parallel.** Capability breadth (more
  executor kinds + more connectors) is independent of
  multi-worker orchestration. They're additive.
- **v1.4 is a long tail.** Operator UX work is large + ongoing.
  Start with the high-impact piece (web dashboard) early; the
  rest can land incrementally.
- **v2.0 + v2.1 require v1.1 stable.** Self-improvement +
  governance assume the production-reliability foundation is
  solid.
- **v2.2 (more demo verticals) is the customer signal.** Each
  new vertical proves the platform claim "ANY role" with one
  more data point.

## What to put in the queue NOW

This document is the strategic plan; the immediate-action
items are 12 briefs added to BUILD_STATUS.md as TODO rows
this tick. They span v1.1 reliability + v1.2 capability +
v2.2 demo verticals — the highest-impact-now slice of the
plan above.

The conductor's standing replenishment loop continues to
fire whenever either queue dips ≤3, drawing from this plan
+ recent PR-body deferrals + git-log direction.

## Stop condition

The conductor's stop condition stays as defined in the loop
prompt: *"every brief MERGED AND claude-e2e-integration-test
is green"*. As of tick 56, v1's e2e test is green; the
condition holds while new briefs land + their tests pass.

When the queue is empty AND no new replenishment ideas
remain (full mission delivered), append a `Plan complete`
note to BUILD_STATUS.md with the date + a link to the
production-deploy recording.

Until then, the loop keeps going.

---

## Tick 198 amendment — deep-research findings ingested

The operator commissioned a deep-research pass over the codebase
that produced ~280 actionable items. The full backlog is captured
in [MISSION_BACKLOG.md](MISSION_BACKLOG.md) (status-tagged,
cross-referenced to existing briefs/PRs). The headline framing
from that analysis:

- **Substrate (primitives): ~75% complete.** Architecture is sound.
- **Runtime (closed loops): ~45% complete.** Many wires are missing
  between substrate primitives that exist and the user-visible loops
  that prove the mission claims.
- **Mission spirit: ~45% delivered.**

The gap is enumerable, scoped, and almost entirely tractable —
no architectural dead-ends visible. The two structural risks
(no CI; no gateway auth) are 1-day and 1-week fixes respectively.
The strategic risk (others ship 19 messaging platforms; SKYCOT
has fewer in-tree adapters) is multi-month catch-up,
parallelizable with mission closure.

### Phase A — load-bearing wires + production perimeter (queued tick 198)

13 new `mission-*` briefs were added to BUILD_STATUS.md this tick.
They split into two groups:

**Load-bearing wires (8 items; one already in queue):**

| Brief | Owner | Cite | Closes |
|---|---|---|---|
| mission-wire-outbound-notify-on-ask | claude | item #1 | mission claim 6 (Slack/email/SMS approvals) |
| mission-wire-mcp-action-executor | claude | item #2 | unbricks marketplace-seller demo |
| mission-wire-api-action-executor | codex | item #3 | direct REST executor; mirror of mcp |
| codex-duty-scheduler-gateway-wiring | codex | item #4 | (already TODO in queue) mission claim "works autonomously" |
| mission-wire-memory-compose-context | claude | item #5 | mission claim 1 (persistent identity) |
| mission-wire-judge-on-skill-creation | claude | item #6 | mission claim 4 ("judge vetoes uncertain") |
| mission-wire-judge-on-forge-promote | claude | item #7 | enforces auto_promote_threshold 0.85 |
| mission-wire-feedback-in-cu-executor | codex | item #8 | operator votes flow into propose gate |

**Production perimeter (5 active + 1 deferred-to-last):**

| Brief | Owner | Status |
|---|---|---|
| mission-reenable-ci-workflows | codex | SHIP-BLOCKER |
| mission-gateway-bearer-auth | claude | SHIP-BLOCKER |
| mission-otlp-exporter | claude | active |
| mission-migration-reconciliation | claude | active (large refactor) |
| mission-coverage-tooling | codex | waits on CI re-enable |
| mission-headline-reconciliation | claude | DEFERRED — must be the LAST brief executed before declaring production-ready (numbers move weekly) |

### Phases B–F — drawn from MISSION_BACKLOG.md as queue drains

The conductor's Step-8 queue replenishment loop continues, now
drawing from MISSION_BACKLOG.md sections in priority order:

- Phase B (3–4 weeks): mission-claim closure (E2B sandbox image, CU
  variant update, subtask ledger, planner→dispatch, vertical depth
  catch-up to gtm-engineer parity)
- Phase C (4–6 weeks): production hardening tail (TLS, rate limiting,
  migration walk + upgrade tests, telemetry tail, compliance start)
- Phase D (parallel, ongoing): catch-up volume (more providers /
  skills / connectors / channels)
- Phase E (2–3 months): differentiate via self-improvement (Dreams
  closure)
- Phase F (3–6 months): scale-out (perf + horizontal scale + deeper
  vertical breadth)

The conductor should NOT author all backlog items as briefs at once.
Author 3–5 per replenishment tick per the standing Step-8 sizing,
preferring the next-phase items.

### Sequencing rationale

The deep-research analysis produced this sequencing
recommendation, paraphrased:

1. **Stop bleeding credibility first** (Phase A items: CI, headline
   honesty prep, gateway auth, the 8 load-bearing wires). 1 week of
   focused work converts substrate into a runtime the
   marketplace-seller demo can run end-to-end.
2. **Production perimeter** before any external customer (Phase A
   production blockers). Roughly a month, makes enterprise deployment
   possible.
3. **Mission claim closure** (Phase B). 3–4 weeks. Lights up the six
   mission claims as actually-deliverable, not just architecturally
   present.
4. **Volume parallelizes with everything else** (Phase D).
5. **Self-improvement is the differentiator** (Phase E). Dreams +
   judge integration are the genuinely novel wins; ship after the
   foundation is solid.

### Stop condition update

The original stop condition stays valid. The amendment adds:
*"AND every Phase A brief is MERGED"* as a soft pre-condition
before any external "production-ready" claim is made. The
mission-headline-reconciliation brief is the gating artifact —
do not author "production-ready" comms until it lands.
