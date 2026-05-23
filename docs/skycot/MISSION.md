# SKYCOT mission

> **Project rename — Argo → Skycot.** The project will rename from Argo to **Skycot** (domain: skycot.com). Continue using `argo` everywhere for now in code, binaries, crates, env vars, `~/.argo/`, `argo.toml`, and existing identifiers. Do not start renaming proactively. The user-facing product name is Skycot; the developer-facing binary stays `argo` until a coordinated rename pass. This matches the canonical statement in [`CLAUDE.md`](../../CLAUDE.md) §"Project rename".

The persistent ground-truth of what we are building. Every agent
working on this repo (Claude, Codex, future agent families) reads
this file before doing anything. Architecture decisions, PR
priorities, and product framing must trace back to one of the six
load-bearing claims below. If a proposed change does not advance
the mission, do not ship it.

---

## The mission

> AI workers that humans can bring into their team as a real
> teammate, that can be delegated a responsibility / task / work /
> role; the platform can build an AI worker for it, draw upon a
> shared skills library, or even learn and build its own new
> skills; it has memory; it works autonomously and alongside
> humans; AI workers can fully autonomously do real-world work;
> that work is any task a human can do on a PC/Mac; approval
> requests can flow via many different communication methods.

The above paragraph is the mission statement, verbatim from the
operator. The six load-bearing claims it makes are:

1. **Teammate, not tool.** A worker has persistent identity,
   voice, memory, role-membership over time. You don't run it;
   you delegate to it.
2. **Delegation is the unit.** "Be my marketing intern" or
   "handle inbound recruiter messages" is the contract — not
   "run this one job." A worker has duties, KPIs, and a charter.
3. **The platform builds workers.** Operator describes a role in
   plain English; SKYCOT generates the vertical (system prompt,
   action vocabulary, memory schema, eval seeds). No hand-
   authoring TOML.
4. **Skills are shared and learned.** A library workers draw
   from, plus a learning loop where successful novel sequences
   become reusable skills (extends the existing pending-review
   pattern). Cross-tenant sharing with provenance + trust.
5. **Real-world execution = any task a human can do on a PC.**
   That requires two execution backends: **MCP** (well-known
   APIs / tools) AND **computer use** (Anthropic-style desktop
   control with screenshots + mouse + keyboard for novel GUIs
   and unstructured tasks). Workers must have BOTH.
6. **Multi-channel approval gates.** Email, Slack, SMS, push,
   voice, web. The current CLI-only surface is a v0 placeholder,
   not the product.

---

## Strengthening items

These two strengtheners are not new claims — they make existing
architectural moats explicit so the swarm and the marketing both
treat them as first-class.

### Hash-chained audit as evidence

Every worker action is hash-chained into `audit_record`. Each
row's `self_hash` is `SHA-256(prior_row_hash || canonical_content)`,
so any tampering between writes is detectable. Verifiable months
later via `argo audit verify`. SOC 2-ready forensic trail **by
construction** — not a separate logging system bolted on. The
audit chain is the load-bearing evidence layer the enterprise
buyer is paying for.

### Multi-worker org structure

Workers form an **org**, not a fleet. Cohorts group teammates by
department / skill family / trust grade. Escalation chains route
work through named participants. Handoffs pass control with
payload between workers (or to a human). Auctions let multiple
bidders compete for delegated tasks. Supervisors manage worker
subprocess lifecycles. The substrate is `argo-orchestrator`; the
surface is `/cohorts`. This is the multi-worker org moat —
none of the competitors (Zapier, n8n, Crew.ai, Lindy) ship it.

---

## What "LIVE" means per claim

A claim is **LIVE** only when a falsifiable test that exercises
the whole end-to-end loop passes. The eval rounds score each
claim against these criteria; the conductor queues fix-briefs
for anything that's HALF-LIVE or OFF. Aspirational claims with
no test are explicitly NOT LIVE.

