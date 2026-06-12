# pricing-byok-wire-routing-stage-transport

**Owner**: claude or codex — **P2 (BYOK completeness).**
**Depends on**: #2861 (BYOK per-tenant provider keys — landed `build_transport_for_tenant`).
**Authored against**: origin/main `c8b69418f` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh pricing-byok-wire-routing-stage-transport

## Problem (adversarial review of #2861, non-blocking finding)

#2861 wired `build_transport_for_tenant` (BYOK tenant-key precedence) into the
main `turn_run_loop` only. The other ~9 `build_transport` call sites still use
the process-global platform key, so BYOK precedence does NOT apply to them —
e.g. `turn_routing.rs:39` (the routing-stage transport), plus judge_eval,
forge_mutator, eval_skill_honesty, provider_distillation, ghostwriter,
orchestration_team, pitch_ask, genesis_stream.

Not a security regression (those paths just bill the platform key, not the
tenant's), but it means an Enterprise BYOK tenant's routing/judge/forge/etc.
LLM calls bill SKYCOT's platform account instead of their own — undermining
the zero-COGS margin model the BYOK tier promises.

## Fix
1. Wire `build_transport_for_tenant(..., allow_platform_fallback=true)` into
   `turn_routing.rs:39` so routing-stage calls honor BYOK precedence (fallback
   keeps non-BYOK tenants byte-identical).
2. Audit the remaining `build_transport` callers; for each tenant-attributable
   path, decide BYOK-vs-platform deliberately and convert or document why it
   stays platform (e.g. genesis/pitch may intentionally bill platform).
3. NOTE this composes with the SpendGate single-choke-point brief
   (`launch-wire-spendgate-llm-cost-ceiling`): both want every transport build
   to flow through one tenant-aware seam. Prefer coordinating so the tenant
   key resolution + the spend ceiling share the `build_transport` choke point
   rather than two parallel wrappers.

## Acceptance
- A BYOK tenant's routing-stage call resolves the tenant key (test pins
  `turn_routing` transport uses `build_transport_for_tenant`).
- An explicit, documented decision (convert or keep-platform) for each
  remaining `build_transport` caller.

~80-150 LOC. **🟡 P2 — BYOK only half-wired until routing-stage honors it.**
