# Persona — the Worker itself (stub)

> The AI worker as a "user" of the platform's internal surfaces:
> memory, skills, escalation chain, peer workers. Not human. Not
> visible in the UI directly. But its needs constrain the
> system's design.

## Why include the worker as a persona?

Because the platform is what it does for the worker. If memory
is unreliable, the worker's identity decays across sessions. If
the skill library is hard to search, the worker re-invents bad
patterns. If the escalation chain is undefined, the worker
either over-asks (annoying the human) or under-asks (acting
beyond its policy). Designing the human-facing surface without
designing the worker's "user experience" of the platform
produces a slick UI on top of an incoherent agent.

## What the worker needs

1. **Stable identity across sessions.** `worker_id` is its name;
   `persona` is its voice; `memory_namespace` is its history;
   `role` is its charter. These survive restart, redeploy,
   model upgrade. (Largely shipped: Worker / Persona / Role
   primitives merged.)
2. **Queryable memory scoped to its identity.** "What do I know
   about this customer?" → a single query against its own
   namespace with no cross-tenant or cross-worker leak.
   (Shipped: `mission-memory-per-worker-scope`, `mission-memory-
   query-scored-action`, `mission-wire-memory-compose-context`.)
3. **Skill discovery + selection at runtime.** "I need to send
   an email — which skill should I use? Which version?" with
   trust-grade filtering. (Partial: `mission-skill-select-variant-
   runtime-wire` TODO; `mission-skill-store-available-for-worker`
   TODO.)
4. **A defined escalation chain.** "I don't know how to handle
   this — who do I ask?" with deterministic answer: action
   policy → worker policy → roster policy → workspace default.
   (Shipped: `mission-handoffwalker-manager-chain`.)
5. **Cost-awareness.** "Am I about to spend more than my daily
   budget?" with a soft throttle before the operator's hard cap
   trips. (Partial: budget alerts shipped; soft-throttle in
   policy chain TBD.)
6. **Audit-friendly self-narration.** Every decision logged
   with enough context that an auditor or the operator can
   reconstruct *why*. Without trying to write the audit log
   itself.

## Design constraints this imposes

- **The platform's UI must not be the worker's source of
  truth.** If the operator edits the worker's role in the web
  app, the change must be persisted to the canonical store
  (the manifest / DB row) and the worker must re-read on its
  next session boot. The UI is a view, not a separate copy.
- **The "what does she know?" view in the worker profile
  page** is rendered from the same memory the worker queries.
  No skim view that disagrees with the deep view.
- **The skill library UI must show what the worker actually
  sees.** Trust grade, version, last-promoted timestamp. If the
  operator's view shows "skill v3 ready" but the worker is
  still on v2, the UX is lying.

## Status

Stub. The bulk of the worker's "user experience" is the
backend invariants, not a UI surface. Expanded into a real doc
only if a design question forces it — e.g. "what does the
operator see when the worker disagrees with the skill library
about which version to use?"
