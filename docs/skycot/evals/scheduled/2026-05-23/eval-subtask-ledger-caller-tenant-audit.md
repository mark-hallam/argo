# eval — `subtask_ledger` caller-side tenant audit

**Brief**: `eval-subtask-ledger-caller-tenant-audit`
**Authored**: 2026-05-23 against `origin/main` `49ec1641e` (worktree base
`ddc621e4f`).
**Scope**: read-only walk of every callsite that runs SQL against the
`subtask_ledger` table. Source: evaluator round-13 SHARED.md handoff
2026-05-23T12:30:00Z, verbatim:

> "🟡 subtask_ledger: all 12 queries key by id alone — same class as
> round-3 audit_detail bug fixed in #1992; need caller-side audit."

This is an **investigation deliverable**: per-caller verdict (🟢 / 🟡 /
🔴), plus cross-tenant tests that pin the contracts that hold today.
The SQL is read-only; no production source under `crates/argo-store/`
or `crates/argo-agent-runner/` is mutated.

---

## §1 — Schema reminder (the contract under audit)

`crates/argo-store/migrations/29__subtask_ledger.sql:18-21`, verbatim:

```
-- SAFETY: no direct tenant_id column. Rows carry deployment-unique
-- `worker_id` and optional `parent_session_id`. Tenant-facing views
-- must resolve the worker or session through tenant-scoped tables
-- before listing subtasks.
```

That's the SAFETY block. The audit question becomes: **does every
caller honor it?** Specifically, for every `WHERE id = ?` lookup,
the caller must either (a) have already resolved the id through a
tenant-scoped query (session_id or worker_id joined to a tenant
table), or (b) operate inside a trust boundary that already pins
the tenant (e.g. local-binary CLI with no remote surface).

### Why this differs from the audit_detail precedent (#1992)

