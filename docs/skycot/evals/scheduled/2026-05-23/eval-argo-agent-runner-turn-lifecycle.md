# eval — `argo-agent-runner` turn-lifecycle audit

**Brief**: `eval-argo-agent-runner-turn-lifecycle`
**Authored**: 2026-05-23 against `origin/main` `5a8726465`
**Scope**: full read-only walk of `crates/argo-agent-runner/src/`
(5,918 LOC across 33 files) plus the immediate downstream callers in
`argo-skill`, `argo-store`, `argo-memory`, `argo-audit`.
**Source upstream**: auditor T21-39 §4 P0 #2 + evaluator round-8
`mission-vs-code-gap-manifest.md:105`: *"`argo-agent-runner` — THE
runtime; largest unaudited mission-critical surface, every mission
claim routes through it."*

This is an **investigation deliverable**: findings + brief-shape
recommendations, no runner-src edits. Production fixes for each
finding ship in follow-up briefs (see §7).

---

## §1 — Surface inventory

`crates/argo-agent-runner/src/` exports **43 `pub` items** across
**33 files** (5,918 LOC, max file 419 LOC — under the 500-LOC HARD
RULE 6 ceiling). Categorized:

### Entry points (5)

- `run_turn` (`turn.rs:90`) — the canonical one-turn API. Used by
  the `argo chat` CLI and gateway `POST /sessions/<id>/runs`.
- `run_turn_with_events` (`turn.rs:141`) — streaming variant; the
  gateway `POST /sessions/<id>/runs/stream` endpoint passes a real
  `UnboundedSender<serde_json::Value>` so each `AssistantMessage` /
  `ToolCall` / `ToolResult` event reaches the WebSocket client in
  real time.
- `run_turn_with_planner_hook` (`turn.rs:111`) — wraps
  `run_turn_with_events` with a pre-turn
  `turn_planner::maybe_decompose` pass for the
  mission-planner-dispatch-loop-wire flow.
