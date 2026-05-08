# Stream B — per-worker memory scope for system prompt hydration

**Owner**: codex.
**Depends on**: `claude-session-worker-id-schema` MERGED.
**Authored against**: origin/main `69b929c8` on `2026-05-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-memory-per-worker-scope`.
**MVP lane**: closes mission claim 1's worker-specific memory gap.

## Scope

#945 wired memory into the agent system prompt, but it reads only
`shared_team_memory`. Current main has the real worker-memory storage path:
`argo_memory::MemoryStore::compose_context(&MemoryNs, action_id)` over
`memory_entry(tenant_id, worker_id, memory_scope, layer)`, and #1013 added
`session.worker_id`. This PR switches system-prompt hydration to prefer the
session's worker namespace when present, while preserving shared-team fallback
for legacy or unbound sessions.

Do not invent `MemoryNs::worker(...)` or `MemoryScope::Personal`; those were
stale names. Use `MemoryNs::new(tenant_id, worker_id, MemoryScope::WorkerSelf)`.
The shared-team path remains supported through the existing
`compose_shared_team_memory` helper.

## Files you may touch

- `crates/argo-agent-runner/src/turn.rs` — narrowly: fetch/pass `session.worker_id` into memory prompt composition.
- `crates/argo-agent-runner/src/turn_memory.rs` — add worker/shared/both composition helper around existing shared-team helper.
- `crates/argo-agent-runner/src/turn_tests.rs` — extend memory prompt tests for worker/default/fallback behavior.
- `crates/argo-memory/src/layers.rs` — narrowly: expose or reuse existing composition output only if a small helper is needed; no schema changes.
- `crates/argo-memory/src/layer_tests.rs` — narrowly: tests for any new helper in `layers.rs`.

## Files you must NOT touch

- `docs/skycot/RUNBOOK.md` — already over the 500 LOC ceiling; do not append.
- `crates/argo-store/migrations/**` — #1013 already added `session.worker_id`; no migration in this brief.
- `crates/argo-identity/src/types.rs` — use existing `MemoryNs::new` and `MemoryScope::WorkerSelf`.
- Anything not on the may-touch list above.

## Acceptance criteria

- [ ] A session with `worker_id = Some(worker)` renders worker-scoped memory rows from `memory_entry`.
- [ ] A session with no worker id keeps the existing shared-team memory behavior.
- [ ] `ARGO_MEMORY_LAYERS=shared` forces shared-team memory even when a worker id is present.
- [ ] `ARGO_MEMORY_LAYERS=worker` with no worker id returns no memory block instead of falling back.
- [ ] `ARGO_MEMORY_LAYERS=both` renders worker memory first and shared-team memory second.
- [ ] Memory read errors still soft-fail with `tracing::warn!` and do not block prompt construction.
- [ ] `cargo clippy -p argo-agent-runner -p argo-memory --all-targets -- -D warnings` passes.
- [ ] `cargo test -p argo-agent-runner -p argo-memory` passes.
- [ ] Every modified file stays under 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `codex-mission-memory-per-worker-scope`
- PR title: `runtime: hydrate worker-scoped memory in system prompts (mission-memory-per-worker-scope)`

## Notes for the implementer

- Pattern-match current `turn_memory::compose_shared_team_memory`; keep the new helper in that sibling file so `turn.rs` stays small.
- `MemoryStore::compose_context(&ns, action_id)` returns a `ContextPack`, not markdown. Render only the fields already intended for prompt context; do not dump raw debug output.
- Use a deterministic action id such as `"system_prompt"` when composing memory for the prompt.
