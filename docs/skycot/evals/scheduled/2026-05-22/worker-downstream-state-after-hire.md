# Eval: Teammates — worker downstream state after hire (mission-claim 1 verification)

**Date**: 2026-05-22 (round 2)
**Pillar**: Teammate creation
**Mission claim**: 1 — Teammate, not tool · 2 — Delegation is the unit
**Verdict**: 🟢 **PASS (with v1-deferred-by-design gaps surfaced)** — hire writes real persona / role / duty / worker rows; trust grade / channels / KPIs intentionally deferred to post-hire flows.

## Setup

Round-2 follow-up to the round-1 5-teammate hire scenario. That round confirmed `POST /api/v1/workers` returns 201 with distinct UUIDs and the persona names show up on `GET /api/v1/team`. This round asks: **does hire ACTUALLY wire up the downstream subsystems**, or is it just persisting a few rows of metadata?

Anti-theater question: when a SKYCOT operator hires "Mariah", does the underlying worker have

1. a real persona row (name, voice preamble, builder attribution),
2. a real role row (charter, escalation),
3. real duty rows (cron, entry action),
4. a memory namespace bound to the worker_id,
5. trust grade defaulted to 0,
6. channels set up,
7. KPIs derived?

## Action

The live gateway (PID 28198) died sometime during this eval session — `:7400` no longer accepts connections, `~/.argo/gateway.pid` was removed, `gateway.log` shows a failed restart (`./target/release/argo: No such file`). That's a separate finding (see [scenario-9-gateway-died-mid-eval.md](#)). For this scenario I pivoted to:

```bash
cargo test -p argo-identity --lib store_inline_tests
```

The 6 inline tests cover the full hire → persist roundtrip + the rejection paths.

## Expected

All 6 tests pass + I can read from source whether downstream subsystems are actually wired.

## Observed

```
running 6 tests
test store::store_inline_tests::create_worker_from_draft_rejects_unknown_policy_level ... ok
test store::store_inline_tests::create_worker_from_draft_requires_one_enabled_duty ... ok
test store::store_inline_tests::create_worker_from_draft_rejects_short_charter ... ok
test store::store_inline_tests::create_worker_from_draft_aggressive_enables_auto_approval ... ok
test store::store_inline_tests::create_worker_from_draft_persists_persona_role_duties ... ok
test store::store_inline_tests::create_get_list_roundtrip ... ok

test result: ok. 6 passed; 0 failed; 0 ignored
```

✅ All 6 pass. Real persistence verified.

## Anti-theater check (source-read)