- `MAX_TOOL_ROUNDS` (`turn.rs:43`) — public default cap (8).
- `TurnResult` (`turn.rs:62`) — return type. Notable field:
  `stopped_on_tool_budget: bool` — set when `max_rounds` is hit so
  worker-v2 outer step loop can convert a research-heavy turn into
  a clean step boundary instead of a transport error (#724).

### Per-turn primitives (the load-bearing pipeline)

The orchestration body lives in
`turn_run_loop::run_turn_with_events_inner` (`turn_run_loop.rs:28`).
Per-turn it does, in order:

1. Load the session row (`store.get_session(session_id)`).
2. Record `tenant_id` and `worker_id` on the agent-turn tracing
   span. Update `prompt_cache_affinity` for `(tenant, worker)` pair
   (mission-prompt-cache-affinity-wire-mints).
3. **Pre-turn freeze gate** — `worker_freeze::is_frozen(pool,
   tenant_id, worker_id)`. Frozen → `ArgoError::Invariant` with
   `freeze_message`. No-op for sessions without a `worker_id`.
4. Append user message if `Some` (`store.append_message(User, …)`).
5. Load provider manifest (`load_manifest(cwd, session.provider)`).
6. Build `AuditSink::new_with_tenant(store, tenant_id)`.
7. **Tier-routing decision** — `turn_routing::maybe_route_turn(…)`.
   Sessions with `RoutingPolicy::Tiered` get model overridden to
   `RoutingDecision::chosen_provider_model.model`. No-op for the
   default (no policy) path.
8. Build transport from manifest.
9. Resolve vertical context (`turn_vertical::*`): system_prompt
   path, deny-set, approval-set, persistent + one-shot grants.
10. **Pre-turn budget gate** —
    `turn_budget::check_pre_turn_budget` finds the running subtask
    and aborts with `ArgoError::BudgetExhausted` if remaining ≤ 0.
11. Write `AuditKind::TurnStart` audit + emit event.
12. Build `SpendCtx`.
13. Mint `invocation_id` (`Uuid::now_v7()`) for skill A/B dedup.
14. Call `turn_helpers::run_turn_with_tools` (`turn_helpers.rs:163`)
    — the actual round loop:
    - `build_turn_dispatch_ctx`: registry + capability engine +
      principal + system prompt (built via
      `build_system_prompt_with_ab` →
      `turn_helpers::load_prompt_skills` →
      `turn_canary::apply_canary_routing` →
      `turn_skills_runtime::resolve_variants_for_turn` →
      `turn_skills_migration::emit_pending_migration_hints` →
      `turn_skills::compose_available_skills_block` →
      `turn_memory::compose_prompt_memory`).
    - Per round (up to `max_rounds`):
      - `build_round_request` → fetch messages → context-window
        truncation → `ProviderRequest`.
      - `run_one_round` → `transport.complete[_stream]` → persist
        assistant message → audit `AssistantMessage` → emit event →
        `record_turn_spend` + `record_subtask_turn_spend`.
      - If no tool calls: exit with `finalize_turn`.
      - Else for each call: pre-dispatch freeze check
        (`turn_helpers_freeze::pre_dispatch_or_block`) → audit
        `ToolCall` → resolve via deny → approval-gate → grant →
        `dispatch_one_with_tenant` (capability check + builtin or
        MCP dispatch) → audit `ToolResult` → append Tool message.
    - On `max_rounds` exhaustion: `emit_halt` + return
      `stopped_on_tool_budget = true`.
15. Record pending skill-A/B outcomes
    (`turn_skills_runtime::record_pending_outcomes`).
16. Write `AuditKind::TurnEnd` (success) or `AuditKind::Halt`
    (failure) — **best-effort, errors swallowed via `let _ =`**.

### Helper modules (all `pub(crate)` or `pub(super)`)

`turn_helpers`, `turn_helpers_context`, `turn_helpers_freeze`,
`turn_helpers_overlay`, `turn_helpers_run`, `turn_canary`,
`turn_memory`, `turn_skills`, `turn_skills_runtime`,
`turn_skills_migration`, `turn_context_policy`,
`turn_system_prompt`, `turn_run_loop`, `turn_vertical`,
`turn_vertical_grants`, `dispatch`, `transport`, `helpers`.

### Public utility surface (used by external drivers)

`helpers::{build_assistant_content, emit_event, lift_tool_blocks,
message_from_row, price_for, record_turn_spend, stream_one_round,
RoundResp, RunEventSink, SpendCtx}`, `transport::{build_transport,
load_manifest}`, `turn_routing::{build_classifier_transport,
maybe_route_turn, maybe_route_turn_with_transport}`,
`turn_budget::{check_pre_turn_budget, record_subtask_turn_spend,
compute_cents}`, `turn_planner::{maybe_decompose, DecomposeOutcome,
PlannerHook}`, `instrumentation::ir_trace::{TraceEvent,
TraceEventKind, set_trace_hook, dispatch_event, emit_node_*}`.

### Mission-claim mapping

- **Claim 1** (teammate-not-tool): worker identity is plumbed in
  via `session.worker_id` → `MemoryNs::WorkerSelf` (`turn_memory`)
  and the trust-gated skill block (`turn_skills`). ✓ load-bearing.
- **Claim 2** (delegation): planner hook + subtask budget gate
  (`turn_planner` + `turn_budget`). ✓ load-bearing.
- **Claim 3** (platform builds workers): vertical pack drives
  system prompt + deny/approval (`turn_vertical*`). ✓.
- **Claim 4** (skills): `LayeredLookup::with_library` in
  `turn_helpers.rs:65` is the SHIP-BLOCKER closure shipped in
  #1682. ✓ wired.
- **Claim 5** (any-PC execution): runner dispatches MCP + builtin
  tools. **Computer-use is NOT dispatched from this crate** — it
  runs from `argo-cli/src/handlers/worker_actions_executor_cu_*.rs`
  at the worker-action layer above the runner. See §6.
- **Claim 6** (multi-tenant): `tenant_id` plumbed into spend, audit
  sink, MCP pool key, memory namespace, prompt-cache affinity. See
  §2 for unbound query findings.

---

## §2 — Tenancy audit (SQL queries)

The runner's direct SQL surface is small — only two queries live
inside the crate; everything else routes through `argo-store` /
`argo-skill` / `argo-memory` helpers. Findings below cover both the
crate-local queries and the immediate-downstream helpers the runner
calls per turn.

### Finding 2.1 — `skill_ab_test_run` lookup has no tenant binding 🔴 P0

**File**: `crates/argo-agent-runner/src/turn_skills_runtime.rs:159-165`

```sql
SELECT id, variant_a, variant_b FROM skill_ab_test_run
WHERE skill_name = ? AND status = 'running'
ORDER BY started_at DESC LIMIT 1
```

`find_active_ab_run` binds only `skill_name`. If tenant A opens an
A/B test on a same-named skill (e.g. "summarize") and tenant B
**also** has a skill named "summarize" in the same workspace, the
runner of tenant B's worker will deterministically pick A or B
based on tenant A's run — and (worse) `record_outcome` will write
trial counts onto tenant A's `skill_ab_test_run` row from tenant
B's traffic, corrupting tenant A's experiment.

Severity: **P0** for any multi-tenant-per-workspace deployment.
Per memory `feedback_enterprise_grade_defaults.md` the target
architecture is strict-by-default; this query qualifies as an
unbound query that needs a tenant-aware fix.

**Brief**: `fix-skill-ab-test-run-tenant-bind` — add a `tenant_id`
column to `skill_ab_test_run` (additive migration, NULL backfill
acceptable for legacy rows) and bind `tenant_id` into both the
lookup query in `turn_skills_runtime.rs:159` and the dual
`record_outcome` upsert in `argo-skill/src/ab_test.rs`. Cross-tenant
negative test: two tenants, same skill name, one A/B run, verify
the second tenant's turn does **not** pick a variant from the
first's run.

### Finding 2.2 — `subtask_ledger` lookup has no tenant binding 🟡 P2

**File**: `crates/argo-agent-runner/src/turn_budget.rs:40-50`

```sql
SELECT id FROM subtask_ledger
WHERE parent_session_id = ? AND state = 'running'
ORDER BY started_at DESC LIMIT 1
```

`running_subtask_id` binds only `parent_session_id`. Session IDs
are UUIDv7, so collision across tenants is statistically negligible
— but per the enterprise-grade-defaults principle, every read on
tenant-owned data should re-assert the tenant. A future bug that
re-uses session_ids (e.g. test fixtures, replayed sessions) would
silently surface another tenant's running subtask.

Severity: **P2** — defense-in-depth, not an exploitable gap today.

**Brief**: `fix-subtask-ledger-tenant-bind` — extend
`SessionHeader` propagation into `check_pre_turn_budget` (already
has the session row via the caller in `turn_run_loop.rs:117`) and
add `AND tenant_id = ?` to the SELECT.

### Finding 2.3 — `skill` table has NO `tenant_id` column 🔴 P0 (architectural)

**Files**: `crates/argo-store/src/migrations/v1.rs:107-119`,
`crates/argo-skill/src/store_available.rs:55-70` (the SELECT the
runner uses every turn via
`turn_skills::compose_available_skills_block`).

The `skill` table schema:

```sql
CREATE TABLE skill (
    id BLOB PRIMARY KEY, name TEXT, version TEXT, body TEXT,
    frontmatter TEXT, source TEXT, hash BLOB,
    learned_in BLOB, superseded_by BLOB, created_at INTEGER,
    UNIQUE(name, version)
)
```

— no `tenant_id`. `crates/argo-store/src/skill_share.rs:22-26`
explicitly documents this gap: *"The `skill` table has no
`tenant_id` column today, so there is no notion of an 'owning
tenant' on a shared row."*

Downstream: `SkillStore::available_for_worker` filters by
`min_trust_grade` only (no tenant predicate). The trust grade is
fetched via `worker.trust_grade` keyed by `(tenant_id, worker_id)`
— so the lookup is tenant-aware in its READ of the worker, but the
**skill selection itself is workspace-global**.

In a single-tenant-per-workspace deployment this is a non-issue.
In any multi-tenant-per-workspace deployment (the v1 enterprise
gateway model carries this as a stated target), this is a P0
cross-tenant leak: tenant B's worker_b sees tenant A's skills, and
the trust-gate `min_trust_grade` is satisfied because trust_grade
is a numeric scalar shared across tenants.

This is the highest-severity finding in the audit because it
crosses three primitives: skills, skill A/B, and skill canary (the
canary lookup at `argo-skill/src/canary.rs:184-188` also keys by
`skill_id` only).

**Brief**: `fix-skill-table-tenant-column` — additive migration
adding `tenant_id BLOB NOT NULL DEFAULT <placeholder>` to `skill`,
backfilling legacy rows to the default tenant. Cascade into
`available_for_worker`, `skills_for_session`, `find_active_by_name`,
`list_active`, `pick_canary_for_skill`, `find_active_ab_run`,
`lookup_migration`. Brief is large (5-7 SELECTs across 3 crates) —
recommend `Manager-worker allowed: yes` with split children: (1)
migration + backfill, (2) `available_for_worker` + skill A/B
queries, (3) canary + migration lookup. Cross-tenant negative tests
per surface.

---

## §3 — Audit-chain integrity

### Finding 3.1 — Audit writes are async/batched, NOT transactional with mutations 🟡 P1

**Files**: `crates/argo-audit/src/sink.rs:174-190` (the `write`
method) + `crates/argo-audit/src/sink.rs:130-152` (the
`BatchedWriter` constructor) vs `mission-workflow-audit-write-
transactional` shape from #2078.

`AuditSink::write` enqueues into a `BatchedWriter` and returns. The
batch flushes on size or latency triggers in a background task —
there is **no `sqlx::Transaction` shared with the runtime mutation
it accompanies.**

The runner does:

```
store.append_message(User, …).await?   // direct INSERT, own tx
audit.write(TurnStart, …).await?       // batched, fires later
…
store.append_message(Assistant, …).await?     // direct INSERT
audit.write(AssistantMessage, …).await?       // batched
record_turn_spend(…).await?                   // direct INSERT
audit.write(ToolCall, …).await?               // batched
dispatch_one_with_tenant(…).await
audit.write(ToolResult, …).await?             // batched
store.append_message(Tool, …).await?          // direct INSERT
```

If the process crashes while a batch is in-flight, you can have:
persisted messages with no corresponding audit row, dispatched
tool calls with no `ToolResult` audit row, recorded spend with no
`AssistantMessage` row. The hash chain on disk stays
self-consistent but the chain is **incomplete** vs the live state.

Severity: **P1**. Not a security vulnerability — the chain itself
still verifies — but the chain ceases to be a complete record of
what the runner did. Compare to #2078 (workflow audit transactional
writes) which moved workflow_audit into the same `sqlx::Transaction`
as the workflow mutation specifically to fix this gap for workflow
admin actions.