| Claim | LIVE means |
|---|---|
| **1 LIVE** | 5 distinct workers persist across gateway restarts, with memory, and one of them can be re-summoned by name 24 hours later. |
| **2 LIVE** | A hired worker's duty cron fires AND invokes its agent runner AND writes a `WorkerDutyFired` audit row within 60 seconds. |
| **3 LIVE** | `argo genesis plan "be my marketing intern" maya && argo workflow run` produces real output, not just files on disk. |
| **4 LIVE** | (option 4A locked; option 4B deferred to v1.x) `argo skill install <pack>` writes the skill to the workspace's skill library, a worker invokes it through a workflow step, and a `SkillInvoked` audit row is written carrying the skill's provenance (source pack + version + author). The closed-loop "learned" half (approve-novel-sequence → new skill → cross-tenant install + trust ratchet) is deferred to v1.x. |
| **5 LIVE** | Two halves — **both must be green**. (i) **MCP**: a workflow step of kind `tool_call` against a registered MCP server's tool routes through `McpConnectionPool` keyed by `(tenant_id, name)`, passes the capability check, returns a result, and writes an `AuditKind::ToolCall` row with tenant_id + action_type + status. (ii) **Computer-use (macOS)**: with `ARGO_NATIVE_MACOS_INTEGRATION=1`, `tests/notes_smoke.rs` opens Notes, types a marker string via osascript, reads it back, asserts the marker round-trips, audit row carries the screenshot artifact id. Linux + Windows computer-use executors are DEFERRED-v1.x per operator memory; v1.0 LIVE for claim 5 is macOS-only. |
| **6 LIVE** | Three sub-tests — **all three must be green**. (a) **Outbound** — a worker triggers an approval-gated action; email + Slack fire the request with an HMAC-signed approval token; approving advances the action and writes `ApprovalGranted`, declining writes `ApprovalDeclined`. (b) **Cross-tenant** — tenant_b cannot see tenant_a's pending approvals via any API (`GET /approvals`, `GET /approvals/bundles`, `POST /audit/export`); attempting to decide on an approval id from another tenant returns 404, not 403, per the existence-leak policy. (c) **Audit-chain** (round-15 strengthener) — `argo audit verify` returns `OK ({N} rows verified)` on a clean DB; after tampering with one row's `payload_json`, it returns `FAIL (row {id} hash mismatch)`. |

Strengtheners get their own LIVE criteria as they mature.

> **Provenance note**: this "Strengthening items" subsection +
> "What 'LIVE' means per claim" subsection were added on
> 2026-05-23 with direct operator sign-off in the evaluator
> round-15 session. Claims 1/2/3 LIVE criteria locked in
> round 15. Claims 4/5/6 LIVE criteria locked in round 18
> (also 2026-05-23, same direct operator sign-off pattern).
> Mission claims 1-6 themselves are unchanged. Future
> amendments to either subsection require the same operator
> sign-off pattern.

---

## What "real teammate" means architecturally

A teammate is not a clone of a vertical; it is an **instance with
identity**. Concretely:

- **Worker** — the instance the operator delegates to. Has a
  `worker_id`, a `persona`, a `role`, a `memory_namespace`, an
  `auto_approval_policy`, channel preferences. Persists across
  sessions / restarts / days / weeks.
- **Persona** — the public-facing voice. Name, tone, signature
  phrases, avatar, bio, attribution to the operator who built
  it.
- **Role** — what the worker does. A charter (free-text), a list
  of duties (each with a cron + entry action), KPIs, an
  escalation path. Role is reusable across personas.
- **Vertical** — the template. Action vocabulary, classification,
  retry policies, instruction templates. A vertical is to a
  worker what a class is to an object.

