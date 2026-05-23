# `argo-tool` in-core inventory vs MISSION / CLAUDE.md claim

**Authored**: 2026-05-23 (evaluator follow-on to gap manifest `argo-tool` row).
**Brief**: `docs/build-plan/streams/eval-in-core-tool-inventory-vs-mission.md`.
**Verdict (short form)**: 🟢 the 15-tool count + name list match the regression
test pin. **Every tool declares a capability** (or, in the case of
`session_search`, explicitly returns `CapabilitySet::empty()` and documents
that the registry's `Capability::ToolDispatch` glob is the gate). **Every
tool has at least one test.** **Every tool's in-core residence is
justified** by direct AgentDeps or sandbox/runtime primitives that an
out-of-process MCP server could not supply without re-introducing the same
plumbing.

No SHOULD-MIGRATE-TO-MCP verdicts. No P0 mission-drift findings: the
"15 in-core tools" line is in `CLAUDE.md` invariant #5, not in
`docs/skycot/MISSION.md`; both are operator-load-bearing and consistent.

## §1 — Source-of-truth cross-check

### 1.a `CLAUDE.md` line that asserts "15 in-core tools"

> `CLAUDE.md:50-52`:
> > "5. MCP-first integration (lean core, currently 15 in-core tools;
> >    new tools default to MCP unless they require direct AgentDeps
> >    or runtime access)"

This is a core-platform invariant — non-negotiable per CLAUDE.md `## Core-
platform invariants` (line 41).

### 1.b `docs/skycot/MISSION.md` cross-check

MISSION.md does **not** contain a literal "15 in-core tools" line. A grep
for `in-core`, `15`, `tools default to MCP` returns no matches in
`docs/skycot/MISSION.md`. The closest content is §"What 'any task a human
can do on a PC/Mac' means" (line 140-155), which describes the two
execution backends (MCP + computer-use) but does not enumerate in-core
tools.

**Drift verdict**: not a drift. CLAUDE.md is the canonical home for
core-platform invariants per its own §"Core-platform invariants" header.
The gap manifest cite "MISSION.md core-platform invariant #5" is a
shorthand — the actual invariant lives in CLAUDE.md. No
`mission-mission-md-tool-list-correction` brief is needed.

### 1.c Regression-test pin

The regression test `tests::builtin_tools_returns_unique_names` at
`crates/argo-tool/src/tools/mod.rs:222-251` asserts:

- `tools.len() == 15`.
- The sorted name list is exactly:
  `clarify, code_execute, delegate_task, file_patch, file_read,
   file_write, memory, process, send_message, session_search, terminal,
   todo, vision_analyze, web_extract, web_search`.

Confirmed against the current `builtin_tools(...)` constructor at
`mod.rs:66-84`. The 3 alternative constructors (`builtin_tools_with_memory`,
`builtin_tools_with_runtime`, `builtin_tools_with_runtime_and_message_dispatcher`)
all return the same 15 tools in the same order — only the wiring differs.

## §2 — Per-tool inventory matrix

Sources:

- `(a) capability declared` — `Tool::required_capabilities()` body in each
  tool's source file. The registry-level `Capability::ToolDispatch
  { tool_name_glob }` gate runs **before** this and is universal; this
  column lists the **additional** per-call capabilities declared by the
  tool itself.
- `(b) tenant-scoped` — traced through `dispatch(...)` to the first
  persistence or external touch. Tenant binding is read off
  `ToolContext::principal` or the wired store handle.
- `(c) test exists` — counted as either an inline `#[cfg(test)]` in the
  tool's `.rs` file or a `#[path = "..._tests.rs"] mod` companion plus
  any integration test in `crates/argo-tool/tests/`.
- `(d) MCP equivalent` — `rg crates/argo-mcp-connector-*` for the tool's
  primary verb. "—" means no MCP connector in this workspace performs the
  same job.
- `(e) justification` — pulled from the file's module docstring; "AgentDeps"
  means the tool requires a runtime-owned handle (MemoryStore, SessionStore,
  OrchestratorStore, MessageDispatcher, ProcessRegistry, sandbox backend,
  ClarifyDispatcher, VisionDispatcher).