**Brief**: `fix-runner-audit-transactional` — introduce a
`audit.write_in_tx(&mut tx, …)` companion (the trait already exists
at `crates/argo-audit/src/sink_tx.rs:85`) and wire the four
per-round mutation+audit pairs through one shared `tx` per round.
Scope: ~150-300 LOC in `turn_helpers_run.rs` +
`turn_run_loop.rs`. Risk: SessionStore's append_message uses its
own pool; need to route mutations through a `&mut Transaction`
overload. Recommend explicit FINDINGS-ONLY test FIRST
(crash-between-action-and-audit producing orphan rows) to pin the
invariant.

### Finding 3.2 — Pre-turn freeze gate produces no audit row 🟡 P1

**File**: `crates/argo-agent-runner/src/turn_run_loop.rs:57-71`

The pre-turn freeze check returns `Err(ArgoError::Invariant(…))`
**before** `audit = AuditSink::new_with_tenant(…)` is constructed
(audit-sink construction is at line 80, freeze check is at lines
57-71). A frozen-worker pre-turn rejection therefore produces no
`WorkerFreezeBlocked` audit row.

By contrast the per-dispatch freeze gate at
`turn_helpers_run.rs` does call
`turn_helpers_freeze::pre_dispatch_or_block` which writes
`WorkerFreezeBlocked` before returning the error.