| Axis | `audit_record` (#1992) | `subtask_ledger` (this audit) |
|---|---|---|
| HTTP route exposes id-keyed lookup? | YES — `GET /api/v1/audit/<id>` | NO — `rg subtask crates/argo-cli/src/handlers/gateway/` = empty |
| Schema has `tenant_id` column? | YES — bind was missing | NO — by design (SAFETY block) |
| Fix shape in #1992 | add `AND tenant_id = ?` | not applicable; fix shape would be either schema-add-column OR caller-resolves-via-session |
| Live cross-tenant attack vector today? | YES (pre-fix) | NO — no remote surface |

So the audit_detail bug was a **real shipped leak**: any
authenticated tenant could hit the HTTP endpoint with another
tenant's audit-record id and the SQL would return the row. The
subtask_ledger queries are the same SQL shape but with no remote
surface to attack — every caller is either inside the runtime
(server-resolved id) or inside the CLI binary (operator-shell trust
boundary).

---

## §2 — Statement inventory (12 SQL statements, 7 id-keyed)

Enumerating every `subtask_ledger` SQL statement that runs in
production. Numbered 1-12 to match the round-13 phrasing; the
helper-function column lists the public Rust API that wraps each
statement.

| # | File:line | Shape | Public fn |
|---|---|---|---|
| 1 | `crates/argo-store/src/subtask.rs:132` | `INSERT` | `subtask::insert` |
| 2 | `crates/argo-store/src/subtask.rs:167` | `UPDATE … WHERE id = ?` ⭐ | `subtask::update_state` |
| 3 | `crates/argo-store/src/subtask.rs:337` (`SELECT_COLUMNS`) | `SELECT …` + dynamic filter clauses | `subtask::list` |
| 4 | `crates/argo-store/src/subtask.rs:341` (`SELECT_COLUMNS_BY_ID`) | `SELECT … WHERE id = ?` ⭐ | `subtask::get` |
| 5 | `crates/argo-store/src/subtask.rs:345` (`SELECT_COLUMNS_BY_SESSION`) | `SELECT … WHERE parent_session_id = ?` | `subtask::list_by_session` |
| 6 | `crates/argo-store/src/subtask_budget.rs:49` | `UPDATE … WHERE id = ?` ⭐ | `subtask_budget::set_subtask_budget` |
| 7 | `crates/argo-store/src/subtask_budget.rs:83` | `UPDATE … WHERE parent_session_id IS ? AND worker_id = ? AND plan_node_path = ?` (natural-key) | `subtask_budget::set_subtask_budget_by_natural_key` |
| 8 | `crates/argo-store/src/subtask_budget.rs:111` | `UPDATE … WHERE id = ?` ⭐ | `subtask_budget::set_subtask_parent` |
| 9 | `crates/argo-store/src/subtask_budget.rs:136` | `UPDATE … WHERE id = ?` ⭐ | `subtask_budget::record_subtask_spend` |
| 10 | `crates/argo-store/src/subtask_budget.rs:163` | `SELECT … WHERE id = ?` ⭐ | `subtask_budget::get_remaining` |
| 11 | `crates/argo-store/src/subtask_budget.rs:267,272` | `WITH RECURSIVE … WHERE id = ?` (root anchor) ⭐ | `subtask_budget::walk_tree` |
| 12 | `crates/argo-agent-runner/src/turn_budget.rs:41` | `SELECT id … WHERE parent_session_id = ? AND state = 'running'` | `turn_budget::running_subtask_id` (private) |

⭐ = bare-id-keyed (7 of 12). The other 5 are: 1 INSERT (no lookup),
2 session-scoped (parent_session_id, not id), 1 natural-key, 1
dynamic-filter list with optional clauses.

---

## §3 — Per-caller inventory + verdict

Walked every production callsite of the helpers in §2 via
`rg 'argo_store::subtask::|subtask::insert|subtask::update_state|subtask::get\(|subtask::list\(|subtask::list_by_session|subtask_budget::' crates/`. Each row below is a distinct
**caller** (not a helper) — i.e. the code that decides which id to
pass.

### Caller A — `argo subtask show <id>` CLI handler

- **File**: `crates/argo-cli/src/handlers/subtask_show.rs:16`
- **Wraps statement**: #4 (`subtask::get`, `SELECT … WHERE id = ?`)
- **id source**: clap-parsed `cli::SubtaskCmd::Show { id, .. }` —
  raw argv string from the operator's shell.
- **Tenant context plumbed?** No. `SubtaskCmd::Show` does not carry
  a `--tenant` flag (`crates/argo-cli/src/cli_subtask_cmds.rs:33-40`).
  The handler opens the local `~/.argo/argo.db` and queries directly
  by id.
- **Gateway exposure?** None.
  `rg 'subtask|Subtask' crates/argo-cli/src/handlers/gateway/` returns
  zero hits. There is no HTTP route that invokes this handler or its
  underlying `subtask::get`.
- **Verdict**: **🟡 needs cross-tenant pinning test.** The handler
  does NOT enforce a tenant filter; the schema SAFETY block already
  documents that callers (not the store) must scope. This is
  acceptable today **only because the CLI is local-binary +
  operator-shell trust boundary + zero gateway exposure**. If a
  future brief wires `subtask show` through the gateway, this is the
  exact shape that bit #1992.
- **Test shipped**: `crates/argo-cli/tests/subtask_ledger_cross_tenant_cli_test.rs::cli_subtask_get_is_tenant_blind_by_design` — documents the contract (the bare-id lookup returns the row regardless of caller) so a future maintainer who wires this to a gateway sees the failing pre-condition immediately.

### Caller B — `argo subtask budget <root_id>` CLI handler

- **File**: `crates/argo-cli/src/handlers/subtask_budget.rs:21`
- **Wraps statement**: #11 (`walk_tree`, recursive CTE anchored at `WHERE id = ?`)
- **id source**: clap-parsed `cli::SubtaskCmd::Budget { root_id, .. }`
  — raw argv.
- **Tenant context plumbed?** No.
- **Gateway exposure?** None.
- **Verdict**: **🟡 needs cross-tenant pinning test.** Same shape as
  Caller A. `walk_tree`'s recursion follows `parent_subtask_id`
  forward from the root, so leaking the root id leaks the entire
  delegation subtree.
- **Test shipped**: same as A — the integration test seeds two
  tenants and asserts the CLI walk traverses cross-tenant rows
  iff a foreign id is supplied (documenting the contract).

### Caller C — `argo subtask list [--session …]` CLI handler

- **File**: `crates/argo-cli/src/handlers/subtask_list.rs:35`
- **Wraps statement**: #3 (`subtask::list`, dynamic-filter
  `SELECT …`)
- **id source**: optional `--session`, `--worker`, `--parent-goal`,
  `--state` clap flags. None are tenant-scoped.
