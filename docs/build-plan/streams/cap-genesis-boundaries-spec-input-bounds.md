# cap-genesis-boundaries-spec-input-bounds

Authored against: origin/main 66192d1a7294ca57ca9b938a5e3e291941fd587e on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh cap-genesis-boundaries-spec-input-bounds

## Goal

Cap the Genesis BoundariesSpec inputs server-side (escalation-topic count +
per-topic string length) so an adversarial/huge spec can't amplify allocation
in policy synthesis.

## Why now (adversarial review of #2796, hardening note)

`policy_synth` iterates `escalation_topics` unbounded and `normalize_topic`
allocates per char. Bounded in practice only by the gateway body read + a single
file write (no OOM-grade amplification), but uncapped is imprudent for an
operator-supplied spec on the policy path.

## Fix

In the gateway handler (routes_genesis_stream / payload) or the synth entry,
reject (400) or truncate a spec whose escalation_topics exceeds a sane cap
(e.g. 64 topics) or whose per-topic length exceeds a cap (e.g. 200 chars).
Prefer explicit 400 with an honest message. Add a malformed/garbage-spec
negative test (the review noted serde #[serde(default)] handles it but there's
no explicit test).

## Acceptance / verifier

1. A spec with >cap topics (or an over-long topic) is rejected/truncated, not
   processed unbounded.
2. A malformed-JSON spec is handled gracefully (negative test).
3. cargo test -p argo-genesis + the gateway genesis route tests green.

## Size / owner

~20-40 LOC + tests. Owner: claude. 🟡 P2 SECURITY-HARDENING — input bounds on
the operator-supplied policy spec.