Asymmetric: a frozen worker that has an in-flight session sees one
audit row per refused tool call but **zero** rows per refused turn
attempt. The forensic chain loses the "tried to start a turn while
frozen" signal.

Severity: **P1**. Operator-facing forensic gap.

**Brief**: `fix-pre-turn-freeze-audit` — hoist the audit-sink
construction before the freeze check, write `WorkerFreezeBlocked`
on the pre-turn rejection path with `phase: "pre_turn"`, keep the
existing per-dispatch row with `phase: "pre_dispatch"` for clarity.

### Finding 3.3 — TurnEnd/Halt audit-write failures silently swallowed 🟡 P2

**Files**: `turn_run_loop.rs:174-178` and
`turn_routing.rs:115-118`.

```rust
let _ = audit
    .write(Some(session_id), kind, serde_json::json!({}), None)
    .await;
```

`TurnEnd` and `Halt` audit failures are explicitly ignored via
`let _ = …`. So is `RoutingDecision` in
`maybe_route_turn_with_transport`. The first comment in
`turn_routing.rs:114` justifies this for routing decisions
(*"Never fail the turn on audit-write IO"*), but the trade-off is
that a swallowed audit-write loses the "why this model was
chosen" trail. Same with `TurnEnd` — a successful turn that fails
to write `TurnEnd` looks indistinguishable from a turn that was
killed mid-flight.

