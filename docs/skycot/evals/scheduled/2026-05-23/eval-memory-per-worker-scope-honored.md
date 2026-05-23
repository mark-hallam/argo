# eval-memory-per-worker-scope-honored — findings

**Brief**: `docs/build-plan/streams/eval-memory-per-worker-scope-honored.md`
**Verdict**: 🟢 **AUDITED — substrate honors per-worker scope.**
**Audited at**: origin/main `6056b48cb` on 2026-05-23
**Author**: claude
**Tests added**: `crates/argo-memory/tests/per_worker_scope_test.rs`
  (7 tests, all passing — see "Contract pins" below).

## TL;DR

The `MemoryStore` namespace-aware surface (`append_in_namespace*`,
`query`, `query_scored`, `compose_context`,
`run_nightly_summary_job`, `summary_exists`) filters reads and writes
by `(tenant_id, worker_id, memory_scope)` at the SQL bind layer.
Worker B inside the same tenant cannot read worker A's
`MemoryScope::WorkerSelf` rows; a `WorkerSelf` write is invisible to
the `Role` and `SharedTeam` scope reads against the same
tenant+worker tuple; the cross-tenant boundary holds even when two
workers accidentally share a `WorkerId`.

The legacy `MemoryStore::append` / `list_active` / `get` / `revert`
/ `supersede` / `set_lock` surface predates `MemoryNs` and binds only
`workspace_id` (+ entry id). Those callsites are not part of the
mission-claim-1 substrate contract — they remain workspace-scoped
APIs used by the CLI memory-file editor. See
"Legacy non-namespaced surface" below for the inventory + risk note.

No leak was found. **No fix-brief needed.** The gap-manifest row for
`argo-memory` flips 🔴 NOT AUDITED → 🟢 AUDITED.

## Read + write path walk

The per-worker SQL bind lives in two locations:

1. `crates/argo-memory/src/layers.rs:258-278` — `layer_rows`, the
   private helper that backs every namespace-scoped read:

   ```sql
   SELECT ...
   FROM memory_entry m
   LEFT JOIN memory_lock l ON l.entry_id = m.id
   LEFT JOIN memory_embedding e ON e.entry_id = m.id
   WHERE m.tenant_id = ?
     AND m.worker_id = ?
     AND m.memory_scope = ?
     AND m.layer = ?
     AND m.superseded_by IS NULL
   ORDER BY m.learned_at ASC
   ```

   Bound params (`layers.rs:270-273`):
   `ns.tenant_id.0.as_bytes()`, `ns.worker_id.0.as_bytes()`,
   `scope_as_str(ns.scope)`, `layer.as_str()`. The triple is the
   sole authority for which rows the caller sees.

2. `crates/argo-memory/src/layers.rs:110-128` —
   `append_in_namespace_with_layer`, the only namespace-aware write
   path. The INSERT writes `(tenant_id, worker_id, memory_scope)`
   alongside the entry body, so every later `layer_rows` read can
   filter the row in/out.

`append_in_namespace` (`layers.rs:77-94`) is a thin
`append_in_namespace_with_layer(.., MemoryLayer::Recent)` shim.

### Composite reads

- `query` / `query_scored` (`layers.rs:167-222`) → `layer_rows(ns,
  Indexed)`. Bound by the triple.
- `compose_context` (`layers.rs:225-239`) → `layered_entries(ns,
  Summary)` + `layered_entries(ns, Recent)` + `query_scored(ns,
  action_id, 24)`. All three read paths go through `layer_rows`.

### Summary job

- `run_nightly_summary_job` (`summary.rs:135-174`) discovers
  active worker namespaces via `summary_namespaces`
  (`summary.rs:176-211`), which `GROUP BY tenant_id, worker_id,
  memory_scope` against the per-month window — the grouping itself
  is the per-worker contract.
- `summary_exists` (`summary.rs:231-254`) binds the
  `(workspace_id, tenant_id, worker_id, memory_scope)` quad before
  the `body LIKE marker%` check, so the per-month
  one-summary-per-worker idempotence is per-tuple, not per-tenant.

## Verdict matrix

