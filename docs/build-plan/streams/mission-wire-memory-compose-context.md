# Mission Phase B wire — wire memory hydration into agent system prompt (rewrite v2, pragmatic-scope)

**Owner**: claude. **Depends on**: F-memory-layering (#786), F-memory-shared-team (#819), claude-memory-tool-wire (#875) — all MERGED.

> **REWRITE NOTE (tick 269 by conductor)**: original brief assumed
> `MemoryNs::compose_context()` lived at `crates/argo-memory/src/types.rs`
> but actual stub is at `crates/argo-identity/src/types.rs:133`. Original
> also required `(tenant, worker_id, scope)` triple but `SessionHeader` has
> only `(tenant_id, vertical_id)` — no `worker_id`. One tab BLOCKED.
>
> This rewrite uses the **pragmatic-scope** path: ship the wire with a
> **workspace-shared scope** as v1, with a clear follow-up brief for
> per-worker scope once the session schema can be extended.
> The wire stops being a stub. Operators get layered memory. Per-worker
> memory becomes a follow-up.

## Scope

`MemoryNs::compose_context()` returns an empty string. `build_system_prompt`
in `argo-agent-runner/turn.rs` never injects layered memory. Operators
running multi-session conversations get no continuity beyond what the
session itself preserves.

**v1 wire**: When building the system prompt, look up
`MemoryNs::shared_team(tenant_id)` (the workspace-shared scope that DOES
work with the existing `SessionHeader`), call `compose_context()` on it,
inject the result into the system prompt as a `## Memory` block.

**Per-worker memory** (the original brief's intent) requires extending
the session schema with `worker_id`. That's a separate brief, queued as
follow-up after this v1 lands.

## Approach (concrete)

1. **Implement `MemoryNs::shared_team(...).compose_context()`** at
   `crates/argo-identity/src/types.rs:133` (the stub):
   - Read top-N memory rows by recency (configurable via env
     `ARGO_MEMORY_COMPOSE_TOP_N`, default 8)
   - Return formatted markdown block:
     ```
     ## Memory (workspace-shared, last 8)
     - 2026-05-07: <content>
     - 2026-05-06: <content>
     ...
     ```
   - Return empty string if no rows (so build_system_prompt doesn't
     append an empty `## Memory` header)
2. **Modify `argo-agent-runner/turn.rs::build_system_prompt`**:
   - Take a `&SessionHeader` parameter (already in scope at most callers)
   - Look up `MemoryNs::shared_team(session.tenant_id)`
   - Call `compose_context()` and append to the system prompt if non-empty
3. **Soft-fail on memory store unavailable**: if `MemoryNs::shared_team`
   returns an error (e.g. table missing on stale DB), log a `tracing::warn!`
   and continue with no memory injection. Memory should never block
   conversation flow.
4. **Document the "workspace-shared v1" scope in the system prompt itself**
   so operators understand which memory bucket they're seeing.

## Files you may touch

- `crates/argo-identity/src/types.rs` — narrowly: implement the
  `compose_context()` stub at line 133 (~50 LOC)
- `crates/argo-agent-runner/src/turn.rs` — narrowly: 1 lookup + 3-line
  conditional append in `build_system_prompt` (~10 LOC)
- `crates/argo-identity/src/memory_compose_tests.rs` (NEW, ≤ 200 LOC) — 4 unit tests:
  - empty store → empty string
  - 8 rows → markdown block with all 8
  - 20 rows → top-8 most recent
  - missing table → empty string + tracing warn
- `crates/argo-cli/tests/memory_in_system_prompt_smoke.rs` (NEW, ≤ 100 LOC) —
  1 integration test: append a row via `argo memory append`, start a
  session, verify the system prompt sent to the model includes the row content.
- `docs/skycot/RUNBOOK.md` — narrowly: 1 paragraph noting the v1
  workspace-shared scope + the upcoming per-worker follow-up

## Files you must NOT touch

- `crates/argo-store/src/session.rs` or migrations — adding `worker_id`
  to session is a separate brief
- `argo-cli/src/handlers/sessions/*` — no schema changes
- `MemoryNs::worker(...)` constructor — preserve as-is for the follow-up brief

## Acceptance criteria

- [ ] `MemoryNs::shared_team(tenant).compose_context()` returns non-empty
      formatted markdown when memory rows exist
- [ ] `build_system_prompt` injects `## Memory` block when shared-team
      memory has rows
- [ ] Empty store → no `## Memory` block in system prompt (clean)
- [ ] Missing memory table → no crash, tracing warn, no `## Memory` block
- [ ] Top-N is configurable via `ARGO_MEMORY_COMPOSE_TOP_N` (default 8)
- [ ] 4 unit tests + 1 integration smoke
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-identity -p argo-agent-runner` clean
- [ ] RUNBOOK paragraph documents v1 workspace-shared scope + follow-up

## Follow-up brief (deferred)

After this lands, queue a separate brief:
- `claude-session-worker-id-schema` — extend `session` table with
  `worker_id` column + thread it through gateway POST `/sessions` +
  `argo session start --worker <id>` flag. Once that lands, a third brief
  can swap `MemoryNs::shared_team` to `MemoryNs::worker(tenant, worker_id, scope)`
  for proper per-worker memory.

## Branch + PR

- Branch: `claude-mission-wire-memory-compose-context`
- PR title: `runtime: hydrate workspace-shared memory into agent system prompt (mission-wire-memory-compose-context)`

## Notes for the implementer

- This is a **pragmatic v1 ship** — don't try to extend the session schema
  (out of may-touch). The point is to stop the stub returning empty and
  give operators something useful + observable.
- The `tracing::warn!` on missing-table is important — workspace-shared
  memory table is created on first use; stale DBs need a graceful path.
- Mission claim 1 ("teammate, not tool — persistent identity over time")
  goes from 0% delivered to ~50% with v1. The follow-up brief takes it to ~90%.