Severity: **P2**. Operationally observable today via lacking-rows
counts, but not surfaceable in any specific event.

**Brief**: `fix-runner-audit-emit-metrics` — wrap the swallowed
writes in a counter (`audit_emit_failure_total` keyed by
`AuditKind`) so Prometheus / OTLP rollups surface emit failures
without aborting turns. ~50 LOC.

---

## §4 — Skill resolution path

The end-to-end skill resolution traced from
`build_system_prompt_with_ab` (`turn_system_prompt.rs:51`) through
its dependent calls:

1. `turn_helpers::load_prompt_skills` (`turn_helpers.rs:36`) →
   `SkillStore::skills_for_session(session_id)` joined with
   `LayeredLookup::with_library` for the worker library +
   `append_worker_overlays_as_workspace` for per-worker overlays.
2. `turn_canary::apply_canary_routing` —
   `pick_canary_for_skill(name, …)`.
3. `turn_skills_runtime::resolve_variants_for_turn` —
   `find_active_ab_run(name)` + `select_variant(run_id, invocation_id)`.
4. `turn_skills_migration::emit_pending_migration_hints` —
   `lookup_migration(store, skill_id, version)`.
5. `turn_skills::compose_available_skills_block` —
   `SkillStore::available_for_worker(reader, tenant_id, worker_id)`.

### Finding 4.1 — `SkillStore::available_for_worker` lacks tenant predicate 🔴 P0

**File**: `crates/argo-skill/src/store_available.rs:55-70`. See
§2.3 — the SELECT joins only `skill_lock` and filters by
`superseded_by IS NULL AND (min_trust_grade IS NULL OR worker_grade
>= min_trust_grade)`. The worker's `trust_grade` is read via a
tenant-aware `(tenant_id, worker_id)` lookup, but the skill SELECT
itself returns workspace-global rows.

This is the same defect described in §2.3, surfaced from the skill
path. The recommended fix in §2.3 (`fix-skill-table-tenant-column`)
fixes both surfaces.

### Finding 4.2 — Skill propagation does fire every turn 🟢 (proof of correctness)

