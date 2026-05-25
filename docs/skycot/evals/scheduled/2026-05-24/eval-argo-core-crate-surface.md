# `argo-core` crate surface audit

**Date**: 2026-05-24
**Brief**: `eval-argo-core-crate-surface`
**Mission link**: Shared core substrate — pins the walking-skeleton `Agent` plus the resume / steer / iteration-policy primitives that `argo-config::RestartPolicy` (and any future runner refactor) depend on.
**Verdict**: Surface is coherent and now pinned by an active contract test. **No new `argo-core` substrate fix-brief is needed.** One non-blocking observation about the crate's role is captured under "Findings" below; it is the kind of follow-up that belongs in a conductor housekeeping sweep, not in a substrate fix-brief.

## Public surface inventory

`crates/argo-core/src/lib.rs` exposes 17 names across 5 modules:

| Slice | Re-exports |
|---|---|
| `agent` | `Agent`, `AgentDeps`, `RunTurnOptions`, `TurnOutcome` |
| `audit` (re-exported from `argo-audit`) | `AuditKind`, `AuditSink` |
| `iter_policy` | `decide_iteration_extension`, `ExtensionDecision`, `ExtensionLimits`, `IterationOnMax` |
| `resume` | `apply_restart_policy`, `mark_session_discarded`, `scan_interrupted`, `InterruptedSession`, `RestartAction`, `RestartPolicy` |
| `steer` | `SteerChannel` |

The new contract test at `crates/argo-core/tests/surface_contract.rs` pins all 17 with a compile-fence + count sentinel.

## Test coverage added

`surface_contract.rs` adds active coverage for:

| Contract | Pin |
|---|---|
| Re-export inventory | Compile-fence imports plus `inventory.len() == 17` sentinel + explicit `fn`-pointer coercions for the two free functions. |
| Iteration policy (§6.6, Amendment 9) | `IterationOnMax::default() == Halt`; `Halt` always halts; `PromptUser` proposes `new_max_if_approved`; `AutoContinue` extends by `extension_size`; the hard ceiling (`extensions_used >= max_extensions`) overrides every policy including `AutoContinue`; snake_case serde tags for `[loop.on_max]` config parsing. |
| Resume policy (§6.5, Amendment 13) | `RestartPolicy::default() == PromptUser` (no silent auto-replay on daemon boot); snake_case serde tags; `apply_restart_policy` is total and routes each session by variant; `RestartAction` round-trips via serde with the `kind` discriminator; `scan_interrupted` returns empty on a fresh store; `mark_session_discarded` is idempotent on a missing session. |
| Steer channel (§6.3, Amendment 14) | Per-session FIFO drains exactly once; depth resets to 0 after drain; session slots are isolated. |
| `Agent` / `AgentDeps` shape | `AgentDeps` and `Agent` are `Send + Sync` (Tokio-runtime safe); `Agent::new` signature is stable; `RunTurnOptions` and `TurnOutcome` keep struct-literal constructibility (no required-field drift). |
| Audit re-export | `AuditSink::new(Arc<SessionStore>) -> AuditSink` and every `AuditKind` variant the loop currently writes (`TurnStart`, `TurnEnd`, `AssistantMessage`, `ToolCall`, `ToolResult`, `Halt`, `SteerEnqueued`) are pinned via compile-fence. |

The test is intentionally I/O-light: only `SessionStore::open(":memory:")` is touched (already used by the in-crate tests under `src/tests*.rs`), and no real provider transport is built.

## Production consumers

Static grep of the workspace finds exactly **one external crate** that depends on `argo-core`, and it consumes a single symbol:

| Consumer | Symbols | Assessment |
|---|---|---|
| `argo-config` (`sections_core.rs`, `tests.rs`, `lib.rs`) | `RestartPolicy` | Lives `[loop.on_restart]` config parsing. Stable. |

No workspace crate imports `argo_core::Agent`, `AgentDeps`, `RunTurnOptions`, `TurnOutcome`, `SteerChannel`, `scan_interrupted`, `mark_session_discarded`, `apply_restart_policy`, `decide_iteration_extension`, `ExtensionDecision`, `ExtensionLimits`, `IterationOnMax`, `InterruptedSession`, or `RestartAction`. The active agent loop lives in `argo-agent-runner`; the active orchestrator lives in `argo-orchestrator`. Neither depends on `argo-core`.

This matches the lib.rs docstring's framing — the crate is the §6 Phase-1 walking skeleton that ships as a reference / contract surface, not the production runtime path. The substrate is correct; the spec invariants it pins (default-Halt iteration policy, default-PromptUser restart policy, prompt-cache-safe steer drain) are exactly the ones a future runtime refactor that re-adopts these primitives must honor.

## Findings

**No `argo-core` substrate fix is needed.** The five sub-systems compile, are internally tested, and now have a public-surface contract test pinning their shape.

One observation worth surfacing for a future conductor housekeeping sweep (not a substrate brief, not blocking v1.0):

- **Walking-skeleton role is no longer documented in the README ledger.** The single external consumer of `argo-core` is `argo-config::RestartPolicy`. If a reader picks up `crates/argo-core/src/lib.rs` cold, the doc comment correctly says "§6 Phase 1 walking skeleton" — but there is no in-tree pointer from `BUILD_STATUS.md` / `RESUME.md` indicating that `argo-agent-runner` is the production agent loop today and `argo-core` is contract-pinned substrate. This is a docs-only nit; the substrate is still correct and useful (the contract test pins resume + iter-policy + steer semantics that the production runner can adopt or copy from). Anyone authoring a brief that needs to extend agent-loop behavior should grep both crates before adding a parallel surface — that is the standard "grep before write" rule from `DRIFT_PREVENTION.md`, not a new finding.

## Follow-up briefs

None required.

## Verification

Required command:

```bash
cargo test -p argo-core --tests
```

The PR body should report the final output from that command plus the cross-crate `scripts/workspace-clippy-fast-fail.sh` gate.
