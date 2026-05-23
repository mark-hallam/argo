# `argo-tool` substrate audit — registration, schema, MCP bridge, sandboxing

**Authored**: 2026-05-23 (evaluator follow-on to gap manifest `argo-tool` row, **beyond** the 15-tool inventory closed by `eval-in-core-tool-inventory-vs-mission`).
**Brief**: `docs/build-plan/streams/eval-argo-tool-surface-beyond-15-in-core.md`.
**Verdict (short form)**: 🟡 substrate is mostly clean — the registry's
capability gate, schema export, lazy-MCP discovery, and the
`path_guard` lexical fence all work as documented. **Five non-trivial
findings** all flow from the same architectural posture: `argo-tool` is
deliberately lean (it does not depend on `argo-mcp`, does not own the
MCP pool, does not own background-process scoping), so several
load-bearing invariants live one crate up in `argo-agent-runner` —
where they are partially missing.

The findings, in priority order:

| ID | Severity | Finding | Where |
|---|---|---|---|
| F1 | 🟡 P1 broken-wire | `terminal { background: true }` + `process` tool **dead in production** — `ToolContext.background_processes` is never set by `argo-agent-runner::build_dispatch_ctx`. Every dispatch returns "no `ProcessRegistry` attached". | `crates/argo-tool/src/tool.rs:131`; `crates/argo-agent-runner/src/dispatch.rs:171-173` |
| F2 | 🟡 P2 sharp-edge | `ProcessRegistry` has **no tenant scope** at the data model. `spawn`/`list`/`status`/`tail`/`kill` operate on raw `ProcessId`. Today the registry is per-context so cross-tenant kill is structurally impossible; if F1 is fixed by sharing one registry per gateway boot, F2 becomes an instantly-active cross-tenant leak. | `crates/argo-tool/src/process_registry.rs:80-289` |
| F3 | 🟡 P2 known gap | `path_guard::confine_path` is **lexical-only** — a symlink inside the workspace pointing outside still escapes confinement. Documented in the file header (#677 deferred symlink canonicalization). | `crates/argo-tool/src/path_guard.rs:30-32` |
| F4 | 🟡 P2 structural | MCP-discovered tools have a **weaker per-call cap fence** than in-core tools. `ToolRegistry::dispatch` enforces both `Capability::ToolDispatch` + per-arg `Tool::required_capabilities`; the MCP fast-path in `argo-agent-runner::dispatch_mcp` only enforces `ToolDispatch`. | `crates/argo-tool/src/registry.rs:188-212`; `crates/argo-agent-runner/src/dispatch.rs:198-217` |
| F5 | 🟢 P3 doc-drift | `builtin_tools(workspace_root: PathBuf)` arg is **ignored** (commented as "reserved for future per-workspace defaults"). Workspace resolves at dispatch time from `ToolContext.workspace_root`. The arg is dead weight + a footgun (callers think they're scoping the tools). | `crates/argo-tool/src/tools/mod.rs:66-84` |

Pin tests at `crates/argo-tool/tests/argo_tool_substrate_pin_test.rs`
(this round) capture the substrate invariants that are **today
correct** (registry capability gate, lazy MCP discovery, path-guard
escape rejection, ProcessRegistry tenant-blind-by-design) so future
refactors cannot silently regress them. F1 + F2 + F3 are documented
through `#[ignore]`'d FINDINGS-ONLY tests that demonstrate the gaps
and name the fix-briefs.

---

## §1 — Substrate inventory

`crates/argo-tool/src/` is six files + one `tools/` directory:

| File | LOC | Role |
|---|---|---|
| `lib.rs` | 51 | Module wiring + re-exports |
| `tool.rs` | 262 | `Tool` trait + `ToolContext` + `ToolSchema` + helpers |
| `registry.rs` | 309 | `ToolRegistry` — name→`Tool`, MCP server/route table, `dispatch()` cap-gate |
| `path_guard.rs` | 120 | `confine_path()` — lexical workspace-escape rejection (#677) |
| `process_registry.rs` | 379 | Background `ProcessRegistry` — spawn/list/tail/kill |
| `process_registry_tail.rs` | 135 | Ring-buffered stdout/stderr pump + unix signal helper |
| `tools/*` | 6760 (mod 349 + 26 files) | 15 in-core tools + tests (audited separately by `eval-in-core-tool-inventory-vs-mission`) |

Strict deny lints in force per `lib.rs:30`:
`#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]`.

## §2 — Registry + dispatch gate

### 2.a Capability gate (CORRECT — pin test added)

`ToolRegistry::dispatch` at `registry.rs:188-212` is the **single seam**
for in-core tool invocation. The contract:

```text
1. Lookup name → Tool (error if missing).
2. engine.check(principal, Capability::ToolDispatch { tool_name_glob = name }).
3. For each cap in tool.required_capabilities(&args): engine.check(...).
4. tool.dispatch(args, ctx).
```

The test `dispatch_denies_when_principal_lacks_tool_dispatch`
(`registry.rs:275-299`) pins step 2. Step 3 is the per-call
fine-grained fence — e.g. `FileRead` returns
`Capability::FsRead { path_glob: args.path }` so a principal granted
`FsRead { "/workspace/*" }` cannot read `/etc/passwd` even when the
broader `ToolDispatch { "file_read" }` glob passes. **Pin test added
this round**: `registry_dispatch_enforces_per_arg_required_capabilities`
in the new `argo_tool_substrate_pin_test.rs`.

### 2.b MCP server registration + schema export

The registry holds an internal MCP table (`registry.rs:31-36`):

- `servers: BTreeMap<String, serde_json::Value>` — transports keyed by name.
- `discovered: BTreeSet<String>` — which servers' tools have been listed.
- `tools: BTreeMap<String, McpToolRegistration>` — discovered tool → server.
- `mcp_pool: Option<Arc<dyn Any + Send + Sync>>` — opaque pool injected by agent-runner.

**Why opaque `Arc<dyn Any>`?** `argo-tool` deliberately does **not**
depend on `argo-mcp` (lean-core invariant). The pool is downcast in
`argo-agent-runner::dispatch::mcp_pool` (`dispatch.rs:277-281`). This is
the right architectural call — but it means `argo-tool` cannot enforce
that the injected pool is actually tenant-keyed. The per-tenant key is
the agent-runner's responsibility (and is correctly applied at
`dispatch.rs:56-58`: `pool.acquire(tenant_id, name, ...)`).

`schemas()` (`registry.rs:170-181`) returns the union of in-core + MCP
schemas sorted by name. Builtin tools shadow MCP names: when MCP
discovery hits a name collision the MCP entry is dropped with a
`tracing::warn!` (`registry.rs:131-139`). **Pin test added this
round**: `registry_schemas_merge_in_core_with_mcp_sorted_by_name` +
`registry_record_mcp_tools_does_not_shadow_in_core`.

### 2.c MCP route lookup + lazy discovery

`mcp_route(tool_name)` (`registry.rs:150-160`) returns
`Some((server_name, transport))` for any discovered MCP tool, else
`None`. The agent-runner's `dispatch_mcp` at
`dispatch.rs:198-241` is the consumer: when the in-core lookup
misses, it triggers `discover_mcp_tools` (which `list_tools` against
every undiscovered server, populating the registry) then re-tries
`mcp_route`. The lazy discovery means a fresh registry pays the
discovery cost on the first MCP call per turn. **Pin test added this
round**: `registry_mcp_route_misses_until_record_mcp_tools_called`.

### 2.d F4 — MCP-dispatch cap-gate is weaker than in-core (structural)

`dispatch.rs:206-217` (the MCP fast-path in agent-runner) enforces
**only** `Capability::ToolDispatch { tool_name_glob }`. There is no
equivalent of `Tool::required_capabilities` because MCP-discovered
tools have no Rust impl. The in-core path enforces both (see §2.a).

**Why not a fix-brief**: there is no per-tool capability descriptor
convention in MCP (the protocol's tool schema is argument-shape only,
not security-shape). Adding per-call caps for MCP would require either
(a) a manifest-side declaration in `vertical.toml` that maps `<server,
tool>` → cap-set, or (b) wrapping every MCP tool in an in-core
`Tool` adapter that declares caps. Both are larger architectural
questions belonging to the MCP roadmap, not to a substrate audit.

**Pin captured**: the in-core gate is asserted by the new
`registry_dispatch_enforces_per_arg_required_capabilities` test. The
MCP gate asymmetry is documented here and called out in the report —
not asserted against by a test because the test would just pin a
known structural limit.

## §3 — Schema validation surface

The `Tool::parameters()` method returns a `serde_json::Value`
containing a JSON Schema. **There is no validation step in
`ToolRegistry::dispatch`** — args are passed straight through to
`tool.dispatch(args, ctx)`. Each tool re-parses its args via
`serde_json::from_value::<Args>(args)` and returns a typed
`ArgoError::Tool { msg: "invalid arguments: …" }` on parse failure
(see `file_read.rs:52-56`, `terminal.rs` `Args` parsing, etc.).

**Verdict**: this is **correct** — JSON Schema validation in the
registry would duplicate (and could diverge from) the strongly-typed
`#[derive(Deserialize)]` shapes. The schema in `parameters()` is for
the provider/model's tool-list (so the model knows the call shape);
the actual fence is the typed `Deserialize` in `dispatch()`. Two
sources of truth, one source of enforcement.

The same pattern holds for `required_capabilities(args)` — each tool
parses args, derives the per-call cap, returns it. If parsing fails
in `required_capabilities`, the cap-set check at
`registry.rs:207-210` propagates the parse error before `dispatch`
gets called. **Pin test added this round**:
`registry_dispatch_propagates_required_capabilities_parse_error`.

## §4 — MCP bridge — argo-tool ↔ argo-mcp

### 4.a Bridge architecture (CORRECT)

`argo-tool` ships **no MCP-protocol code**. The bridge is entirely in
`argo-agent-runner::dispatch.rs`:

```text
argo-tool::ToolRegistry              argo-agent-runner::dispatch
  ┌──────────────────────────┐         ┌─────────────────────────┐
  │ servers / discovered /   │ <─wires─│ register_manifest_mcp_  │
  │ tools tables             │         │   servers (per-turn,    │
  │                          │         │   from argo-vertical)   │
  │ mcp_pool: Arc<dyn Any>   │ <─wires─│ McpConnectionPool wraps │
  │                          │         │   argo_mcp::McpClientPool│
  │                          │         │   keyed by (tenant_id,  │
  │                          │         │   server_name)          │
  │ mcp_route(name)          │ <─reads─│ dispatch_mcp(...) calls │
  │                          │         │   pool.acquire +        │
  │                          │         │   client.call_tool      │
  └──────────────────────────┘         └─────────────────────────┘
```

Tenant scoping happens at `dispatch.rs:56-58`:
`pool.acquire(tenant_id, name, ...)`. The argo-tool layer is
tenant-blind by design — the pool key carries the tenant.

### 4.b No parallel MCP surface inside argo-tool

`rg "McpConnectionPool|McpClientPool"` across `crates/argo-tool/` returns
zero hits (the type lives in `argo-agent-runner` and `argo-mcp`
respectively). The round-4 `McpConnectionPool` cross-tenant
credential-leak finding is therefore contained — `argo-tool` cannot
re-introduce it because it doesn't own the type.

### 4.c Per-turn pool churn (caller-side observation, NOT an argo-tool bug)

The bridge docstring at `dispatch.rs:20-26` claims "Subprocesses stay
live across turns; LRU + idle-TTL eviction reaps stale entries." But
`build_dispatch_ctx` at `dispatch.rs:92` creates a fresh
`McpConnectionPool::default()` per turn (which in turn calls
`McpClientPool::new(PoolConfig::default())` — a fresh pool). The
"warm across turns" claim is therefore **not true today**.

**Out of scope for this brief** (the fix lives in `argo-agent-runner`,
not `argo-tool`). Flagged here for the conductor: a P2 fix-brief
`mission-mcp-connection-pool-shared-across-turns` would lift the pool
to gateway boot and store an `Arc<McpConnectionPool>` on the agent
runner's shared state. Tag with `Authored against` once a fresh sweep
confirms the gap.

## §5 — Sandboxing

### 5.a Path confinement (`path_guard`)

`confine_path(workspace_root, requested)` (`path_guard.rs:38-56`)
performs two checks:

1. After joining `workspace_root.join(requested)`, **no `..` component
   may remain in the resolved path** — catches `../escape.txt` and
   `a/../../escape.txt` (both rejected by `nested_parent_traversal_rejected`).
2. If `requested.is_absolute()`, the joined path must lexically
   `starts_with(workspace_root)` — rejects `/etc/passwd`.

8 unit tests at `path_guard.rs:58-119` pin these rules. **Pin test
added this round**: `path_guard_rejects_symlink_inside_workspace_today`
(F3 — `#[ignore]`'d FINDINGS-ONLY) demonstrates the symlink-escape
that is not caught.

### 5.b Process registry (`process_registry`)

`ProcessRegistry` is a `HashMap<ProcessId, Arc<ProcessHandle>>` behind
an `RwLock` (`process_registry.rs:114-115`). API:

- `spawn(SpawnRequest { command, working_dir })` → `ProcessId`.
- `list()` → `Vec<ProcessInfo>` (reaps as it walks).
- `status(&ProcessId)` → `Option<ProcessInfo>` (reaps that one).
- `tail(&ProcessId)` → `Option<TailOutput>` (≤32 KB stdout + stderr).
- `kill(&ProcessId, grace: Duration)` → SIGTERM → wait `grace` → SIGKILL.
- `reap_finished()` → drops any process whose `try_wait` shows exited.

The child is spawned via `bash -c $command` at `process_registry.rs:137-147`
— same shell-eval pattern as `Terminal`, scoped by the caller-supplied
`working_dir`. There is **no path-guard on `working_dir`** here
(`Terminal::dispatch` passes `ctx.workspace_root` straight through);
the path-guard fences the `path` argument to `file_*` tools, not the
cwd argument to `terminal`. This is the right split — the workspace
root is the cwd, so confining cwd against itself is a tautology — but
it means a tool that synthesizes a cwd from args (e.g. a future
`spawn_in_subdir`) would need to confine the subdir explicitly.

### 5.c F1 (broken-wire) + F2 (tenant-blind) detail

Two findings on the process-registry sandboxing surface:

**F1 — broken wire.** `dispatch_one_with_tenant` at
`dispatch.rs:171-173` constructs `ToolContext` via
`ToolContext::new_with_tenant(...)` and immediately sets `call_id` +
`session_id`. It never calls `with_background_processes(...)`. Grep
across `crates/` confirms `with_background_processes` is only invoked
in test files (`tests/process_registry_integration.rs:20`,
`src/tools/process.rs:238`, `src/tools/terminal_tests.rs:357`). In
production, every `terminal { background: true }` dispatch returns:

> "background unsupported in this context: no ProcessRegistry attached"
>   (`terminal.rs:317`)

And every `process { action: ... }` call returns the same. The
docstring at `process_registry.rs:1-12` describes a feature that is
only reachable from in-crate tests.

Fix-brief shape: `mission-agent-runner-wire-process-registry-per-session`
— construct one `Arc<ProcessRegistry>` per worker session (so killing
the session reaps the background processes), thread it through
`build_dispatch_ctx`, attach via `ctx.with_background_processes(...)`.
Pin test (added this round, `#[ignore]`'d):
`production_tool_context_lacks_process_registry_today`.

**F2 — tenant-blind data model.** `ProcessRegistry` has zero
references to `TenantId`. `ProcessHandle` carries no tenant. The
`tail`/`kill`/`status` methods take a `ProcessId` and look it up in
the map — anyone with the `ProcessId` can target the process. Today
this is contained because:

1. The registry is per-context (F1: not wired in production), so the
   blast radius is the test that constructed it.
2. The registry has only ever been instantiated locally in test code.

But if F1 is fixed by sharing **one** registry across all sessions on
a gateway boot (the natural fix), F2 becomes an instant cross-tenant
leak: tenant B's worker, given a `process_id` from tenant A's audit
log, can kill A's background process. The right shape is to:

(a) Thread `tenant_id` through `SpawnRequest` (or store on `ProcessHandle`).
(b) Add a `tenant_id: TenantId` arg to `list`/`status`/`tail`/`kill`.
(c) Filter the `HashMap` lookups by tenant.

Fix-brief shape: `mission-process-registry-tenant-keyed`. Should land
**before** the F1 fix-brief, or be combined with it.

Pin test (added this round, `#[ignore]`'d):
`process_registry_kill_is_tenant_blind_today` — demonstrates that a
caller with knowledge of a `ProcessId` can kill it regardless of
tenant context (because there's no tenant context to consult).

### 5.d F3 — Path-guard symlink gap (known, deferred)

The file header at `path_guard.rs:30-32` documents:

> "Lexical-only — does not follow symlinks. A symlink inside the
>  workspace pointing outside still escapes. Follow-up PR (#678+) can
>  add canonicalization for the symlink case; the lexical check
>  catches the obvious / accidental traversal attempts that are the
>  most common mistake."

Pin test added this round (`#[ignore]`'d FINDINGS-ONLY):
`path_guard_rejects_symlink_inside_workspace_today` — creates a
symlink inside a temp workspace pointing to `/etc/hosts`, calls
`confine_path(workspace, symlink_name)`, asserts the call succeeds
(demonstrating the gap), and renders the resolved path. The test is
ignored so it does not gate CI; un-ignore it once the
`mission-path-guard-canonicalize-symlinks` brief lands.

## §6 — Tool inventory (cross-link to round-9)

The 15 in-core tools are inventoried in
`eval-in-core-tool-inventory-vs-mission.md` (same date). This audit
does not re-derive the inventory. Cross-cutting observations only:

- All 4 `builtin_tools_*` constructors (`mod.rs:66`, `:93`, `:145`,
  `:173`) return the same 15 tools in the same order; only the wiring
  differs (memory store, namespace, message dispatcher).
- `Memory` has 4 layered constructor paths
  (`default()` / `new(...)` / `with_namespace(...)` / wrapping the
  last two from runtime helpers). Tested separately.
- F5 — the `_workspace_root: PathBuf` arg on `builtin_tools` is unused
  in **every** constructor (prefixed `_` in 3 of 4 signatures,
  unprefixed but unread in the 4th). Fix-brief shape:
  `mission-builtin-tools-drop-unused-workspace-root-arg`.

## §7 — Verdict + follow-ups

**🟡 substrate audit verdict**: the registry, schema export, and MCP
bridge surface are clean. The sandbox surface has **one P1 broken
wire** (F1) and **one P2 latent cross-tenant leak** (F2) that will
co-fire if F1 is fixed naively. Plus one known P2 path-guard gap
(F3) and one structural cap-fence asymmetry (F4) that lives at the
MCP-roadmap level. F5 is a cosmetic API cleanup.

### Follow-up briefs

| Brief | Shape | Severity | Why |
|---|---|---|---|
| `mission-process-registry-tenant-keyed` | Add `TenantId` to `ProcessHandle`; tenant-filter `list`/`status`/`tail`/`kill`. ~100 LOC + cross-tenant negative test. | 🟡 P2 → escalates to **P0** if `mission-agent-runner-wire-process-registry-per-session` lands first. | F2. Order with F1 fix-brief. |
| `mission-agent-runner-wire-process-registry-per-session` | Add `Arc<ProcessRegistry>` to dispatch ctx (per session); attach via `ctx.with_background_processes`. ~80 LOC + integration test that drives `terminal { background: true }` → `process { action: "list" }` end-to-end through `dispatch_one_with_tenant`. | 🟡 P1 | F1. Land **after** the tenant-key brief above, or combine. |
| `mission-path-guard-canonicalize-symlinks` | Optional `std::fs::canonicalize` pass after lexical check, with feature flag to keep the lexical-only path for tests that pre-create paths. ~50 LOC + symlink-escape test. Un-ignore the pin test added this round. | 🟡 P2 | F3. Known deferred since #677. |
| `mission-builtin-tools-drop-unused-workspace-root-arg` | Remove the unused `_workspace_root: PathBuf` arg from all 4 `builtin_tools_*` signatures; update ~3 callers. Mechanical. | 🟢 P3 | F5. Cosmetic; do as ledger-replenishment filler. |
| `mission-mcp-connection-pool-shared-across-turns` | Lift `McpConnectionPool::default()` from per-turn (`dispatch.rs:92`) to gateway boot; pass `Arc<McpConnectionPool>` through `build_dispatch_ctx`. Honors the existing docstring claim. | 🟢 P3 | §4.c. Lives in `argo-agent-runner`, not `argo-tool`. |

F4 (MCP cap-fence asymmetry) is **not** a fix-brief — it's a
structural artifact of MCP's argument-shape-only tool schema. Future
manifest-level cap declarations (vertical.toml `mcp_servers[*].tools[*].caps`)
are the conventional fix; that belongs to the MCP roadmap, not a
single bug-fix brief.

### Pin tests added this round

`crates/argo-tool/tests/argo_tool_substrate_pin_test.rs`:

1. `registry_dispatch_enforces_per_arg_required_capabilities` (live).
2. `registry_dispatch_propagates_required_capabilities_parse_error` (live).
3. `registry_schemas_merge_in_core_with_mcp_sorted_by_name` (live).
4. `registry_record_mcp_tools_does_not_shadow_in_core` (live).
5. `registry_mcp_route_misses_until_record_mcp_tools_called` (live).
6. `path_guard_rejects_symlink_inside_workspace_today` (**`#[ignore]`'d** — F3 FINDINGS-ONLY).
7. `production_tool_context_lacks_process_registry_today` (**`#[ignore]`'d** — F1 FINDINGS-ONLY).
8. `process_registry_kill_is_tenant_blind_today` (**`#[ignore]`'d** — F2 FINDINGS-ONLY).

The 5 live tests pin substrate invariants that are currently correct;
they fail loudly if a future refactor regresses the registry gate,
schema merge, or MCP-discovery contract. The 3 ignored tests document
each finding's gap so un-ignoring is the signal that a fix-brief has
landed.
