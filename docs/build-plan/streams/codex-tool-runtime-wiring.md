# codex-tool-runtime-wiring — Wire Memory/DelegateTask/SessionSearch/SendMessage end-to-end from agent runner

**Owner**: codex. **Depends on**: nothing (but operationally complements F-004 codex-toolcontext-tenant).
**Authored against**: origin/main `25821ea8` on `2026-05-15`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-tool-runtime-wiring`.
**Source**: F-009 in `docs/skycot/audit/2026-05-15-codex-full-architecture-audit.md`. Operator reframing 2026-05-15: don't hide unwired tools via registry split — WIRE them.

## Scope

P2 correctness. Several in-core tools are registered with `Default::default()` impls that ERROR at dispatch time with "not wired":

- `Memory::default()` → `"memory store not wired"` (`crates/argo-tool/src/tools/memory.rs:206`)
- `DelegateTask::default()` → errors without an orchestrator (`crates/argo-tool/src/tools/delegate_task.rs:143`)
- `SessionSearch::default()` → `"session store not wired"` (`crates/argo-tool/src/tools/session_search.rs:139`)
- ~~`SendMessage::default()` → unwired dispatcher~~ **SCOPED OUT 2026-05-16:** codex pre-claim found there is NO production `MessageDispatcher` in argo-tool/argo-agent-runner; only `SendMessage::with_dispatcher(Arc<dyn MessageDispatcher>)` exists. Wiring SendMessage requires first BUILDING the production dispatcher (touches argo-channels + AgentDeps extension), which is out of scope for this brief. SendMessage stays unwired in this brief; a follow-up `codex-message-dispatcher-production-wiring` brief will build the dispatcher + wire SendMessage.

The agents see these tools advertised (they're in `builtin_tools()` at `mod.rs:63`), try to call them, and get errors. The fix is NOT to hide them but to WIRE them: pass the runtime deps (`MemoryStore`, `OrchestratorStore`, `SessionStore`, channel dispatcher) from the agent runner's session context into the registry constructor.

The infrastructure already exists:
- `builtin_tools_with_memory(store, workspace_id)` already wires Memory (see comment at `mod.rs:81`).
- The agent runner has access to these stores via `AgentDeps`.
- The MISSING piece is: the canonical builtin-tools constructor that wires ALL 4 unwired tools from the agent-runner side.

Scope (revised 2026-05-16 — 3 tools, not 4):
- Add a `builtin_tools_with_runtime(deps: &AgentDeps) -> Vec<Arc<dyn Tool>>` (or equivalent — name it whatever's idiomatic) in `argo-tool/src/tools/mod.rs` that takes Memory + DelegateTask + SessionSearch stores and wires them. (SendMessage stays unwired — see follow-up note above.)
- Update the agent runner in `argo-agent-runner/src/turn.rs` to call the new constructor instead of bare `builtin_tools()`.
- Make bare `builtin_tools()` `#[cfg(test)]`-only OR mark `#[deprecated]` with a doc-comment pointing at the new constructor.
- Update production callers of `builtin_tools()` (run `rg 'builtin_tools\(\)' crates --type rust -l` to identify) to use the wired variant.
- Tests prove: an agent run that calls Memory/DelegateTask/SessionSearch/SendMessage actually dispatches (no "not wired" error).

LOC: ~280.

## Files you may touch

- `crates/argo-tool/src/tools/mod.rs` — add `builtin_tools_with_runtime` constructor; gate or deprecate bare `builtin_tools`. ≤80 modified lines.
- `crates/argo-tool/src/tools/memory.rs` — narrowly: confirm `Memory::with_store` exists and is reachable from the new constructor. ≤15 modified lines.
- `crates/argo-tool/src/tools/delegate_task.rs` — narrowly: confirm a wired constructor exists; add one if not. ≤25 modified lines.
- `crates/argo-tool/src/tools/session_search.rs` — narrowly: confirm wired constructor exists. ≤15 modified lines.
- `crates/argo-tool/src/tools/send_message.rs` — narrowly: confirm wired constructor exists. ≤25 modified lines.
- `crates/argo-agent-runner/src/turn.rs` — update tool registry construction to use the wired variant; pass deps from session context. ≤40 modified lines.
- `crates/argo-agent-runner/src/dispatch.rs` — narrowly: if dispatch builds the registry, switch to the wired variant. ≤30 modified lines.
- Test sibling: integration test in `argo-tool/tests/` proving wired dispatch end-to-end. ≤80 LOC.

## Files you must NOT touch

- The Tool trait itself in `argo-tool/src/tool.rs` — no signature changes.
- Store implementations (`MemoryStore`, `OrchestratorStore`, `SessionStore` internals) — those already exist; this brief just plumbs them through.
- F-008's invariant doc change — separate brief (`codex-claude-md-tool-count-invariant-update`).
- Anything outside the may-touch list above.

## Acceptance criteria

- [ ] `builtin_tools_with_runtime(deps)` (or equivalent) exists and wires the 3 in-scope tools (Memory + DelegateTask + SessionSearch). SendMessage remains unwired — covered by follow-up brief `codex-message-dispatcher-production-wiring`.
- [ ] Agent runner uses the wired constructor; verify with `rg 'builtin_tools\(\)' crates --type rust` returning only test hits.
- [ ] Integration test proves Memory/DelegateTask/SessionSearch/SendMessage dispatch SUCCEEDS (no "not wired" error) when invoked via the agent runner.
- [ ] Bare `builtin_tools()` is `#[cfg(test)]` or `#[deprecated]`.
- [ ] `cargo test -p argo-tool -p argo-agent-runner` passes.
- [ ] `cargo clippy -p argo-tool -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-tool-runtime-wiring`
- PR title: `feat(tool): wire Memory/DelegateTask/SessionSearch/SendMessage end-to-end from agent runner (codex-tool-runtime-wiring)`

## Notes for the implementer

- Operator reframing 2026-05-15: the audit's F-009 proposed splitting the registry to HIDE unwired tools. Operator's better fix: WIRE them. This brief is the wire-up.
- If `AgentDeps` (or equivalent) already aggregates the 4 stores/dispatcher, the constructor signature is simple. If not, take whatever's cleanest.
- The session-tenant-id work (F-004 `codex-toolcontext-tenant`) is operationally adjacent — both brief affect what flows from session into tools. If F-004 lands first, the wired constructor here can also pass the real tenant. If F-004 hasn't landed, leave tenant plumbing alone (F-004's brief handles that).
- After this brief lands, agents will SEE the tools advertised AND have them work. The previous "advertise but error" behavior becomes pure success.