`assemble_worker_from_draft` at [crates/argo-identity/src/store_hire.rs:151](crates/argo-identity/src/store_hire.rs#L151) constructs:

| Field | Source | Real? |
|---|---|---|
| `persona.name` | `draft.suggestedName` | ✅ |
| `persona.voice_profile.preamble` | `draft.voice_profile` OR auto: `"You are {name}, an autonomous SKYCOT worker."` | ✅ |
| `persona.builder_attribution` | `principal_id` (the hiring human) | ✅ |
| `role.charter` | `draft.charter` (validated ≥ 20 chars) | ✅ |
| `role.duties` | `draft.duties` **filtered by `default_enabled=true`** | ✅ |
| `role.escalation` | `format!("approval_policy_level={}", draft.approval_policy.level)` | ⚠ (just a string tag; no real escalation path bound) |
| `memory_namespace` | `MemoryNs::new(tenant_id, worker_id, MemoryScope::WorkerSelf)` | ✅ (typed correctly) |
| `auto_approval.enabled` | `draft.approval_policy.level == "aggressive"` | ✅ |
| `circuit_state` | `CircuitState::Closed` (hardcoded) | ✅ |
| `channels` | `Vec::new()` (empty) | 🟡 **v1-deferred** |
| `kpis` | `None` (empty) | 🟡 **v1-deferred** |
| `level` | `WorkerLevel::Entry` (hardcoded) | ✅ |
| `trust_grade` | `0` (hardcoded) | ✅ (correct — trust ratchets up post-hire) |

`create_worker` at [crates/argo-identity/src/store.rs](crates/argo-identity/src/store.rs) wraps it in a single SQL transaction that writes to four tables:

1. `INSERT INTO persona (id, worker_id, tenant_id, name, voice_preamble, avatar, bio, builder_attribution, provenance_chain_json) VALUES (...)`
2. `INSERT INTO role (...)`
3. `INSERT INTO worker (...)`
4. `INSERT INTO duty (...)` — one per duty (no insert if all duties are `default_enabled=false`, but `validate_hire_draft` already rejects that)

All four writes commit atomically. Verified by the passing `create_worker_from_draft_persists_persona_role_duties` test.

## Gap analysis — what hire does NOT do today

🟡 **Channels are not bound at hire time.** Per `assemble_worker_from_draft:201`, `channels: Vec::new()`. The mission's "approval gates flow via many different communication methods" (claim 6) is opt-in post-hire. The SPA hire wizard (Step 5) doesn't ask which channels the worker should use — the operator wires them up later via `PATCH /api/v1/workers/<id>/channels`. **This is v1 by design** but worth knowing.

🟡 **KPIs are not auto-derived.** Per `Role { ... kpis: Vec::new() }` and `Worker { ... kpis: None }`. The Mariah card mockup (`/team`, persona detail page) shows "3 listings live (+33%) · 11 buyer replies · 2 flagged" — those numbers can only render after `worker_kpi` rows are populated through the duty-execution audit trail. At hire time the worker has zero KPIs and the SPA would render placeholders. Operator should treat the Mariah-card mockup as "what it looks like after a few days of operation", not "immediately after hire".

🟡 **Memory is namespaced but not seeded.** `memory_namespace` is set, but the hire flow doesn't `INSERT INTO worker_memory` or analogous. The worker's first turn allocates memory rows. Anti-claim-1 concern: does a "real teammate" remember anything on day 1? Per code, no. **By design** — onboarding tasks teach the worker.

🟢 **Trust grade defaults to 0 (entry-level).** This is correct per the trust-ratchet design — successful executions + zero RBAC denies advance it. Skill marketplace (`/skills` page) and the trust-grade chip (#1979) read this and display accordingly. Confirmed by source-read of `crates/argo-trust-engine/` (not exhaustively probed this round).

## Round-2 update on the live gateway tenant

Cannot verify because the live gateway is dead (separate finding). The 7 EVAL workers from round 1 are still in `~/.argo/sessions.db` (file unchanged), but I'd need a working `:7400` to query their downstream state via `GET /api/v1/workers/<id>/overview`, `…/memory`, `…/skills`, `…/trust-grade`. That endpoint shape is documented in [`routes_dispatch_get_api.rs:200-280`](crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api.rs) but unreachable right now.

## Gap / follow-up brief candidates

**🟡 P2 — `mission-hire-channel-bind-wizard-step`**: Extend the hire wizard with a "How does this worker reach you?" step that defaults to operator's existing channel preferences, so a fresh hire has at least one channel bound. Without this, the first auto-approval-ask from any new worker has no destination — the audit row writes but the operator never hears about it. **Operator-decisional** whether to fix in v1 or v1.x.

**🟢 INFO — Mariah-card visual-vs-reality gap**: The `/team` brand-loud mockup shows live KPIs + animated counters, but a worker hired in the last hour has none of those. A 1-line empty-state copy line on the worker card ("New hire — onboarding metrics will appear after the first duty run") would set the right expectation. Not a brief candidate; a marketing copy decision.

## What this scenario proves

✅ **Mission claim 1** ("Teammate, not tool"): the hire endpoint creates persona + role + duty + worker rows in a real ACID transaction. The worker has a persistent identity that survives gateway restarts. **Not theater.**

🟡 **Mission claim 2** ("Delegation is the unit"): partial. The worker has a charter + duties at hire, but the duties don't fire until the duty scheduler crons them — and the duty scheduler boot wiring landed in #939 per the operator's MISSION.md notes. Need a separate scenario to verify the duty scheduler actually pulls a hired worker's duties into its cron rotation. Future-eval candidate.

## State

- 6/6 argo-identity hire tests pass against current `main`.
- The 7 EVAL workers from round 1 are still in `~/.argo/sessions.db` (DB file is 3.3MB, untouched).
- Live gateway dead — see scenario 9.
