# Mission load-bearing wire #4 — wire memory hydration into agent system prompt

**Owner**: claude. **Depends on**: F-memory-layering (#786), F-memory-shared-team (#819), claude-memory-tool-wire (#875) merged.

> SOURCE: deep-research findings tick 198, Section 1, item #5.
> Mission claim 1 ("teammate, not tool — persistent identity over
> time") is undelivered today because `MemoryNs::compose_context()`
> in argo-memory returns an empty string and the agent system prompt
> never sees layered memory.

## Scope

[`crates/argo-memory/src/types.rs`](crates/argo-memory/src/types.rs)
defines `MemoryNs::compose_context()` as a stub returning empty.
[`crates/argo-memory/src/lib.rs`](crates/argo-memory/src/lib.rs)
defines a real `MemoryStore::compose_context()` method that walks
recent + indexed + summary layers, but it is never called from
the agent runner.

[`crates/argo-agent-runner/src/turn.rs`](crates/argo-agent-runner/src/turn.rs)
defines `build_system_prompt(store, sid, ...)` which assembles the
per-turn system prompt from active skills + per-session attachments.
This brief plugs the memory layer in: after active skills, before
per-session attachments, inject the worker's layered memory context
for the current `(tenant, worker_id, scope)` triple.

After this lands, when Maya the worker is asked "what did we discuss
about Stripe last week?", her system prompt actually contains the
prior summary tier — mission claim 1 lights up.

## Approach

1. Replace `MemoryNs::compose_context()` stub with a real
   implementation that delegates to `MemoryStore::compose_context`
   for the namespace's `(tenant, worker_id, scope)` triple. The
   stub-vs-real split exists because `MemoryNs` was originally a
   pure-data type; promote it to one that holds an `Arc<MemoryStore>`
   handle (or accept the store as a parameter on the call —
   implementer's choice; document the decision).
2. In `build_system_prompt` in `argo-agent-runner/src/turn.rs`,
   resolve the worker's `MemoryNs` from the session, call
   `compose_context(...)`, append the result to the system prompt
   under a `## Memory` section.
3. Token budget: cap memory hydration at `ARGO_MEMORY_MAX_TOKENS`
   (default 4096); truncate older recent rows first, then summary
   tier, never trim active skill instructions.
4. Tests: assert that a worker with prior memory rows gets a
   non-empty Memory section; a fresh worker gets an empty (or
   absent) section; the cap is honored.

## Files you may touch

- `crates/argo-memory/src/types.rs` — narrowly: real
  `compose_context()` impl (or accepting a store parameter)
- `crates/argo-memory/src/lib.rs` — narrowly: re-export if needed;
  no API removals
- `crates/argo-memory/src/lib_tests.rs` — narrowly: add 3 new tests
  (empty namespace returns empty; populated namespace returns layered;
  cap honored)
- `crates/argo-agent-runner/src/turn.rs` — narrowly: add memory
  hydration call in `build_system_prompt`; behind an optional
  `memory_ns: Option<MemoryNs>` parameter so existing callers
  without identity don't break
- `crates/argo-agent-runner/src/turn_tests.rs` — narrowly: 2 new
  tests (with and without memory_ns)
- `crates/argo-cli/tests/memory_in_system_prompt_smoke.rs` (NEW) —
  integration smoke: spawn a worker, write 3 memory rows, run a
  turn, assert the system prompt sent to the provider contains
  expected memory snippets

## Files you must NOT touch

- The Voyage embedding stub (`embedding.rs`) — separate brief
  (covered in MISSION_BACKLOG.md as future work)
- The Haiku summary stub (`summary.rs`) — separate brief
- argo-skill, argo-judge — separate concerns

## Acceptance criteria

- [ ] `MemoryNs::compose_context()` returns real layered context
      (or the call site explicitly passes a store)
- [ ] `build_system_prompt` injects memory under `## Memory` section
      when `memory_ns` is provided
- [ ] Token cap (`ARGO_MEMORY_MAX_TOKENS`, default 4096) enforced
- [ ] Existing callers without identity continue to work unchanged
      (backwards-compatible)
- [ ] 5 new unit tests across argo-memory + argo-agent-runner
- [ ] 1 integration smoke verifying provider-bound prompt contains
      memory
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-memory -p argo-agent-runner` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-mission-wire-memory-compose-context`
- PR title: `memory: hydrate layered context into agent system prompt (mission-wire-memory-compose-context)`

## Notes for the implementer

- The Voyage embed is still a hash-stub; that is acceptable for v1
  — the recent + summary tiers still work without semantic search.
  A future brief will swap in the real Voyage call.
- If the `MemoryNs` → `MemoryStore` reach causes a circular dep,
  prefer the "pass store as parameter" form over the "hold Arc"
  form.
- The cap parameter exists to keep the system prompt finite when a
  worker has years of memory; don't over-engineer (no rope-style
  trimming).
- After this lands, mission claim 1 stops being "infrastructure
  exists" and starts being "loop is firing".
