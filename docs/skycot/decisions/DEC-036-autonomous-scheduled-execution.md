---
id: DEC-036
date: 2026-06-05
status: active
supersedes: none
amends: none
---

# DEC-036 — Autonomous scheduled execution, gated by the existing per-action auto-approval policy

## Decision

Operator greenlit (2026-06-05) turning **on** autonomous scheduled duty
execution — replacing today's blanket "deny every fire" fail-safe
(`crates/argo-cli/src/handlers/gateway/boot_spawns.rs:207` `ClosedGate`) with the
**real per-action auto-approval policy evaluator that already shipped**
(`argo_identity::auto_approval::evaluate`, brief `H-auto-approval` #796–805).

Greenlit as **direction**, built behind a **default-OFF** flag
(`ARGO_DUTY_AUTOFIRE`); flip to live only after Phase A ("measurable &
demonstrable") lands and the gate is verified. Reversible by flag.

## Why

Verified 2026-06-05 ground-truth audit: a hired teammate cannot run on its
schedule. Two compounding defects — (1) the gateway scheduler is wired
`ClosedGate` (denies every fire, `duty_scheduler_gate.rs:70`); (2) the hire
path writes the `duty` table while the scheduler reads `role_duty`, with no
bridge. The mechanism to do real work already exists (`GatewayDutyAgentRunner`
→ `argo_agent_runner::run_turn`) and the safety evaluator already exists; only
the wiring is missing. Autonomy is the difference between "drafts when I poke
it" and "works for me on a schedule." Flag-gating preserves the
[[DEC-025]]/[[DEC-029]] safety + truth posture until proven.

## How to apply

- **Bridge** (`uf-scheduler-hire-roleduty-bridge`): hire writes the scheduler's
  table (`role_duty`) so a teammate's cadence reaches the scheduler; reconcile
  the `tenant_id` TEXT-vs-BLOB difference between the two tables.
- **Gate** (`uf-scheduler-auto-approval-gate`): an `AutoApprovalDutyGate` adapts
  `auto_approval::evaluate` (worker policy + fresh spend snapshot + a
  `PendingAction` from the duty) → `FireDecision`. An `Ask` decision routes to
  the approvals inbox, NOT a silent fire.
- **Default OFF**; flip per-tenant/global only after verification. The 24h
  probation ([[DEC-025]] #2) still forces approval on everything during probation.
- **Truth-bounded** ([[DEC-029]]): "armed" ≠ "verified firing" — require
  current-turn evidence (a fired-session audit row / a real `run_turn` tail)
  before claiming it runs.

Relates: [[DEC-025]] (#2 probation, #9 tenancy), [[DEC-029]] (truth-bound),
`H-auto-approval` (#796–805). Phase B of
`docs/skycot/plans/USEFULNESS-FLOW-IMPLEMENTATION-PLAN-2026-06-05.md`.