| # | Tool | (a) capability | (b) tenant-scoped | (c) test | (d) MCP equivalent | (e) justification | verdict |
|---|---|---|---|---|---|---|---|
| 1 | `clarify` | `ToolDispatch{glob="clarify"}` (`clarify.rs:143`) | yes — `ClarifyDispatcher` is wired per-turn by the agent runner with the session's `(tenant_id, worker_id)` already bound | `clarify_tests.rs` + `tests/todo_clarify_integration.rs` | — | Direct AgentDeps: needs the per-session operator-Ask surface (TUI/gateway/CLI). An MCP server cannot synchronously surface a question to the bound operator surface. | JUSTIFIED-IN-CORE |
| 2 | `code_execute` | `Spawn{Container}` (`code_execute.rs:162`) | yes — sandbox is staged in a temp-dir per call; no cross-tenant filesystem reachability | inline `#[cfg(test)]` at `code_execute.rs:351+` + `tests/code_execute_integration.rs` | — | Sandbox primitive: requires `argo_sandbox::Sandbox` handle. Container/microVM lifecycle is runtime-owned. | JUSTIFIED-IN-CORE |
| 3 | `delegate_task` | `Spawn{Subprocess}` (`delegate_task.rs:117`) | yes — wired form takes `Arc<OrchestratorStore>`; handoff insert binds caller `(tenant_id, session_id)` from `ToolContext` (delegate_task.rs:1-12) | `delegate_task_tests.rs` + `tests/delegate_task_real_smoke.rs` | — | Direct AgentDeps: `OrchestratorStore` handoff queue is runtime-owned; subagent spawn requires AgentRunner internals. | JUSTIFIED-IN-CORE |
| 4 | `file_patch` | `FsRead{path}` + `FsWrite{path}` (`file_patch.rs:60-65`) | yes — path is confined to workspace via `confine_path` per the tool-layer convention; workspace root is per-tenant | inline `#[cfg(test)]` at `file_patch.rs:136+` | `argo-executor-filesystem` (filesystem executor scopes file access per `worker_fs_root`) | Filesystem primitive: must operate on the worker's bound `workspace_root`. The executor wrapper exists but the in-core tool is the dispatch leaf the agent loop calls. | JUSTIFIED-IN-CORE |
| 5 | `file_read` | `FsRead{path}` (`file_read.rs:57`) | yes — same workspace-root confinement as `file_patch` | inline `#[cfg(test)]` at `file_read.rs:117+` (3 tests) | `argo-executor-filesystem` | Filesystem primitive (same rationale as `file_patch`). | JUSTIFIED-IN-CORE |
| 6 | `file_write` | `FsWrite{path}` (`file_write.rs:54`) | yes — same workspace-root confinement | inline `#[cfg(test)]` at `file_write.rs:114+` | `argo-executor-filesystem` | Filesystem primitive (same rationale). | JUSTIFIED-IN-CORE |
| 7 | `memory` | `MemoryWrite` for `write`; `MemoryWriteTeam` for `write_team`; empty otherwise (`memory.rs:188-197`) | yes — wired form binds `(tenant_id, worker_id, memory_scope)` via `Memory::with_namespace` per round-13 audit `eval-memory-per-worker-scope-honored` | `memory_legacy_tests.rs` + `memory_namespace_tests.rs` + `tests/memory_namespace_smoke.rs` + `tests/memory_query_scored_smoke.rs` | — | Direct AgentDeps: requires `Arc<MemoryStore>` + `WorkspaceId` + optional `MemoryNs`. Memory substrate is intentionally not exposed via MCP — would lose the per-worker scope guarantee. | JUSTIFIED-IN-CORE |
| 8 | `process` | `Spawn{BackgroundProcess}` (`process.rs:87`) | yes — `ProcessRegistry` is per-session on `ToolContext`; cannot enumerate other sessions' processes | inline `#[cfg(test)]` at `process.rs:343+` + `tests/process_registry_integration.rs` | — | Direct AgentDeps: child-process lifecycle is owned by `ProcessRegistry` on `ToolContext`. Strictly stronger than `terminal`'s `Spawn{Subprocess}`. | JUSTIFIED-IN-CORE |
| 9 | `send_message` | `MessageSend{channel, risk_tier}` (`send_message.rs:210`) | yes — wired `MessageDispatcher` is constructed per-turn with the session's `(tenant_id, audit_sink, channel_registrations)` per round-9 work `mission-send-message-production-dispatcher` | `send_message_tests.rs` + `tests/send_message_integration.rs` | — (channels are gateway transports, not MCP connectors) | Direct AgentDeps: requires `MessageDispatcher` over the per-turn `SessionStore` + `AuditSink` + per-tenant outbound channel registrations. | JUSTIFIED-IN-CORE |
| 10 | `session_search` | `CapabilitySet::empty()` (`session_search.rs:119-121`) — documented: registry-level `ToolDispatch{glob="session_search"}` is the gate, "no further per-call capabilities are declared" (`session_search.rs:12-15`) | yes — wired form binds `Arc<SessionStore>` for the caller's tenant; FTS5 query is scoped to that store; "the FTS5 query never crosses tenant boundaries" (`session_search.rs:8-11`) | `session_search_tests.rs` + `tests/session_search_integration.rs` | — | Direct AgentDeps: requires `Arc<SessionStore>` bound to the caller's tenant. Read-only by construction. | JUSTIFIED-IN-CORE |
| 11 | `terminal` | `Spawn{Subprocess}` / `Spawn{Container}` / `Spawn{BackgroundProcess}` per backend + `background` flag (`terminal.rs:97-106, 227+`) | yes — `LocalSandbox` runs in the worker's `workspace_root`; sandbox/container backends inherit per-call confinement | `terminal_tests.rs` + `tests/terminal_integration.rs` + `tests/terminal_streaming_integration.rs` | — | Sandbox primitive: requires `argo_sandbox::Sandbox` + `ProcessRegistry` for background spawns. | JUSTIFIED-IN-CORE |
| 12 | `todo` | `ToolDispatch{glob="todo"}` + per-action `FsRead` (`list`) / `FsWrite` (`add`/`complete`/`remove`) (`todo.rs:103-110`) | yes — state file is `<workspace_root>/.argo/todo.json`; workspace root is per-tenant | `todo_tests.rs` + `tests/todo_clarify_integration.rs` | — | Filesystem primitive over the worker's workspace; not API-shaped (no remote backend). | JUSTIFIED-IN-CORE |
| 13 | `vision_analyze` | `ToolDispatch{glob="vision_analyze"}` + optional `FsRead{path}` when image is a local path (`vision.rs:171-180`) | yes — local image paths go through `confine_path` (`vision.rs:14-25` "an agent cannot ask vision_analyze to read /etc/shadow.png (#677)"); provider routing reuses the agent's currently-bound transport | `vision_tests.rs` + `tests/vision_dispatch.rs` | — | Direct AgentDeps: provider routing reuses the session's bound multimodal provider transport. An MCP server would have to re-route the image through a separate provider account, losing the per-session billing + audit binding. | JUSTIFIED-IN-CORE |
| 14 | `web_extract` | `NetEgress{host}` (`web/mod.rs:194`) | per-call, host-scoped via `NetEgress`; no per-tenant identity needed (read-only fetch) | inline `#[cfg(test)]` at `web/mod.rs:254+` (5 tests across `web::tests` covering both search + extract) | — (the noted "browser MCP server" #642 is not yet present as a workspace crate) | Lean fallback: real HTTP-backed implementation so worker pre-flight (#722) doesn't refuse autonomous research jobs when no MCP browser connector is configured. Could migrate when the browser MCP lands; see §3 finding for rationale. | JUSTIFIED-IN-CORE (today) |
| 15 | `web_search` | `NetEgress{host}` (`web/mod.rs:105`) | per-call host-scoped via `NetEgress`; backend selection reads env (`TAVILY_API_KEY` → Tavily; `BRAVE_API_KEY` → Brave; default DuckDuckGo HTML scrape) | inline `#[cfg(test)]` at `web/mod.rs:254+` | — | Lean fallback (same rationale as `web_extract`). | JUSTIFIED-IN-CORE (today) |

## §3 — Capability-coverage roll-up

| Bucket | Count | Tools |
|---|---|---|
| Declares a per-call capability beyond `ToolDispatch` glob | 14 | clarify, code_execute, delegate_task, file_patch, file_read, file_write, memory (when action ∈ {write, write_team}), process, send_message, terminal, todo, vision_analyze (when image is a local path), web_extract, web_search |
| Returns `CapabilitySet::empty()`; relies on registry glob only | 1 | session_search (documented in module docstring) |
| Declares >1 capability variant | 5 | file_patch (FsRead + FsWrite), memory (MemoryWrite / MemoryWriteTeam by action), terminal (Spawn{Subprocess|Container|BackgroundProcess} by backend + flag), todo (ToolDispatch + FsRead/FsWrite by action), vision_analyze (ToolDispatch + FsRead when local path) |
| Spawn-bearing | 4 | code_execute, delegate_task, process, terminal |
| FS-bearing | 5 | file_patch, file_read, file_write, todo (state file), vision_analyze (local image) |
| Net-bearing | 2 | web_search, web_extract |
| Memory-bearing | 1 | memory |
| Message-bearing | 1 | send_message |
| Operator-Ask-bearing | 1 | clarify |

## §4 — In-core vs MCP boundary check

The CLAUDE.md invariant tests every in-core tool against the rule
*"new tools default to MCP unless they require direct AgentDeps or
runtime access."*

| Reason it lives in core | Tools |
|---|---|
| **Direct AgentDeps** (runtime-owned handle: MemoryStore, SessionStore, OrchestratorStore, MessageDispatcher, ProcessRegistry, ClarifyDispatcher, VisionDispatcher) | clarify, delegate_task, memory, send_message, session_search, vision_analyze, process |
| **Sandbox / runtime primitive** (`argo_sandbox::Sandbox` + container lifecycle) | code_execute, terminal |
| **Filesystem primitive** (workspace-root confinement, `confine_path`, `worker_fs_root`) | file_patch, file_read, file_write, todo |
| **Lean fallback** (real HTTP-backed implementation so workers don't depend on operator-configuring an MCP connector) | web_search, web_extract |

Every tool's residence is justified. The only tools whose justification
isn't a hard "AgentDeps or sandbox primitive" requirement are `web_search`
+ `web_extract`. Per the file header (`web/mod.rs:1-10`) they were
promoted from stubs to real implementations precisely because the worker
v2 pre-flight gate (#722) was refusing autonomous research jobs while
the tools were schema-only. The noted browser MCP server (#642) is not
yet present in this workspace — `rg crates/argo-mcp-connector-` returns
no browser/web/search connector. **If/when** an MCP browser connector
ships and is operator-configurable by default, these two tools become
SHOULD-MIGRATE-TO-MCP candidates. Today there is no MCP parity for them,
so they stay in-core as the lean fallback.

## §5 — Findings escalation

Per the brief's escalation table:

- **Any tool with NO declared capability?** No. session_search documents
  `CapabilitySet::empty()` as a deliberate registry-glob-only gate; this
  is not the same as "no capability declared" — the registry's
  `Capability::ToolDispatch{glob="session_search"}` is the structural
  gate. **No `mission-tool-<name>-add-capability-declaration` brief
  needed.**
- **Any tool with NO test?** No. All 15 tools have at least one
  `#[test]` / `#[tokio::test]` reachable from `cargo test -p argo-tool`
  (either inline `#[cfg(test)]`, a `#[path = "..._tests.rs"] mod`
  companion, or a `tests/<tool>_integration.rs` integration test).
  **No `eval-tool-<name>-add-test` brief needed.**
- **Any SHOULD-MIGRATE-TO-MCP tool?** None today. **No
  `mission-tool-<name>-migrate-to-mcp` brief needed.** See §4 for the
  `web_search`/`web_extract` future-state note.
- **MISSION.md "15 tools" claim drift?** No. The claim lives in
  CLAUDE.md invariant #5, not in MISSION.md. Count + names match the
  pinned regression test. **No `mission-mission-md-tool-list-correction`
  brief needed.**

## §6 — Pin test added

`crates/argo-tool/tests/in_core_tool_inventory_pin_test.rs` adds a
coverage-pin alongside the existing name-list pin:

- **Per-tool capability pin** (`tool_declares_capability_or_documented_empty`):
  For each of the 15 tools, asserts `required_capabilities(<example
  args>)` returns either a non-empty `CapabilitySet` **or** matches the
  `session_search` documented-empty exemption. Future drift where a tool
  silently regresses to empty without the documentation seam fails the
  pin.
- **Per-tool test-coverage pin** (`tool_has_at_least_one_test_file`):
  For each tool, asserts at least one of these files exists in the
  workspace: `crates/argo-tool/src/tools/<name>.rs` containing a
  `#[cfg(test)]` block, a `crates/argo-tool/src/tools/<name>_tests.rs`
  companion, or a `crates/argo-tool/tests/<name>_*.rs` integration
  test. This is the *coverage* pin the brief asked for (the round-11
  test pins the names; this pins that someone is exercising them).

## §7 — Cross-references

- Round-11 regression test (name list + count): `crates/argo-tool/src/tools/mod.rs:222-251`.
- Gap-manifest row this audit closes: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` `argo-tool` row (line 83).
- Per-worker memory scope audit (round 13): `docs/skycot/evals/scheduled/2026-05-23/eval-memory-per-worker-scope-honored.md`.
- Cred-scoping audit (referenced by `vision_analyze`/`web_*` egress paths): `docs/skycot/evals/scheduled/2026-05-23/eval-credentials-cross-tenant-isolation.md`.

## §8 — Done-criteria roll-up

- [x] All 15 tools classified across all 6 cells.
- [x] CLAUDE.md citation included with verbatim line.
- [x] MISSION.md cross-check stated explicitly: claim lives in CLAUDE.md, not MISSION.md.
- [x] Pin test added at `crates/argo-tool/tests/in_core_tool_inventory_pin_test.rs`.
- [x] Gap-manifest `argo-tool` row flipped 🟢 (see PR diff).
- [x] At least one paragraph per SHOULD-MIGRATE-TO-MCP finding — none today; web_* note in §4.