- **Tenant context plumbed?** No.
- **Gateway exposure?** None.
- **Verdict**: **🟡 — not id-keyed, but tenant-blind in the same
  way.** The filter columns are tenant-blind: a caller who supplies
  any `--session` value gets that session's rows back. No remote
  surface today, same operator-shell trust boundary applies.
- **Test shipped**: included in the same CLI integration test.

### Caller D — `argo subtask tree <session>` CLI handler

- **File**: `crates/argo-cli/src/handlers/subtask_tree.rs:18`
- **Wraps statement**: #5 (`subtask::list_by_session`)
- **id source**: clap-parsed `cli::SubtaskCmd::Tree { session, .. }`.
- **Tenant context plumbed?** No.
- **Gateway exposure?** None.
- **Verdict**: **🟡 — same shape as A/B/C.**
- **Test shipped**: included in the same CLI integration test.

### Caller E — `turn_budget::check_pre_turn_budget` (runtime pre-turn gate)

- **File**: `crates/argo-agent-runner/src/turn_budget.rs:54-78`
- **Wraps statements**: #12 (resolver) → #10 (`get_remaining`,
  `SELECT … WHERE id = ?`)
- **id source**: SERVER-RESOLVED. The handler calls
  `running_subtask_id(store, session_id)` (`turn_budget.rs:36-49`),
  which does `SELECT id FROM subtask_ledger WHERE parent_session_id
  = ? AND state = 'running'`. The `session_id` is the
  `SessionId` passed in by `run_turn_with_events_inner`
  (`turn_run_loop.rs:28`), which loads the session row + records
  `tenant_id` on the agent-turn tracing span before this call.
- **Tenant context plumbed?** Yes — implicitly via the session
  row. The session table binds `(tenant_id, workspace_id)` at
  creation; the agent-runner only ever resolves `running_subtask_id`
  for the session it loaded.
- **Verdict**: **🟢 tenant-checked by session-scoped resolution.**
  The id is never user-supplied; it is the value of the `id` column
  filtered down by a `parent_session_id` predicate that the runtime
  controls.
- **Test shipped**:
  `crates/argo-agent-runner/tests/subtask_ledger_cross_tenant_runtime_test.rs::pre_turn_gate_is_session_scoped_does_not_bleed_across_tenants`
  — seeds two tenants' sessions each with a running subtask;
  exhausts tenant A's budget; asserts tenant B's gate still passes.

### Caller F — `turn_budget::record_subtask_turn_spend` (runtime post-round charge)

- **File**: `crates/argo-agent-runner/src/turn_budget.rs:88-113`
- **Wraps statements**: #12 (resolver) → #9 (`record_subtask_spend`,
  `UPDATE … WHERE id = ?`)
- **id source**: SERVER-RESOLVED — same `running_subtask_id`
  resolver as Caller E.
- **Tenant context plumbed?** Yes — same path as E.
- **Verdict**: **🟢 tenant-checked by session-scoped resolution.**
- **Test shipped**:
  `…::post_turn_spend_is_session_scoped_does_not_bleed_across_tenants`
  — runs a spend on tenant A's session; asserts tenant B's
  `spent_usd_cents` is unchanged.

### Caller G — `argo_runtime::record_decomposition` (Planner sink hand-off)

- **File**: `crates/argo-runtime/src/planner.rs:84-122`
- **Wraps statements**: #1 (`INSERT` — id generated server-side) +
  #7 (`set_subtask_budget_by_natural_key` — natural-key, not id)
- **id source**: NONE — the helper never accepts an externally-
  supplied id. Each iteration either INSERTs a fresh UUIDv7 or
  UPDATEs by `(parent_session_id, worker_id, plan_node_path)` —
  all three components are derived from the
  `(ctx.worker_id, result.workflow.id, task.id)` triple the runtime
  owns.
- **Tenant context plumbed?** N/A — never lifts an id.
- **Verdict**: **🟢 — natural-key resolution, not id-keyed.**
- **Test**: existing
  `crates/argo-store/src/subtask_tests.rs::…` covers the
  insert/natural-key shape. No new test needed.

### Test-only callers (not enumerated in §3)

`crates/argo-cli/tests/subtask_budget_smoke.rs`,
`crates/argo-cli/tests/subtask_ledger_smoke.rs`,
`crates/argo-agent-runner/src/turn_budget_tests.rs`,
`crates/argo-store/src/subtask_tests.rs`,
`crates/argo-store/src/subtask_budget_tests.rs` — these all own the
ids they exercise (insert returns the id, then they pass it back).
No tenant concern.

