# argo-runtime vs argo-agent-runner clarification

Date: 2026-05-25
Brief: `eval-argo-runtime-vs-agent-runner-clarify`
Verdict: clarified - `argo-runtime` is live shared substrate, not deprecated.

## Source gap

`docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` line 106 previously said:

> `argo-runtime` | All | NOT AUDITED | Relationship to argo-agent-runner unclear; may be deprecated or a wrapper. | P2 - clarify.

That ambiguity is resolved here.

## Responsibilities

`argo-runtime` owns store-free runtime primitives that multiple runtime callers compose: planner traits and template planning, decompose gates, context compression/truncation policy, action throttling, anomaly scoring, OOM restart decisions, worker resource cages, prompt-cache affinity, worker-pool elasticity, and Dreams plan-template distillation. It deliberately avoids `argo-store`, `argo-audit`, and `argo-agent-runner` dependencies so these policies can be reused by the CLI, orchestrator, store adapters, and the agent turn loop without creating workspace cycles.

`argo-agent-runner` owns the live multi-turn agent loop. It loads sessions from `SessionStore`, resolves providers and transports, applies freeze/budget/routing gates, builds system prompts, dispatches tools, records spend and audit rows, emits streaming events, and calls into `argo-runtime` when a turn needs shared policy or decomposition primitives. It is the product runtime path; `argo-runtime` is the upstream primitive library it consumes.

## Ownership Rules

A new safety primitive belongs in `argo-runtime` when it is pure policy or deterministic decision logic that can run without a session store, provider transport, audit sink, or tool dispatcher. If the primitive must read/write sessions, emit audit rows, inspect provider manifests, or gate actual tool execution in a live turn, the integration belongs in `argo-agent-runner` while any store-free policy core can remain in `argo-runtime`.

A per-turn budget cap belongs in `argo-agent-runner` for enforcement because the turn loop has the session, audit sink, and failure semantics. A pure allocator or decision helper can live in `argo-runtime` only if it stays store-free; the current `turn_budget` module in `argo-agent-runner` is the correct home for live enforcement.

A session-lifecycle hook belongs in `argo-agent-runner` because session start/end/freeze/routing events are turn-loop behavior with store and audit side effects. `argo-runtime` may expose helper types for policy calculation, but it should not own session lifecycle I/O.

## Public Surface Comparison

Public type-like inventory scan:

- `argo-runtime`: 50 public structs/enums/traits/types across planner, dispatch gate, context, throttle, anomaly, OOM restart, elasticity, prompt-cache affinity, and resource-cage modules.
- `argo-agent-runner`: 9 public structs/enums/types: `DecomposeOutcome`, `PlannerHook`, `RoundResp`, `RunEventSink`, `SpendCtx`, `TraceEvent`, `TraceEventKind`, `TraceHook`, and `TurnResult`.
- Duplicate public names between the two crates: none.

The conceptual pairs are adapters, not parallel surfaces. `PlannerHook` and `DecomposeOutcome` in `argo-agent-runner` adapt `argo-runtime::{Planner, DecomposeGate, SubtaskSink, record_decomposition}` into the turn loop; they do not reimplement planning. `ContextWindowPolicy` in `argo-agent-runner` maps config into `argo-runtime::context_compress::CompressionConfig` before calling runtime compression/truncation helpers.

## Evidence

Direct dependency direction:

- `crates/argo-runtime/Cargo.toml` depends on `argo-types`, `argo-workflow`, and utility crates only. It has no `argo-agent-runner`, `argo-store`, or `argo-audit` dependency.
- `crates/argo-agent-runner/Cargo.toml` depends on `argo-runtime`.

Static consumer evidence:

- `crates/argo-agent-runner/src/turn_planner.rs` consumes `record_decomposition`, `DecomposeGate`, `Planner`, `PlannerContext`, and `SubtaskSink`.
- `crates/argo-agent-runner/src/turn_helpers_context.rs` consumes `context_compress` and `context_truncate` to prepare provider context before a turn.
- `crates/argo-agent-runner/src/turn_context_policy.rs` converts config into `argo_runtime::context_compress::CompressionConfig`.
- `crates/argo-store/src/subtask.rs` implements `argo_runtime::SubtaskSink`.
- `crates/argo-store/src/session_store.rs` stores `PromptCacheAffinity`.
- `crates/argo-cli/src/handlers/plan_generate.rs`, `worker_actions_dispatch.rs`, `work_loop.rs`, and `worker_elasticity.rs` consume planner, throttle, anomaly, and elasticity primitives.
- `crates/argo-orchestrator/src/worker_supervisor.rs` consumes OOM restart and worker resource cage primitives.

## Pin Test

`crates/argo-runtime/tests/runtime_agent_runner_contract.rs` locks the relationship clarified here:

- `argo-runtime` must not depend on `argo-agent-runner`, `argo-store`, or `argo-audit`.
- `argo-agent-runner` must depend on `argo-runtime`.
- Planner/gate/sink primitives remain consumable as trait objects by the turn-loop adapter.
- Context truncation and prompt-cache affinity remain store-free runtime surfaces.

The broader `crates/argo-runtime/tests/surface_contract.rs` path requested by the brief is already occupied by open PR #2228 (`eval-argo-runtime-crate-surface`). This brief uses a separate relationship-focused test file to avoid a deterministic conflict while still pinning the intended contract.

## Conclusion

`argo-runtime` is not a deprecated wrapper. It is the upstream, store-free runtime substrate. `argo-agent-runner` is the downstream live turn orchestrator that binds those primitives to sessions, providers, tools, audit, spend, and streaming events. Future work should keep that dependency direction: pure reusable policy in `argo-runtime`; live turn/session side effects in `argo-agent-runner`.
