# codex-memory-tool-namespace-wiring — Thread tenant/worker/MemoryNs through agent dispatch (F-03 P1)

**Owner**: codex. **Depends on**: nothing (`codex-toolcontext-tenant` already MERGED #1727 supplies tenant in ToolContext).
**Authored against**: origin/main `58e853b6` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-memory-tool-namespace-wiring`.
**Source**: F-03 P1 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Mission alignment

- **Claim 1 (teammate, not tool):** Workers MUST have a persistent `memory_namespace` (`docs/skycot/MISSION.md:56`). Without this fix, the model sees a tool that claims private/team scoping but actually writes to workspace-global memory.
- **Claim 3 (multi-worker handoff):** Worker B continues from shared memory without re-asking (`docs/skycot/MISSION.md:219`). Requires team namespace addressable from the runtime, not just in prompt hydration.

## Scope

P1 correctness. The prompt hydration path uses `MemoryNs` correctly (`crates/argo-agent-runner/src/turn_memory.rs:84`), but the TOOL CALL path doesn't:

1. `Memory::new` (used by runtime) leaves `namespace = None` (`crates/argo-tool/src/tools/memory.rs:39`). `Memory::with_namespace` exists (`memory.rs:84` doc) but is not called from agent runner.
2. `query_scored` errors unless tool is bound to a worker namespace (`crates/argo-tool/src/tools/memory_query.rs:79`).
3. `build_dispatch_ctx` signature accepts only `Option<(MemoryStore, WorkspaceId, Option<String>)>` (`crates/argo-agent-runner/src/dispatch.rs:69`) — no `tenant_id`, no `worker_id`, no `MemoryNs`.

The tool's schema lies to the model: it claims private = worker namespace, team = tenant-shared, but legacy `list/read/write` are workspace/vertical-global only.

Scope:
- Extend `build_dispatch_ctx` to accept `MemoryNs { tenant_id, worker_id }` alongside the existing memory tuple.
- Update `turn_helpers.rs` to derive `MemoryNs` from `session.tenant_id` (now in scope post-F-004) + worker context.
- Update `builtin_tools_with_runtime` in `crates/argo-tool/src/tools/mod.rs` to construct `Memory::with_namespace(store, ws, vertical, ns)` for the agent-runner code path. Bare `Memory::new` stays for schema-only/test callers.
- Add integration test: a model/tool call executes `query_scored` successfully AND a second worker cannot read the first worker's private memory.

LOC: ~280.

## Files you may touch

- `crates/argo-agent-runner/src/dispatch.rs` — extend `build_dispatch_ctx` signature with `Option<MemoryNs>`; update internal wiring. ≤80 modified lines.
- `crates/argo-agent-runner/src/turn_helpers.rs` — derive `MemoryNs { tenant: session.tenant_id, worker: session.worker_id }` and pass to dispatch ctx (`turn_helpers.rs:154`, `:159` are current sites). ≤50 modified lines.
- `crates/argo-tool/src/tools/mod.rs` — `builtin_tools_with_runtime` constructs `Memory::with_namespace` instead of `Memory::new` for production path (`mod.rs:130`, `:136`, `:138`). ≤40 modified lines.
- `crates/argo-tool/src/tools/memory.rs` — narrowly: ensure `Memory::with_namespace` is `pub` if it isn't already; doc update if signature shifts. ≤25 modified lines.
- `crates/argo-tool/src/tools/memory_query.rs` — narrowly: confirm `query_scored` succeeds when namespace bound (`memory_query.rs:79`). ≤15 modified lines.
- `crates/argo-agent-runner/tests/memory_namespace_runtime_tests.rs` (NEW) — integration test: worker A writes private, worker B can't read it; both can read team. ≤120 LOC.

## Files you must NOT touch

- `memory_tests.rs` (585 LOC, at-risk per F-09) — out of may-touch. If a test fixture needs updating, escalate.
- `crates/argo-memory/**` — that crate is correct; this brief is the WIRE through agent dispatch.
- `crates/argo-agent-runner/src/turn_memory.rs` — prompt hydration is correct; don't touch.

## Acceptance criteria

- [ ] `build_dispatch_ctx` signature accepts `Option<MemoryNs>`.
- [ ] `builtin_tools_with_runtime` constructs `Memory::with_namespace` for the agent-runner path (verify with `rg 'Memory::new\(' crates/argo-tool crates/argo-agent-runner --type rust | grep -v '_tests.rs\|/tests/' ` showing zero production hits beyond schema-only callers).
- [ ] New integration test proves cross-worker namespace isolation.
- [ ] `query_scored` succeeds from a model/tool call via the agent runner (existing test that errored on "tool bound to a worker namespace" now passes).
- [ ] `cargo test -p argo-tool -p argo-agent-runner` passes.
- [ ] `cargo clippy -p argo-tool -p argo-agent-runner --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `codex-memory-tool-namespace-wiring`
- PR title: `feat(agent-runner): bind memory tools to tenant/worker/MemoryNs (codex-memory-tool-namespace-wiring) — F-03 P1`

## Notes for the implementer

- The audit's F-03 specifically calls out `query_scored`, `list_team`, `read_team`, `write_team` as documented-but-unreachable. After this brief, all 4 should be exercisable through the agent runner.
- The post-F-004 `ToolContext.tenant_id` is the right source for the tenant; `worker_id` likely lives in session or vertical context (check `turn.rs` for the canonical source).
- Don't try to fix `memory_tests.rs` LOC issue in this brief — that's F-09's scope.