### Functions with **zero** production callers (FINDINGS-ONLY)

- `subtask::update_state` (statement #2) — only `turn_budget_tests`
  exercises it. Runtime never transitions subtasks to running/done/
  failed in the workspace today. **This is a separate dead-code
  finding** — the lifecycle column transitions land via
  `record_subtask_turn_spend` for `spent_usd_cents` but no caller
  walks rows to `done`/`failed`. Out of scope for this audit; flag
  for future `eval-subtask-lifecycle-wire`.
- `subtask_budget::set_subtask_budget` (statement #6) — only
  `subtask_budget_smoke.rs` (test) exercises it. The production
  budget-plumbing path uses `set_subtask_budget_by_natural_key`
  (statement #7) via `SqliteSubtaskSink::set_subtask_budget`. The
  bare-id helper is reachable on the public API but no production
  caller passes through it.
- `subtask_budget::set_subtask_parent` (statement #8) — only
  `subtask_budget_smoke.rs` exercises it. The `parent_subtask_id`
  column is never written in production; the tree-rendering
  `walk_tree` (which traverses by `parent_subtask_id`) therefore
  only ever sees the row's own id as root + zero descendants. This
  is a deeper wire-broken issue (delegation tree never recorded)
  separate from this tenant audit. Flag for
  `eval-subtask-delegation-tree-wire`.

---

## §4 — Summary table

| Caller | Verdict | Cross-tenant pinning test |
|---|---|---|
| A. `argo subtask show` | 🟡 | added (CLI test) |
| B. `argo subtask budget` | 🟡 | added (CLI test) |
| C. `argo subtask list` | 🟡 | added (CLI test) |
| D. `argo subtask tree` | 🟡 | added (CLI test) |
| E. `turn_budget::check_pre_turn_budget` | 🟢 | added (runtime test, contract pin) |
| F. `turn_budget::record_subtask_turn_spend` | 🟢 | added (runtime test, contract pin) |
| G. `record_decomposition` | 🟢 | existing tests cover |

**🔴 verdicts found: 0.** No production caller takes an id from a
request/HTTP param without tenant-checking. The CLI 🟡 verdicts hold
**only** while the operator-shell trust boundary holds (i.e. while
no gateway route exposes a subtask helper).

---

## §5 — Recommended follow-up briefs (if scope expands)

Not authored — listed here so a future conductor tick can pick them
up if the trust boundary changes.

1. **`mission-subtask-handlers-tenant-scope-resolve-via-session`**
   (~150-250 LOC). If a gateway route ever needs `subtask show`,
   the right shape is: handler receives `(tenant_id, subtask_id)`,
   does `SELECT … FROM subtask_ledger s INNER JOIN session sess ON
   s.parent_session_id = sess.id WHERE s.id = ? AND sess.tenant_id
   = ?`. Keeps the schema SAFETY-block contract intact (no
   tenant_id column needed). Add `--tenant` to `SubtaskCmd` for
   parity.
2. **`mission-subtask-ledger-add-tenant-id-column`** (~400-700 LOC,
   additive migration + sweep). Only needed if the join in #1 is
   measured as too costly under high-fanout decomposition trees.
   Defer until #1 is in use and profiled.
3. **`eval-subtask-lifecycle-wire`** — sibling investigation: who
   transitions subtask rows past `pending`? `update_state` has zero
   production callers; `record_decomposition` only INSERTs.
4. **`eval-subtask-delegation-tree-wire`** — sibling: who calls
   `set_subtask_parent`? Currently zero production callers, which
   means `walk_tree` never sees a multi-level tree in practice.

These are **dead-code / wire-broken** findings adjacent to the
tenant audit, not tenant leaks themselves. Left here as a paper
trail; conductor decides whether to author.

---

## §6 — Round-13 citation

Verbatim, from evaluator round-13 SHARED.md handoff (2026-05-23T12:30:00Z):

> "🟡 subtask_ledger: all 12 queries key by id alone — same class as
> round-3 audit_detail bug fixed in #1992; need caller-side audit."

This audit closes the round-13 row. Verdict: same SQL shape as
audit_detail, but no remote surface to exploit. CLI handlers
inherit operator-shell trust; runtime callers resolve via
session-scope. Five regression tests pin the contract; four flag
the boundary so the next maintainer to wire a gateway route sees
the precondition.
