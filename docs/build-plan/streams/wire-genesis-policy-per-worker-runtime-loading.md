# wire-genesis-policy-per-worker-runtime-loading

Authored against: origin/main 66192d1a7294ca57ca9b938a5e3e291941fd587e on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh wire-genesis-policy-per-worker-runtime-loading

## Goal

Make the truth-boundary policy that #2796 synthesizes from the Genesis
access-control panel ACTUALLY ENFORCE per-worker at runtime — auto-load each
worker's persisted policy at session boot, instead of the current
process-global `ARGO_TRUTH_BOUNDARY_POLICY_PATH`.

## Why now (adversarial review of #2796, honestly-disclosed scope gap)

#2796 synthesizes + persists a per-vertical `TruthBoundaryPolicy` TOML and
proves (round-trip test) it loads + Escalates via the canonical evaluator. BUT
per-worker RUNTIME auto-loading is DEFERRED — today the policy is the artifact a
future loader reads; the runtime only loads a process-global path. So the
operator's per-teammate access-controls are synthesized + stored but NOT yet
auto-applied at each worker's session. The feature is honest (NO THEATER,
disclosed) but not fully enforcing.

## Fix

At worker/session boot (the agent-runner turn path that builds the session's
TruthBoundaryPolicy), load the per-worker/per-vertical persisted policy
(the path #2796 writes next to the vertical) keyed on the session's
tenant_id + vertical_id (server-authoritative), and apply it to that session's
`evaluate_claim` enforcement. Fall back to the existing global path / no-policy
(Allow-by-default) only when no per-vertical policy exists. Do NOT take the
policy path from any client/spec field — derive it from the resolved tenant +
vertical.

## Acceptance / verifier

1. A Genesis-hired teammate whose synthesized policy gates a claim class
   (e.g. financial Escalate) actually Escalates that claim at runtime in a live
   agent turn — without setting ARGO_TRUTH_BOUNDARY_POLICY_PATH.
2. A teammate with no synthesized policy still runs (Allow-by-default preserved).
3. Cross-tenant: tenant A's worker never loads tenant B's policy.
4. cargo test for the agent-runner turn + truth-boundary path green.

## Size / owner

~60-150 LOC. Owner: claude. 🔴 P1 — completes the Genesis access-control feature
(makes the operator's guardrails actually bite per teammate); WS1/truth-boundary.
