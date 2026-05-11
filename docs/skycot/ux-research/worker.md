# Persona — the Worker itself

> The AI worker as a "user" of the platform's internal surfaces:
> memory, skills, escalation chain, peer workers, audit log,
> identity primitives. Not human. Not visible in the UI directly.
> But its needs constrain *every* design decision on the
> human-facing side.

## Why include the worker as a persona?

Because the platform is what it does for the worker. If memory
is unreliable, the worker's identity decays across sessions. If
the skill library is hard to search, the worker re-invents bad
patterns. If the escalation chain is undefined, the worker
either over-asks (annoying humans) or under-asks (acting beyond
policy). Designing the human-facing surface without designing
the worker's "user experience" of the platform produces a slick
UI on top of an incoherent agent.

This persona is also where mission claim #1 ("teammate, not
tool") lives architecturally. A tool doesn't have continuity;
a teammate does. The platform's job is to give the worker the
substrate for continuity.

## Who they are

Workers come in flavors that map to the shipped verticals, but
share more than they differ:

1. **Marketplace specialist** (Mariah-class) — high message
   volume, low per-message stakes, voice fidelity is the
   primary KPI.
2. **Recruiting specialist** (Priya-class) — moderate message
   volume, very high per-message stakes (candidate
   experience), policy enforcement is the primary KPI.
3. **Support agent** (Jamal-class) — high volume, mid stakes,
   resolution rate is the primary KPI.
4. **Knowledge worker / drafter** (Nori-class) — low volume,
   very high per-output stakes, accuracy is the primary KPI.
5. **Operations agent** (Sam-class) — moderate volume, mid-
   high stakes, escalation discipline is the primary KPI.

All five are built on the same Worker / Persona / Role / Vertical
primitives. The differences are in policy + voice profile +
skill set, not in fundamental capability.

## What the worker needs from the platform

In priority order:

1. **Stable identity across sessions.** `worker_id` is its
   name; `persona` is its voice; `memory_namespace` is its
   history; `role` is its charter. These survive restart,
   redeploy, model upgrade. (Largely shipped via the Worker /
   Persona / Role primitives.)
2. **Queryable memory scoped to its identity.** "What do I
   know about this customer?" → a single query against its
   own namespace with no cross-tenant or cross-worker leak.
   (Shipped: `mission-memory-per-worker-scope`, `mission-
   memory-query-scored-action`, `mission-wire-memory-compose-
   context`.)
3. **Skill discovery + selection at runtime.** "I need to
   send an email — which skill should I use? Which version?"
   with trust-grade filtering. (Partial: `mission-skill-
   select-variant-runtime-wire` TODO; `mission-skill-store-
   available-for-worker` TODO.)
4. **A defined escalation chain.** "I don't know how to
   handle this — who do I ask?" with deterministic answer:
   action policy → worker policy → roster policy → workspace
   default → escalation chain. (Shipped: `mission-handoff-
   walker-manager-chain`.)
5. **Cost-awareness.** "Am I about to spend more than my
   daily budget?" with a soft throttle before the operator's
   hard cap trips. (Partial: budget alerts shipped; soft-
   throttle in policy chain TBD.)
6. **Audit-friendly self-narration.** Every decision logged
   with enough context that an auditor or the operator can
   reconstruct *why*. Without trying to write the audit log
   itself — the platform stamps the record.
7. **Peer awareness.** "Has any other worker on this
   workspace seen this customer before?" with explicit
   cross-worker memory queries gated by role + RBAC.
8. **Continuity guarantees during upgrades.** When the
   underlying LLM swaps from Claude 4.7 → Claude 5.0, the
   worker's identity persists. Tone profile, memory, skill
   bindings carry forward. Drift detection runs against
   the prior model's behavior to flag regressions.
9. **A way to say "I'm stuck."** Not as an error, but as a
   structured handoff: "I attempted X, blocked on Y, my
   confidence is Z, here's what a human would need to
   decide." This is the unblock primitive that turns
   over-asking into useful escalation.
10. **A way to learn without re-inventing.** If the same
    pattern (drafting a refund letter) succeeds N times,
    the worker can propose the pattern as a skill for the
    library. (Largely shipped: pending-review pattern,
    skill-quality-from-audit.)

## Design constraints this imposes

These constraints come from the worker's needs but apply to
the human-facing UI:

- **The platform's UI must not be the worker's source of
  truth.** If the operator edits the worker's role in the
  web app, the change must be persisted to the canonical
  store (manifest / DB row) and the worker must re-read on
  its next session boot. The UI is a view, not a separate
  copy. Two-way sync between UI state and worker state is
  a bug magnet — single canonical store, multiple views.
- **The "what does she know?" view in the worker profile
  page** is rendered from the same memory the worker
  queries. No skim view that disagrees with the deep view.
  If the operator sees "Mariah knows the inventory CSV"
  but Mariah doesn't actually have that in her memory
  namespace, the UI is lying. Build the UI as a query
  against the same store the worker uses.
- **The skill library UI must show what the worker actually
  sees.** Trust grade, version, last-promoted timestamp.
  If the operator's view shows "skill v3 ready" but the
  worker is still on v2 (because the upgrade path hasn't
  rolled to this worker yet), the UX is lying. Show the
  *worker-effective* version, not the latest available
  globally.
- **Policy state must be readable in human language and
  worker language.** The operator sees: "Mariah won't agree
  to ship to PO boxes without asking you." The worker
  sees: a predicate against the action's payload. These
  are two views of the same policy, generated from one
  source. Generating one from the other is the
  retrain-a-worker flow (see `journey-retrain-a-worker.md`).