A vertical is the design-time artifact; a worker is the runtime
artifact. Today the codebase conflates them (a "session" inherits
the vertical's prompt; there is no per-worker identity layer).
Building the Worker / Persona / Role primitives is Stream E in
the 14-day plan.

---

## What "any task a human can do on a PC/Mac" means

Two execution backends, used per-action by the worker:

- **MCP executor** — wraps the existing `argo-mcp` client.
  Calls a registered MCP server's tool. Used for well-known
  APIs and structured operations (gmail send, slack post,
  calendar create-event, github open-issue). Deterministic,
  fast, audit-friendly.
- **Computer-use executor** — runs the Anthropic computer-use
  loop inside an `argo-sandbox` (E2B Firecracker microVM).
  Sees a desktop, takes screenshots, emits mouse + keyboard.
  Used for novel GUIs without APIs (Facebook Marketplace,
  Gumtree, internal SaaS dashboards), unstructured visual work
  (extract data from a PDF preview), or anything where "look at
  the screen and decide" is the only path.

Per-action manifest declares which executor the action uses.
Verticals authored by humans pick deliberately; verticals
generated by Genesis pick from a heuristic table (well-known APIs
→ mcp; novel UIs → computer_use; ambiguous → human as the safe
fallback).

A worker that lacks computer use is bounded to "what we have an
MCP for." A worker that lacks MCP is forced to drive every API
through a browser screen. Both backends together = the worker
can do any human-on-a-PC task.

---

## What "approval through many channels" means

The approval gate is the load-bearing safety primitive. Today it
fires only via CLI / HTTP gateway. To be a real teammate the
worker must reach the operator wherever the operator is —
phone, Slack, email, push notification, voice call.

`NotifyChannel` is a registered trait. Each channel impl handles
both outbound (request approval) and (where applicable) inbound
(receive the operator's reply). Per-action manifest declares
which channels at which risk level; per-operator preferences
override (quiet hours, urgent-only thresholds). HMAC-signed
approval tokens settle idempotently — first reply wins, the rest
get a "resolved by X" follow-up.

The approval gate itself is unchanged. Only the surface that
notifies the operator + collects the reply is multi-channel.

---

## What we are NOT building

- **Not a marketing-only product.** GTM-Engineer was the first
  proof vertical because marketplace selling has clear approval
  gates and real money. Future verticals (recruiter, support
  triage, ops on-call, research analyst, calendar coordinator)
  share zero domain code.
- **Not a marketplace.** OCI-distributed skills happen on top of
  the Worker OS, not as the product itself.
- **Not a CRM.** A Worker can talk to your CRM via MCP. It is
  not a CRM.
- **Not a chat product.** Workers can chat with humans (Slack,
  email reply), but the product is the autonomous loop, not the
  chat surface.
- **Not a prompt-engineering tool.** Workers are generated from
  delegation specs by Genesis; operators describe the role in
  English, not in prompts.

---

## Status: substrate vs runtime (snapshot 2026-05-07)

The mission paragraph above is the **target**. The current state
of the codebase against that target was assessed in a deep-research
pass at tick 198 (~280 actionable items captured in
[docs/build-plan/MISSION_BACKLOG.md](../build-plan/MISSION_BACKLOG.md));
the framing below is refreshed against the May Phase A swarm
that closed most of the named runtime gaps.

- **Substrate (primitives): ~85% complete** (was 75%). The
  architecture is sound. Worker / Persona / Role / Vertical
  types exist; capability engine real; multi-tenant by
  construction; MCP wire end-to-end; computer-use loop landed;
  8 channel implementations real with real backends; judge
  crate real; A/B + skill quality real; audit chain
  hash-verified; OCI distribution real. Phase A added gateway
  bearer-token auth at the dispatch layer (#946), the OTLP
  exporter (#938), and CI / coverage tooling (#930 / #942).
- **Runtime (closed loops): ~70% complete** (was 45%). Most
  wires the tick-198 capture flagged as missing have landed:
  - Judge is consulted by skill creation (#936, fail-OPEN) AND
    forge promote (#940, fail-CLOSED — promotion blast-radius
    is larger).
  - Duty scheduler is spawned in gateway boot (#939) — the
    autonomous loop foundation is live.
  - Layered memory composition is injected into the agent
    system prompt (#945, workspace-shared scope; per-worker is
    queued).
  - Outbound notify channels are called when the auto-approval
    gate returns `Decision::Ask` (#931).
  - Action-level McpExecutor wired (#933) — per-action manifest
    `executor.kind=mcp` declarations now reach the connector.
    ApiExecutor remains BLOCKED on schema-vs-acceptance
    mismatch (mission-wire-api-action-executor).
- **Mission spirit: ~65% delivered** (was 45%). Each of the
  six load-bearing claims now has a v1 closed loop the
  operator can observe end-to-end. Gaps remaining are:
  per-Principal audit-row stamping on the new bearer-auth
  gate; per-worker memory scope (workspace-shared shipped, the
  schema extension is queued); ApiExecutor unblocked; the
  Dreams cohort distillation pipeline (V2.0 Integration B,
  IN_REVIEW #948) hooked into a nightly trajectory walker.

The gap is enumerable, scoped, and almost entirely tractable.
There are no architectural dead-ends visible. The work is
partitioned in MISSION_BACKLOG.md across nine sections.

This section is refreshed each time a headline reconciliation
brief lands (next refresh trigger: when the V2.0 Dreams plan-
template cascade lands or when ApiExecutor unblocks). It does
not change the mission; it makes the gap between mission and
current reality visible to every agent reading this file.

---

## v1 success criteria

We hit v1 launch when:

1. **A generated worker runs autonomously over a non-CLI
   channel.** Operator says "build me a marketing intern named
   Maya"; Genesis emits the vertical; Maya runs duties on a
   schedule, asks for approval via Slack, gets it via Slack
   reply, posts a draft to Marketplace via computer-use, and
   logs the outcome — all without the operator opening a
   terminal.
2. **A learned skill propagates across workers.** Maya learns a
   "post-listing follow-up template" through repetition with
   operator approval; a different worker (e.g., a sales follow-
   up worker) discovers and reuses that skill from the shared
   library.
3. **Multi-worker handoff works end-to-end.** Worker A
   completes its duty, hands off to Worker B with shared
   memory; B continues without re-asking the operator the same
   questions A asked.
4. **Auto-approval under a budget works.** Operator sets "auto-
   approve up to $5 per low-risk action, max $50/day, only
   during weekday business hours"; worker observes the policy;
   no money spent outside the policy.
5. **Audit trail is complete.** Every action has a screenshot
   (computer-use), an evidence chain (MCP + research), and an
   approver (human or auto-approval policy). No silent steps.

The 14-day build plan in `docs/build-plan/BUILD_PLAN_2WK.md`
sequences the work that gets us to these criteria.

---

## How agents stay aligned

- Read `CLAUDE.md` for project conventions (Rust workspace, LOC
  budget, dev-safe-test workflow).
- Read this file (`MISSION.md`) for what we are building and why.
- Read `docs/build-plan/BUILD_PLAN_2WK.md` for the active 14-day
  plan.
- Read `docs/build-plan/CONTRIBUTING_AGENT.md` for coordination
  rules (file boundaries, branch naming, BUILD_STATUS.md
  protocol).
- For your specific task, read the brief at
  `docs/build-plan/streams/<brief-id>.md`.

If a brief and this file disagree, this file wins. If anything
in any of the above contradicts a load-bearing claim above,
escalate to the human operator before coding.