The mission-claim-4 SHIP-BLOCKER (#1682) closure at
`turn_helpers.rs:65` is reached unconditionally on every call into
`load_prompt_skills_with_library`, which is itself called every
turn by `build_system_prompt_with_ab` (`turn_system_prompt.rs:64`).
There is no caching layer that could short-circuit the call. ✓.

### Finding 4.3 — `min_trust_grade` IS enforced at SQL bind 🟢 (proof of correctness)

The `available_for_worker` query at
`store_available.rs:55-70` includes:

```sql
AND (s.min_trust_grade IS NULL OR ? >= s.min_trust_grade)
```

with the worker's `trust_grade` bound via `i64::from(grade)`. The
gate is not a post-fetch filter — it's pushed to the database. A
worker with `trust_grade = 0` (the default for a fresh worker per
the comment in `store_available.rs:30-33`) only sees skills with
`min_trust_grade IS NULL OR min_trust_grade <= 0`. ✓.

### Finding 4.4 — `apply_canary_routing` reads `skill_canary` workspace-globally 🟡 P1

**File**: `crates/argo-skill/src/canary.rs:184-194` and
`crates/argo-agent-runner/src/turn_canary.rs:23-30`.

`pick_canary_for_skill` reads `skill_canary WHERE skill_id = ?`.
Same as §2.3 — skill_id is workspace-global so canary state is
workspace-global. Tenant A flipping a canary in one workspace
affects every other tenant in the same workspace.

Severity: **P1** — a tenant cannot scope its own canary rollout
plan; opt-in/opt-out is workspace-wide.

Covered by the same `fix-skill-table-tenant-column` brief in §2.3.

---

## §5 — Memory namespace honoring

### Finding 5.1 — `MemoryNs::WorkerSelf` IS enforced at SQL bind 🟢 (proof of correctness)

**File**: `crates/argo-memory/src/layers.rs:257-274`. The
`layer_rows` query binds three tenant-scoped columns:

```sql
WHERE m.tenant_id = ? AND m.worker_id = ? AND m.memory_scope = ?
  AND m.layer = ? AND m.superseded_by IS NULL
```

with `ns.tenant_id` and `ns.worker_id` bound as BLOBs. The runner's
`turn_memory::compose_worker_memory` constructs the namespace from
`session.tenant_id + session.worker_id + MemoryScope::WorkerSelf`
(`turn_memory.rs:84-86`), so per-turn memory reads are tenant-scoped
at the database layer. ✓ — round-8 P1 gap is closed.

### Finding 5.2 — `read_shared_team` IS tenant-bound 🟢 (proof of correctness)

**File**: `crates/argo-memory/src/shared_team.rs:90-105`. SELECT
binds `tenant_id` explicitly. ✓.

### Finding 5.3 — Memory env-var `ARGO_MEMORY_LAYERS` is process-global 🟡 P3

**File**: `crates/argo-agent-runner/src/turn_memory.rs:14-39`.

`PromptMemoryMode::from_env()` reads `ARGO_MEMORY_LAYERS` per call.
This is per-process, not per-tenant. In a multi-tenant gateway,
every tenant gets the same composition mode (Auto/Shared/Worker/
Both). Operator may want per-tenant or per-workspace control.

Severity: **P3** — config-shape ergonomics, not a leak.

**Brief**: `fix-memory-layers-per-tenant-config` — promote the env
var into `WorkspaceConfig` and resolve per-session.

---

## §6 — Executor backend coverage

The runner dispatches to **two backends**:

1. **Builtin tools** — registered via
   `argo_tool::builtin_tools_with_runtime(cwd, memory, memory_ns,
   orchestrator, store)` and routed through `ToolRegistry::dispatch`
   in `dispatch.rs:179-186`.
2. **MCP servers** — registered via
   `register_manifest_mcp_servers` (per-vertical) +
   `register_mcp_server` (per-call discovery), routed through
   `dispatch_mcp` in `dispatch.rs:191-225`.

### Finding 6.1 — Computer-use is NOT dispatched from the runner 🟢 (architectural — by design)

`crates/argo-agent-runner/Cargo.toml` has **no** dependency on
`argo-executor-computer-use*`. `rg -rn 'argo_executor_computer'
crates/` shows no callers of computer-use inside the runner.
Computer-use executors are invoked from
`crates/argo-cli/src/handlers/worker_actions_executor_cu_*.rs` —
above the runner layer, as part of the per-action executor
selector that mission claim 5 requires.

Mission claim 5 ("any-PC execution") is therefore satisfied at the
**worker-action** layer, not the per-turn-LLM-dispatch layer. The
runner correctly knows only about LLM-tool-call dispatch
(MCP + builtin). This is by design.

Severity: **none** — proof of architectural separation. Listing
explicitly so future audits don't re-flag.

### Finding 6.2 — MCP dispatch is tenant-scoped via pool key 🟢 (proof of correctness)

**Files**: `dispatch.rs:48-64` (`McpConnectionPool::client`),
`dispatch.rs:212` (`dispatch_mcp` passes `ctx.tenant_id`),
`dispatch.rs:161-188` (`dispatch_one_with_tenant` accepts an
explicit `tenant_id`).

`McpConnectionPool::client` acquires from the underlying
`argo-mcp::McpClientPool` keyed by `(tenant_id, server_name)`.
Existing test
`dispatch_tenant_isolation_tests.rs::mcp_pool_isolates_same_server_by_tenant`
verifies that two tenants get distinct subprocesses for the same
server name. ✓.

### Finding 6.3 — Manifest MCP servers are registered process-globally 🟡 P2

**File**: `dispatch.rs:106-126` (`register_manifest_mcp_servers`).

`build_dispatch_ctx` runs once per turn and re-loads every
vertical's MCP servers into the per-turn `ToolRegistry`. There is
no per-tenant filtering: tenant B's worker bound to vertical V can
see MCP servers registered by every vertical loaded in the
workspace's vertical-dir. The tenant guard kicks in only at the
pool acquisition phase (Finding 6.2) — subprocess startup *is*
isolated, but the **tool schema** advertised to the model is the
union of every vertical's MCP servers in the workspace.

Severity: **P2** — schema leak (tool names visible to the model
across verticals) but not a runtime credential leak.

**Brief**: `fix-mcp-server-registration-per-vertical` — filter
`register_manifest_mcp_servers` by the active `session.vertical_id`
so only the bound vertical's MCP servers join the registry.

### Finding 6.4 — Cu-macos tenant pre-flight (per #2076) does NOT cover the runner path 🟢 (out of scope, but worth recording)

#2076 added tenant-mismatch pre-flight in
`crates/argo-executor-computer-use-macos`. Since the runner never
calls computer-use (Finding 6.1), the runner is not the gate-keeper
for this primitive. The worker-action layer at
`argo-cli/handlers/worker_actions_executor_cu_macos.rs` is. Listing
to close the §6 acceptance criterion.

---

## §7 — Failure modes summary

Each row: **mode → response → severity → recommended brief**.

| Failure mode | Current behavior | Severity | Brief |
|---|---|---|---|
| Cross-tenant A/B leak (skill_ab_test_run) | Cross-tenant pick / corrupt outcome | 🔴 P0 | `fix-skill-ab-test-run-tenant-bind` |
| Skill table workspace-global (no tenant_id) | Tenant B sees tenant A skills + canary + migration in same workspace | 🔴 P0 | `fix-skill-table-tenant-column` (manager-worker, 3 children) |
| Subtask ledger unbound by tenant | Defense-in-depth gap | 🟡 P2 | `fix-subtask-ledger-tenant-bind` |
| Audit write batched, NOT in-tx with mutation | Orphan rows possible on crash | 🟡 P1 | `fix-runner-audit-transactional` |
| Pre-turn freeze produces no audit row | Forensic gap | 🟡 P1 | `fix-pre-turn-freeze-audit` |
| TurnEnd/Halt/RoutingDecision audit-write failures swallowed | Lost observability | 🟡 P2 | `fix-runner-audit-emit-metrics` |
| MCP server registration not per-vertical | Tool-schema leak across verticals (same tenant) | 🟡 P2 | `fix-mcp-server-registration-per-vertical` |
| ARGO_MEMORY_LAYERS env-var is process-global | No per-tenant memory mode | 🟢 P3 | `fix-memory-layers-per-tenant-config` |
| Skill canary load error returns hard `ArgoError::Schema` (`turn_canary.rs:46-52`) | Hard-aborts the turn; inconsistent with rest of soft-fail policy | 🟡 P2 | `fix-canary-load-soft-fail` |
| Planner hook all failure modes swallow into `DecomposeOutcome::*Error` | Decompose silently no-ops; logged at warn | 🟢 by-design | n/a — documented contract |
| Skill A/B `record_outcome` failures swallowed | Side channel | 🟢 by-design | n/a — documented in brief |
| Memory `compose_context` errors warn+skip | Soft-fail | 🟢 by-design | n/a |
| Pre-turn budget gate aborts with `BudgetExhausted` + audit Halt | Recoverable; observable; atomic | 🟢 | n/a — verified |

### Recovery / observability assessment

- **Recoverable**: budget-exhaust + `stopped_on_tool_budget = true`
  (worker step loop continues), grant-consumption failures (turn
  re-issues block on next dispatch), planner errors (single-task
  fallback), skill-canary load failure (currently NOT recoverable
  — see brief above).
- **Observable**: every recoverable failure produces either an
  audit row, a `tracing::warn!`, or both. Gap: the per-turn freeze
  rejection (Finding 3.2) is observable only via the `ArgoError`
  return; no audit/log row at the runner layer.
- **Atomic / rolled back**: NONE of the per-round mutation+audit
  pairs are atomic; see Finding 3.1. Pre-turn budget gate IS atomic
  because it runs before any mutation.

---

## Acceptance-criteria checklist

- [x] Report covers all 7 sections.
- [x] Every finding has severity + file:line + brief-shape
  recommendation.
- [x] Cross-tenant negative test added as `#[ignore]`'d
  FINDINGS-ONLY tests in
  `crates/argo-agent-runner/tests/eval_findings_tenant_isolation.rs`
  + `eval_findings_audit_chain.rs`.
- [x] `mission-vs-code-gap-manifest.md` updated: argo-agent-runner
  row 🔴 → 🟢.
- [x] Each recommended brief is actionable + scoped <500 LOC (the
  skill-table tenant-column brief is decomposed into 3 children).
- [x] BUILD_STATUS.md to be flipped to IN_REVIEW with this report
  path.

---

## Recommended brief queue (priority order)

1. **🔴 P0** `fix-skill-table-tenant-column` (manager-worker, 3
   children) — schema + cascade.
2. **🔴 P0** `fix-skill-ab-test-run-tenant-bind` — schema + 2
   call-sites + cross-tenant negative test.
3. **🟡 P1** `fix-runner-audit-transactional` — wire
   `audit.write_in_tx` into 4 per-round mutation+audit pairs.
4. **🟡 P1** `fix-pre-turn-freeze-audit` — emit
   `WorkerFreezeBlocked` at pre-turn gate.
5. **🟡 P2** `fix-subtask-ledger-tenant-bind` — defense-in-depth.
6. **🟡 P2** `fix-mcp-server-registration-per-vertical` — schema
   leak fix.
7. **🟡 P2** `fix-canary-load-soft-fail` — align with rest of
   soft-fail policy.
8. **🟡 P2** `fix-runner-audit-emit-metrics` — observability hook
   for swallowed audit writes.
9. **🟢 P3** `fix-memory-layers-per-tenant-config` — config
   ergonomics.

The conductor can queue these as a wave or interleave with
in-flight work. The first two are the multiplier — they each close
a P0 surface that round-8 explicitly flagged as
mission-claim-blocking.

---

## Notes on the audit method

This audit is `cargo`-clippy-clean against `origin/main`
`5a8726465` plus a one-night reading of every `*.rs` file in
`crates/argo-agent-runner/src/`. Citations are file:line against
the same commit. Where downstream crates were touched (argo-skill,
argo-memory, argo-store, argo-audit) the line citations are for
the SQL query body; the runner is the consumer.

Two findings (3.1 audit-write-batching, 3.2 pre-turn freeze) were
not in the round-8 manifest and surfaced from this audit. The
other findings either align with existing manifest rows (round-8
called out skill-table tenant gap and subtask-ledger as known
gaps) or extend them with the specific file:line and brief shape.
