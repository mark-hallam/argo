# mission-memory-per-worker-scope — swap shared_team→worker for system-prompt memory injection (Phase B follow-up to #945)

**Owner**: claude. **Depends on**: `claude-session-worker-id-schema` MERGED (need session.worker_id available before we can resolve `MemoryNs::worker(...)` from a session).

## Why

#945 (mission-wire-memory-compose-context) shipped memory hydration into the agent system prompt with **workspace-shared scope only** — the explicit follow-up note in its merge ticket called this out:

> "v1 wire: When building the system prompt, look up `MemoryNs::shared_team(tenant_id)` (the workspace-shared scope that DOES work with the existing `SessionHeader`)... Per-worker memory becomes a follow-up."

With `claude-session-worker-id-schema` landed, sessions now carry a `worker_id`, so `MemoryNs::worker(tenant, worker_id, scope)` becomes resolvable. This brief flips the lookup.

## Scope

In `argo-agent-runner/turn.rs` (or the helper sibling created by #945 — confirm via grep), the memory hydration path currently does:

```rust
let memory_ns = MemoryNs::shared_team(session.tenant_id);
let memory_md = memory_ns.compose_context()?;
```

Swap to:

```rust
let memory_md = match session.worker_id {
    Some(worker_id) => {
        // Per-worker memory is the canonical path now.
        let ns = MemoryNs::worker(session.tenant_id, worker_id, MemoryScope::Personal);
        ns.compose_context().unwrap_or_default()
    }
    None => {
        // Legacy / non-vertical sessions fall back to the workspace-shared layer.
        // Preserves #945 behaviour for sessions created before this brief.
        let ns = MemoryNs::shared_team(session.tenant_id);
        ns.compose_context().unwrap_or_default()
    }
};
```

The fallback is critical — old sessions, ad-hoc REPL sessions, and any vertical that doesn't bind to a specific worker keep the workspace-shared behaviour.

## Approach (concrete)

1. Locate the call site: `grep -n 'shared_team' crates/argo-agent-runner/src/`. Per #945's merge note, the current site is in either `turn.rs` or the helper sibling `turn_memory.rs`.

2. Confirm `MemoryNs::worker(...)` constructor signature in `crates/argo-identity/src/types.rs` — it should already exist as a stub that #945 preserved. If signature drift requires changes, document in PR body.

3. Replace the unconditional `shared_team` call with the `match session.worker_id` shown above.

4. **Add a third memory scope: optional cross-layer composition.** Some operators will want both worker-scoped + workspace-shared memory rendered together (worker memory first, shared below). Add an env knob `ARGO_MEMORY_LAYERS=worker|shared|both` (default `worker` when worker_id present, `shared` otherwise). Section the `## Memory` block by source when `both` is selected.

5. Tests:
   - session with worker_id → worker-scope rows render
   - session without worker_id → shared-team rows render (legacy fallback)
   - `ARGO_MEMORY_LAYERS=both` → both blocks rendered, worker first
   - missing worker memory table → soft-fail tracing::warn (matches #945 pattern)

## Files you may touch

- `crates/argo-agent-runner/src/turn.rs` — narrowly: swap `shared_team` call (or its helper invocation)
- `crates/argo-agent-runner/src/turn_memory.rs` — narrowly: extend the helper if #945 placed the call there
- `crates/argo-agent-runner/src/turn_memory_tests.rs` — narrowly: extend or NEW sibling for the new branch coverage (≤ 200 LOC)
- `docs/skycot/RUNBOOK.md` — narrowly: §memory subsection note `ARGO_MEMORY_LAYERS` knob + per-worker-default behaviour

## Files you must NOT touch

- `MemoryNs` constructors in `argo-identity` — they exist; this brief just calls them
- `MemoryNs::compose_context()` impl in `argo-identity/src/types.rs` (#945) — no logic change needed there
- The `session` table or `SessionHeader` — that's `claude-session-worker-id-schema`'s domain (already MERGED by the time this brief is claimed)
- `argo-cli/src/handlers/gateway/routes_post_sessions.rs` — already has worker_id plumbing from #-schema

## Acceptance criteria

- [ ] Session with `worker_id = Some(...)` → `## Memory` block renders **worker-scope** rows
- [ ] Session with `worker_id = None` → `## Memory` block renders **workspace-shared** rows (back-compat)
- [ ] `ARGO_MEMORY_LAYERS=both` → renders both blocks, worker-scope first, with section headers `## Memory (worker)` and `## Memory (workspace-shared)`
- [ ] `ARGO_MEMORY_LAYERS=shared` → forces shared-team even with worker_id present
- [ ] `ARGO_MEMORY_LAYERS=worker` + no worker_id → empty memory block (no fallback in explicit mode)
- [ ] Missing per-worker memory table on stale DB → tracing::warn + empty block (graceful)
- [ ] 4 unit tests + reuse the integration smoke from #945 (extended to assert per-worker hydration)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-agent-runner` clean

## Branch + PR

- Branch: `claude-mission-memory-per-worker-scope`
- PR title: `runtime: per-worker memory scope for system prompt hydration (mission-memory-per-worker-scope)`

## Notes for the implementer

- This brief is ~30 LOC of logic + ~80 LOC of tests. The heavy lift was in #-schema. Keep the change surgical.
- Mission claim 1 (teammate, persistent identity) reaches ~95% delivered after this brief lands.
- The `ARGO_MEMORY_LAYERS=both` mode is the operator-friendly default for new deployments — document clearly in RUNBOOK that workspace-shared memory is still useful for "everybody on the team should know X" facts (org chart, vendor list, on-call schedule), while worker memory is "what this specific worker has learned about its role."
- Fail-soft semantics match #945: any error in memory composition logs warn-level + returns empty string. Memory never blocks conversation flow.