| Public `MemoryStore` method | Scope bind in SQL | Verdict | Notes |
|---|---|---|---|
| `new(store)` | n/a | 🟢 constructor | — |
| `append_in_namespace` | INSERT writes `(tenant, worker, scope)` | 🟢 honors scope | wraps `append_in_namespace_with_layer(_, Recent)` |
| `append_in_namespace_with_layer` | INSERT writes `(tenant, worker, scope, layer)` | 🟢 honors scope | layers.rs:110-128 |
| `promote_recent_older_than` / `promote_recent_before` | UPDATE filters by `learned_at` + `layer = 'recent'` only | 🟢 by design | global tier promotion job — runs cross-namespace by intent; subsequent reads remain filtered |
| `query` / `query_scored` | `layer_rows` bind `(tenant, worker, scope, layer = 'indexed')` | 🟢 honors scope | layers.rs:258-278 |
| `compose_context` | `layered_entries(ns, Summary)` + `layered_entries(ns, Recent)` + `query_scored` | 🟢 honors scope | 3-layer pack, all per-ns |
| `run_nightly_summary_job` | `summary_namespaces` GROUP BY `(tenant, worker, scope)`; per-ns iteration | 🟢 honors scope | summary.rs:135-211 |
| `write_shared_team` / `read_shared_team` | binds `tenant_id` only | 🟢 by design | parallel `shared_team_memory` table — tenant-wide handoff, no per-worker filter intended |
| `append` (legacy) | binds `workspace_id` | 🟡 non-namespaced — see below | workspace-file editor path; pre-MemoryNs |
| `supersede` / `revert` (legacy) | derives `workspace_id` + `file` from prev entry | 🟡 non-namespaced — see below | mutates legacy rows only |
| `set_lock` (legacy) | binds `entry_id` only | 🟡 non-namespaced — see below | per-entry, lock is row-level |
| `list_active` (legacy) | binds `workspace_id` + `file` | 🟡 non-namespaced — see below | workspace-file editor read |
| `versions_for_entry` (legacy) | derives `workspace_id` + `file` from head | 🟡 non-namespaced — see below | per-file history |
| `get` (legacy) | binds `entry_id` only | 🟡 non-namespaced — see below | row-level fetch |
| `append_for_vertical` / `list_for_vertical` / `list_skill_candidates_for_vertical` | binds `workspace_id` + `file` + `vertical_id` | 🟢 by design | vertical-scoped (#680 Rule 7) — orthogonal to per-worker scope |

### Legacy non-namespaced surface

The legacy `MemoryStore::append` / `list_active` / `supersede` /
`revert` / `set_lock` / `versions_for_entry` / `get` group binds
only `workspace_id` (or the entry id directly). These predate
`MemoryNs` and are still used by:

- `argo memory append/list/diff/lock-set/lock-clear/revert` CLI
  handlers (`crates/argo-cli/src/handlers/memory.rs`) — the
  human-driven workspace-file editor surface.
- `argo_memory::append_with_class` (`lib.rs:83`) — classification
  wrapper used by tagging callers.

These paths bind `workspace_id`, which already carries an effective
tenant boundary because `workspace.tenant_id` is set at creation
(`crates/argo-store/src/migrations/v1.rs` workspace table). Cross-
tenant isolation is enforced one level up at the workspace boundary.

**Risk**: the legacy paths do not write `tenant_id` / `worker_id` /
`memory_scope` columns, so rows landed via them are invisible to the
namespace-aware reads (`compose_context`, `query_scored`,
`run_nightly_summary_job`) — those queries filter
`memory_scope IS NOT NULL` (via the GROUP BY) and would skip legacy
rows. This is the documented intent: the CLI editor is a
workspace-level surface (one "MEMORY.md per workspace"), and
namespace-aware writes are the worker-runtime surface. Mixing is
not load-bearing for mission-claim-1 today.

If the legacy surface ever needs to feed the worker runtime, the fix
is to add a `MemoryNs` parameter to `append` (and friends) and write
the per-worker columns. Out of scope for this audit.

## Contract pins (added by this brief)

`crates/argo-memory/tests/per_worker_scope_test.rs` — 7 tests, all
passing locally on `argo-memory` test pass (1.15s elapsed):

1. `cross_worker_query_same_tenant_returns_empty` — worker B in the
   same tenant cannot see worker A's WorkerSelf indexed row.
2. `cross_worker_compose_context_same_tenant_excludes_other_worker_rows`
   — pack_b.entries is empty when only worker A has seeded Recent +
   Indexed + Summary rows.
3. `cross_tenant_worker_self_query_returns_empty_even_when_worker_id_repeats`
   — tenant_b reading with an identical `WorkerId` to tenant_a's
   row gets `Vec::new()`.
4. `worker_self_writes_invisible_to_shared_team_scope_read_on_same_ns`
   — WorkerSelf write is filtered out under SharedTeam scope read
   for the same (tenant, worker) tuple.
5. `worker_self_writes_invisible_to_role_scope_read_on_same_ns` —
   same pin against the Role scope.
6. `shared_team_table_isolates_by_tenant_independent_of_worker_self_scope`
   — `shared_team_memory.tenant_id` bind keeps tenants apart even
   when private `worker_self` rows also exist.
7. `nightly_summary_groups_one_summary_per_worker_in_same_tenant` —
   `summary_namespaces` GROUP BY emits one summary row per
   `(tenant, worker, scope)` triple inside one tenant.

## Boundary notes

- The integration test file uses `MemoryStore::new` over an
  `:memory:` `SessionStore`. No real Voyage-3 / Anthropic
  network calls: `Voyage3EmbeddingProvider` returns a deterministic
  hash-of-text vector by default (`embedding.rs:21-34`), and
  `HaikuSummaryProvider` returns a deterministic string when no LLM
  caller is wired in (`summary.rs:96-120`).
- Test #1 (`cross_worker_query_same_tenant_returns_empty`)
  partially overlaps with the existing proptest in
  `crates/argo-memory/src/layer_tests.rs:162-194` — that proptest
  pins the **cross-tenant** boundary; this audit's pin (#1) is the
  same-tenant cross-worker complement. Both are kept.
- `crates/argo-agent-runner/tests/memory_namespace_runtime_tests.rs`
  pins the same isolation at the **tool dispatch** layer (above
  the SQL bind). This audit pins it at the SQL bind layer (one
  level below). Both are kept — different defense-in-depth surfaces.

## Recommended follow-up

None at the substrate level. The argo-memory row flips to
🟢 AUDITED in the gap manifest with no associated fix-brief.

If future scope expansion calls the legacy `append` from the worker
runtime, a fix-brief should add a `MemoryNs` overload that writes
the per-worker columns — but that is conditional and not in flight
today.