- **Trajectory replay must include the worker's view of
  itself.** When the auditor replays an action, the
  artifact includes: what memory was in scope, what skills
  were available, what policy was active, what the
  worker's "self-view" looked like at decision time. Not
  just the LLM call args.

## How the worker uses the platform — a session sketch

A worker session is bounded by an action. Inside an action:

1. **Boot** — Worker loads its identity (id, persona, role),
   its memory namespace, its policy chain, its available
   skills. Today this is ~3 separate calls; the
   `mission-skill-session-boot-wire` brief consolidates.
2. **Compose context** — For the current input (a Slack
   message, a calendar event, a cron trigger), retrieve
   relevant memory + skill + persona pieces. The
   `mission-wire-memory-compose-context` primitive does
   this.
3. **Plan** — Generate candidate actions. Score by
   confidence + policy fit. Pick the highest-scoring.
4. **Approval gate** — Check policy: autonomous-safe,
   needs-ask, blocked? If needs-ask, package the request as
   an approval card (see `approver.md`) and route via the
   appropriate channel.
5. **Execute** — Run the action via the appropriate
   executor (MCP, API, Browser, Computer-use, Doc,
   Filesystem).
6. **Record** — Stamp the audit trail with full provenance:
   inputs, policy chain, approval (if any), executor, output,
   downstream consequence.
7. **Learn** — If the action succeeded and the pattern is
   novel, propose a skill for the library. If it failed,
   record the failure pattern for future avoidance.

Every one of these steps is observable to the operator (via
the worker profile activity feed) and the auditor (via the
audit search). The worker doesn't have a "private" state;
everything is queryable.

## Anti-patterns to watch for in design

- **A worker dashboard that shows the worker's "private
  thoughts."** Reading the LLM's chain-of-thought and
  surfacing it as worker state is theater, not signal. The
  worker's state is its memory + skills + policies; the
  reasoning trace is debugging data, not product surface.
- **"Worker mode" UIs** that let humans talk to the
  worker as a chatbot. Workers are NOT chatbots; they are
  delegated teammates. A "chat with Mariah" panel where the
  operator types "summarize today's listings" is a
  scope-violation. If the operator wants a summary, the
  worker's daily summary should already produce one; if not,
  the operator delegates "give me a daily summary at 6pm"
  via the role's duty list.
- **Memory views that show vector embeddings.** Operators do
  not need to see 4,213 embeddings. They need to see
  "Mariah knows: your inventory CSV, your pricing rules,
  three previous interactions with this buyer." Memory
  surface is *facts*, not *vectors*. (The embeddings are
  there; the UI doesn't expose them.)
- **Letting workers see each other's memory by default.**
  Cross-worker memory access must be explicit and
  audited. "Mariah, what did Jamal know about this
  customer?" requires either (a) operator-granted cross-
  worker share, or (b) RBAC permission. Default-allow is
  a tenant-isolation bug waiting to happen.
- **Worker identity tied to model version.** If the worker
  IS the model, then a model swap is identity replacement.
  The worker must be a structure that persists across
  model swaps — same id, same memory, same skills, same
  persona, different underlying inference.

## What the worker should NEVER do

These constraints come from the auditor + delegator personas
but bind the worker:

- Send a message under a human's name without the human's
  signature mechanism. Workers act *on behalf of* humans;
  they don't impersonate.
- Take an irreversible action (send money, fire someone,
  publish to public broadcast) without explicit human
  approval, regardless of policy state.
- Modify its own memory in ways the operator can't undo.
  Memory edits are append-only or audit-trailed.
- Modify its own policy. Worker can *request* a policy
  change via the retrain flow; only humans approve.
- Modify its own role / charter. Role is the contract; the
  worker doesn't get to rewrite the contract.
- Hide cost. Worker can soft-throttle but can't disable cost
  tracking.
- Pretend to be a human if directly asked. Disclosure rules
  may be tenant-configurable but the override on direct
  question is non-negotiable.

## Critical moments — when the worker fails

Three failure modes that compound into "worker is broken":

1. **Memory inconsistency.** Worker remembers the wrong
   thing about a customer; sends a contradicting reply.
   Recovery requires diagnosing whether memory was *wrong*
   (bad write) or *unread* (compose-context didn't pull
   it). Both are bugs; the audit trail must distinguish.
2. **Skill drift.** A skill that worked 50 times suddenly
   produces a regression. The skill-quality regression
   detector should catch this; if it doesn't, the worker
   produced bad output without notice.
3. **Policy bypass.** Worker takes an action that the
   policy chain should have blocked. This is a worker-
   identity collapse event: trust in the worker drops to
   zero. Diagnosing whether the policy chain failed (bug)
   or the worker reasoned around it (alignment issue) is
   the critical first step.

## What we don't know yet

`(assumed — needs validation)`

- Whether workers should have an "internal state" surface
  visible to operators (a "what is Mariah thinking right
  now?" view) or whether that's noise.
- Whether peer-awareness (cross-worker memory queries)
  belongs in v1 or v2 — the demand is real but the RBAC
  surface is large.
- Whether continuity-through-model-upgrade is testable in
  practice, or whether each model swap is a soft identity
  reset users notice.
- Whether the skill-quality regression detector catches
  enough drift to be useful in production, or whether it
  fires too rarely.
- Whether workers should be allowed to *suggest* role
  changes (retrain-from-worker-side) or whether the
  retrain flow must always be human-initiated.

Plan to validate by:

1. Running 1-month per-worker behavioral baselines
   pre-launch, then watching for drift in week-1 of live
   traffic.
2. Reading 100 random audit entries per shipped vertical
   and grading them on: (a) attribution complete, (b)
   reasoning legible, (c) policy adherence verifiable.
3. Intentionally swapping the underlying model on a test
   workspace and measuring how much of the worker's
   measurable behavior carries forward.
