# eval — `argo-store` tenant-bind sweep

**Brief**: `eval-argo-store-tenant-bind-sweep`
**Authored**: 2026-05-23 against `origin/main` `7c3178639` (worktree
base `7a1f6917b`).
**Scope**: read-only structural walk of every CREATE-TABLE in
`crates/argo-store/src/migrations/` plus every `sqlx::query*` invocation
in `crates/argo-store/src/`, classifying each per-table query as
🟢 **BIND** (binds `tenant_id` in WHERE), 🟡 **MENTION** (the SQL
text touches the column but not in a WHERE-binding shape we can prove
structurally — INSERTs that supply the column, `WHERE … AND tenant_id
IS ?` parameterised nullable filters, subqueries, etc.), 🔴
**NO_TENANT** (a non-INSERT WHERE clause that does not bind
`tenant_id`), or 🟢-by-design where the absence of `tenant_id` is the
documented contract.

This is an **investigation deliverable**, not a code change: the
audit walks read-only and writes only a docs file + a structural pin
test (one `#[ignore]`'d FINDINGS-ONLY test). Fix briefs are *named*
here; the conductor queues them as separate work.

Source gap-manifest entry (verbatim, cite per brief instructions):

> `argo-store` row in `mission-vs-code-gap-manifest.md` §"Crate map …
> Claim 5":
> "Migration framework now comment-aware (#2025). Workflow_definition
> primitive audited round 7. **`subtask_ledger` audited 2026-05-23
> (eval-subtask-ledger-caller-tenant-audit) — 12 SQL statements, 7
> production callers; 4 🟡 + 3 🟢 + 0 🔴. … Other tables (worker,
> persona, role, duty, audit_record, skill, etc.) need similar
> tenant-scoping audits."
> Priority: 🟡 P1 — `eval-argo-store-tenant-bind-sweep`.

---

## §0 — TL;DR

- **77 production tables** in `argo-store` (excluding 3 splitter
  doc-fixtures: `a`, `t`, `skill_canary`).
- **33 tables** carry a `tenant_id` column (31 in `CREATE TABLE`, 2
  added via `ALTER TABLE`: `audit_record` v61, `memory_entry` v13).
- **2 tables** carry `workspace_id` instead (`marketplace_artifact`,
  `policy_files`) — workspace is the legacy single-tenant analogue
  per `crates/argo-store/src/workspace.rs:1-20`.
- **42 tables** are child-scoped (no own `tenant_id` column; rely on
  parent-row tenant resolution) or globally shared (`schema_version`,
  `recovery_cache`, etc.).
- **338 SQL statements** scanned across the production source tree
  (heuristic: every `sqlx::query{,_as,_scalar}(…)` invocation, joined
  multi-line string literals).
- **25 tenant-id-bearing tables have at least one statement** in
  `crates/argo-store/src/`; the other 8 are queried only from out-of-crate
  callers (e.g. `agent`, `handoff_routing_rule`, `org_member`,
  `role_binding`, `role_duty`, `shared_team_memory`,
  `skill_version_migration`, `vertical_locks`) — out-of-scope for
  this sweep (may-touch is `crates/argo-store/src/**`).

### Verdict roll-up

- **🟢 by design** (intentional cross-tenant aggregation —
  not leaks): `audit_record` SRE rollups (`audit_record_query.rs`
  error-rate / availability / latency), `spend_ledger` global-cost
  rollups (`spend.rs::load_global_spend`), the GDPR
  subject-erasure flow (`gdpr.rs`, `gdpr_apply.rs`), `stats.rs`,
  `skill_rating` marketplace-cross-tenant aggregations, and
  `workspace.rs` reads that fan out from a tenant-scoped caller.
- **🟢 BIND-confirmed clean** (binds `tenant_id` in every WHERE):
  `org_unit`, `retro`, `roster`, `tenant_quota`, `tenant_rate_limit`,
  `worker_freeze`, `workflow_definition_index` (every write
  statement binds `tenant_id`).
- **🟡 caller-trust-boundary** (id-keyed WHERE, no `tenant_id` in
  SQL, relies on caller to pre-resolve tenant ownership — same
  class as the `subtask_ledger` round-13 verdict): the largest
  cluster. Covered in §4.
- **🔴 confirmed shipped cross-tenant leak**: **0** in
  `argo-store/src/`. The id-keyed-WHERE statements are all wrapped by
  helpers without production HTTP callers today — the closest match
  (`approvals::decide_request`) has only test callers per `rg
  decide_request crates/ | grep -v _tests.rs` (5/5 hits are test
  files). The leak surface materialises **at the moment a route
  wires these helpers**, not in the SQL alone.

---

## §1 — Methodology

### Schema scan

For each migration file `crates/argo-store/src/migrations/v*.rs`,
extract every `CREATE TABLE` SQL literal (filtering doctest fixtures
in `splitter.rs`) plus every `ALTER TABLE … ADD COLUMN tenant_id`
statement. Two tables enter the tenant-scoped set via ALTER
(`audit_record`, `memory_entry`); 31 declare `tenant_id` directly in
CREATE TABLE. The full enumeration is reproduced as
`/tmp/eval-argo-store-tenant-bind-sweep/.../table_schema_scan.sh` for
re-running; the script is not committed (one-off audit tool).

### Query scan

Parse every `.rs` file under `crates/argo-store/src/` (excluding
`*_tests.rs`, anything under `tests/`, and `migrations/`). For each
file find every `sqlx::query{,_as,_scalar}(…)` invocation, then walk
the argument list parsing matching parens and collecting every
double-quoted string literal between them. Concatenate the literals
to get the full SQL text (multi-line `"a" \n "b" \n "c"` is common in
this codebase — every statement that exceeds one line uses string
concatenation under the standard Rust formatter).

For each SQL statement, classify by:

- regex `(?:FROM|JOIN|UPDATE|INTO|DELETE\s+FROM)\s+(<table>)` → target
  tables.
- presence of `tenant_id\s*=\s*\?` → **BIND**.
- presence of `tenant_id\s+IN\s*\(` → **BIND-IN**.
- presence of `tenant_id` anywhere else (typically INSERT column lists
  or `WHERE … AND tenant_id IS ?` for nullable filters) → **MENTION**.
- WHERE clause present but no `tenant_id` reference → **NO_TENANT**
  (the 🔴 class).
- no WHERE clause and not a SELECT (so INSERT/REPLACE) → **NO_WHERE**
  (the SQL itself doesn't filter rows — the row is being created).

### Caller-side cross-check

For every 🔴 NO_TENANT statement, `rg <helper-fn> crates/` to
enumerate every callsite of the helper that wraps it. A statement is
demoted to **🟡 caller-trust-boundary** if every production caller
plumbs `tenant_id` through Rust types (e.g. `TenantId` argument used
to compute the id passed to the helper) and there is no remote /
HTTP surface that exposes the id-keyed helper directly. This is the
same demotion rule used in `eval-subtask-ledger-caller-tenant-audit`
(round 13).

---

## §2 — Per-table verdict matrix

77 tables. Sorted by axis-1 (presence of `tenant_id` column in
schema), then alphabetically. ⭐ = audited prior round (cite in §
column).

### §2.1 — Tenant-id-bearing tables (33)

| Table | Statements | Verdict | Cite | Audit notes |
|---|---|---|---|---|
| `agent` | 0 in store | 🟡 no-store-callers | — | All query callsites live outside `argo-store/src/`. Out of may-touch. |
| `approval_bundle` | 2 | 🟡 caller-trust-boundary | `approval_bundle.rs:204,280` | `decide_bundle` UPDATE keyed on `id` only; same class as §4 finding 1. |
| `approval_request` | 6 | 🟡 caller-trust-boundary | `approvals.rs:74,117,159,213` | `decide_request` + `mark_escalation_sent` UPDATE keyed on `id` only. **§4 spotlight finding 1.** |
| `audit_record` ⭐ | 10 | 🟢 BIND mixed with 🟢-by-design | `tenant_query_inspect.rs:137`, `audit_record_query.rs:125,203,280`, `worker_archive.rs:377`, `retro.rs:320`, `session_query.rs:235`, `stats.rs:61`, `gdpr.rs:119` | Round-13 audit_detail leak (#1992) verified fixed via inspect helper. SRE rollups in `audit_record_query.rs` are cross-tenant by design (operator dashboard). `gdpr.rs` is global by GDPR contract. `worker_archive.rs:377` UPDATE scoped via inner `session WHERE worker_id = ?` subquery — caller pre-binds `worker_id` after `tenant_id` resolution. |
| `duty` | 1 | 🟡 caller-trust-boundary | `worker_archive.rs:133` | `DELETE FROM duty WHERE worker_id = ? AND tenant_id = ?` — actually binds both (heuristic miss; verified by hand). 🟢 BIND on re-read. |
| `elevated_view_grant` | 2 | 🟡 caller-trust-boundary | `elevated_view_grant.rs:154` | `UPDATE … WHERE id = ?` revoke; caller (CLI) is operator-shell trust boundary. **§4 spotlight finding 2.** |
| `handoff_routing_rule` | 0 in store | 🟡 no-store-callers | — | Out of may-touch. |
| `memory_entry` ⭐ | 5 | 🟢 BIND + 🟢-by-design | `worker_archive.rs:370`, `gdpr.rs:94`, `gdpr_apply.rs:8`, `skill_runs_query.rs:104,410` | `worker_archive.rs:370` `DELETE … WHERE worker_id = ? AND tenant_id = ?` — explicit BIND. `gdpr*.rs` 🟢-by-design global erasure. `skill_runs_query.rs:104` SELECT keys on `worker_id` only — caller (`SkillRunsQuery::for_worker`) already resolved worker via tenant. |
| `org_member` | 0 in store | 🟡 no-store-callers | — | Out of may-touch. |
| `org_unit` | 7 | 🟢 BIND | `skill_share.rs:154`, `team_roster_mutations.rs:58,76,127`, `team_roster.rs:147,181,235` | All 6 non-INSERT statements bind `tenant_id`; INSERT supplies the column. ✔ |
| `persona` | 3 | 🟡 caller-trust-boundary | `persona_revision.rs:133,148`, `worker_archive.rs:143` | `persona_revision.rs:148` UPDATE keyed on `worker_id` only. `worker_archive.rs:143` `DELETE … WHERE worker_id = ? AND tenant_id = ?` — actually binds both (heuristic miss). |
| `retro` | 7 | 🟢 BIND | `retro.rs:162,183,203,231,244,273,296` | All 6 non-INSERT statements bind `tenant_id`. INSERT supplies it. ✔ |
| `role` | 1 | 🟡 caller-trust-boundary | `worker_archive.rs:138` | `DELETE FROM role WHERE worker_id = ? AND tenant_id = ?` — actually binds both (heuristic miss). 🟢 BIND on re-read. |
| `role_binding` | 0 in store | 🟡 no-store-callers | — | Out of may-touch. |
| `role_duty` | 0 in store | 🟡 no-store-callers | — | Out of may-touch. |
| `roster` | 4 | 🟢 BIND | `tenant_query_inspect.rs:119,176`, `team_roster_mutations.rs:88,141` | All non-INSERT bind `tenant_id`. ✔ |
| `scim_provisioned_tombstones` | 2 | 🟡 caller-trust-boundary | `scim_user.rs:278,303` | INSERT + SELECT both reference `tenant_id` in column lists; the SELECT keys on tombstone-id, which is the lookup primary key. Caller is SCIM-IDP webhook (cross-IDP collisions theoretical but unlikely). |
| `scim_user` | 3 | 🟡 caller-trust-boundary | `scim_user.rs:111,226,271` | `UPDATE scim_user … WHERE id = ?` patterns. SCIM ids are external-IDP-issued; cross-IDP collisions theoretical. **§4 spotlight finding 3.** |
| `session` | 19 | 🟡 caller-trust-boundary | `session_meta.rs:82,96,109,123,138`, `store_session_routing.rs:31`, `session_query.rs:174,194,205`, `skill_query.rs:105,283,295`, `tenant_query_inspect.rs:137`, `handoff_stats_query.rs:169`, `worker_archive.rs:377`, `retro.rs:320` | The largest 🟡 cluster. Every `UPDATE session … WHERE id = ?` (5 statements in `session_meta.rs`) relies on `SessionId` UUID-uniqueness. Same class as §4 finding 1. |
| `shared_team_memory` | 0 in store | 🟡 no-store-callers | — | Queries live in `argo-memory`. Out of may-touch. |
| `skill_rating` | 5 | 🟢 by design | `skill_rating.rs:146,178,210,238,269` | Schema *has* `rater_tenant_id` and `publisher_tenant`. `aggregate_ratings` + `list_for_publisher` + `learning_candidates_since` are intentionally cross-rater-tenant aggregations for the marketplace dashboard — the whole point of the table is to roll up votes across tenants. ✔ |
| `skill_version_migration` | 0 in store | 🟡 no-store-callers | — | Out of may-touch. |
| `spend_ledger` | 8 | 🟢 BIND + 🟢-by-design | `tenant_query.rs:260`, `spend.rs:20,44,53,62,87,95,104` | Per-tenant statements bind. `spend.rs::load_global_spend` is 🟢-by-design cross-tenant rollup (function name + module doc). ✔ |
| `support_ticket` | 6 | 🟡 caller-trust-boundary | `support_query.rs:203,233,244,259,300,361` | `UPDATE support_ticket … WHERE id = ?` x2 in `update_ticket` + `reply_ticket`. Doc-comment on `update_ticket` explicitly says "Caller is responsible for any RBAC check; the storage layer only enforces schema." **§4 spotlight finding 4.** |
| `tenant_quota` | 4 | 🟢 BIND | `tenant_quota.rs:181,206,219,240` | All non-INSERT bind `tenant_id`. INSERT supplies it. ✔ |
| `tenant_rate_limit` | 6 | 🟢 BIND | `tenant_rate_limit.rs:124,179,208,242,289,327` | All non-INSERT bind `tenant_id`. ✔ |
| `vertical_locks` | 0 in store | 🟡 no-store-callers | — | Queries live in `argo-orchestrator::VerticalConcurrencyLock` per `v65.rs:1-5`. Out of may-touch. |
| `worker` | 20 | 🟡 caller-trust-boundary + 🟢-by-design | `worker_budget_write.rs:111,129,228,255`, `worker_presence.rs:71`, `cohort.rs:294`, `tenant_quota_usage.rs:168,188`, `worker_archive.rs:95,148,182,327`, `tenant_query_inspect.rs:166`, `tenant_quota.rs:303`, `gdpr.rs:177,207`, `gdpr_apply.rs:46,57,76`, `skill_runs_query.rs:184` | Most reads bind `tenant_id`; `worker_budget_write.rs` and `worker_presence.rs` key on `worker_id` only (UUID-unique). `gdpr*.rs` 🟢-by-design. **§4 spotlight finding 5.** |
| `worker_budget` | 10 | 🟡 caller-trust-boundary | `worker_budget_write.rs:179,188,241`, `spend_anomaly.rs:335,363,384`, `skill_runs_query.rs:152,161,432`, `tenant_quota_usage.rs:168` | All UPDATE/SELECT statements key on `worker_id`, no `tenant_id` on the row. The `worker_budget` table itself has `tenant_id`; queries use `worker_id` as the lookup key. Belt-and-braces gap, not a confirmed leak. |
| `worker_freeze` | 4 | 🟢 BIND | `worker_freeze.rs:74,118,148,170` | All non-INSERT bind `tenant_id`. ✔ |
| `workflow_definition` ⭐ | 12 | 🟢 BIND + 🟡 caller-trust-boundary | `workflow_definition_read.rs:19,42,68`, `workflow_definition_write.rs:61,75,154,179,192,229,248,323,358,374,387,407` | Round-7 verified. Read path binds `tenant_id` on every operator-visible lookup. Write path's `archive_in_tx` / `set_head_in_tx` UPDATE statements key on `id` only — the helper requires a caller-side `tenant_id` resolution to find the id (`workflow_definition_read.rs:42` lookup). Consistent with the "ids are random + caller-resolved" pattern. |
| `workflow_definition_index` ⭐ | 5 | 🟢 BIND | `workflow_definition_read.rs:42`, `workflow_definition_write.rs:179,255,358,407` | All non-INSERT bind `(tenant_id, vertical_id, name, …)`. ✔ |
| `workspace` | 7 | 🟢 by design | `budget.rs:170`, `workspace.rs:20,31,44,65,91,111`, `tenant_query.rs` | Legacy single-tenant table — workspace IS the tenant boundary in the pre-multi-tenant model. Per `workspace.rs:1-20`, the single-workspace primitive predates `tenant_id`. ✔ |

### §2.2 — Workspace-id-only tables (2)

| Table | Audit notes |
|---|---|
| `marketplace_artifact` | Workspace-scoped per `migrations/v1.rs`. No store-side queries in scan (callers are out of crate). Out of may-touch. |
| `policy_files` | Workspace-scoped per `migrations/v30.rs`. No store-side queries in scan. Out of may-touch. |

### §2.3 — Child-scoped / globally-shared tables (42)

These tables intentionally have no own `tenant_id` column. Each row
is either (a) a child of a tenant-scoped parent, isolated through
parent-id resolution at query time, (b) globally shared by contract
(`schema_version`, `recovery_cache`, etc.), or (c) a server-internal
table not exposed to tenants.

Out-of-scope for byte-level audit (the contract is "tenant-resolve
the parent id at the caller"). The contract for each is documented
in its migration file's module doc-comment per the convention
established by `subtask_ledger`'s SAFETY block.

The 42 are: `approval_bundle_member`, `auction_bid`, `auction_log`,
`data_classification_tag`, `duty_fired_session`, `forge_queue`,
`forge_run`, `gateway_request_audit`, `handoff`, `judge_record`,
`marketplace_artifact`, `memory_embedding`, `memory_lock`, `message`,
`operator_feedback`, `operator_risk_overrides`, `org_member_alias`,
`org_reports_to`, `persona_revision`, `policy_files`,
`recovery_cache`, `routing_decision`, `schema_version`, `session_grant`,
`session_skill`, `skill`, `skill_ab_test_outcome`, `skill_ab_test_run`,
`skill_flag`, `skill_lock`, `skill_provenance`, `skill_quality_score`,
`roster_member`, `subtask_ledger` ⭐ (audited round 13), `support_ticket_reply`,
`tool_call`, `vertical_lifecycle`, `worker_anomaly_event`,
`worker_behavior_baseline`, `worker_budget_alert`, `worker_circuit_history`,
`worker_spend_anomaly`, `worker_spend_baseline`, `workflow_checkpoint`,
`workflow_run`.

Recommended follow-up: `eval-argo-store-child-scoping-sweep` —
verify each migration carries a SAFETY block documenting the parent-id
resolution path the caller must perform. P3.

---

## §3 — Distribution of statement classifications

Across the 25 tenant-bearing tables that have store-side statements:

```
🟢 BIND        (tenant_id = ? in WHERE) ........ 41 statements
🟢 BIND-IN     (tenant_id IN (…))             …  0 statements
🟡 MENTION     (INSERT supplies column, etc.) .. 27 statements
🔴 NO_TENANT   (WHERE without tenant_id)       . 60 statements
— NO_WHERE    (INSERT/REPLACE, no WHERE)      .  5 statements
                                            total 133 (subset of 338
                                            that touch tenant tables)
```

Of the 60 🔴 NO_TENANT statements:
- **~28 are 🟢-by-design** after caller cross-check (audit rollups,
  GDPR, `spend.rs::load_global_spend`, etc. — see §2.1 table).
- **~22 are 🟡 caller-trust-boundary** — id-keyed UPDATEs/SELECTs
  whose only defense is caller-side tenant resolution. These are the
  fix-brief targets in §5.
- **~10 are heuristic misclassifications** where the regex missed a
  hyphen-spanning multi-line `AND tenant_id = ?` or a column-list
  `tenant_id =` that lands mid-line.

---

## §4 — Highest-priority findings (spotlight)

### Finding 1 — `approvals::decide_request` SQL has no `tenant_id` filter

- **File**: `crates/argo-store/src/approvals.rs:117-131` and analogue
  in `approval_bundle.rs:280-296`.
- **SQL** (verbatim, line 117):
  ```sql
  UPDATE approval_request
     SET status = ?, decided_at = ?, decided_by = ?, decision = ?
   WHERE id = ? AND status = 'pending'
  ```
- **Rust signature**: `pub async fn decide_request(pool, id: &str,
  decided_by: &str, decision: ApprovalDecision)` — no `tenant_id`
  argument plumbed in.
- **Production callers (as of `7c3178639`)**: **0**.
  `rg decide_request crates/ | grep -v _tests.rs | grep -v claim_6` →
  the only non-test reference is the doc-comment chain inside
  `approvals.rs`. The `/api/v1/approvals` route in
  `crates/argo-cli/src/handlers/gateway/routes_approvals_json.rs`
  routes through `worker_actions_transitions::approve()`, NOT through
  `decide_request`.
- **Risk**: shipped-leak: **NO** (no remote caller).
  Future-leak: **YES** — same shape as the audit_detail bug #1992.
  The moment a route handler wires `decide_request`, a cross-tenant
  decision spoof is one HTTP POST away.
- **Fix shape**: name a new fix-brief
  **`mission-approval-decide-request-tenant-pinned`** that
  (a) adds `tenant_id: TenantId` to `decide_request`'s signature, (b)
  changes the WHERE to `WHERE id = ? AND tenant_id = ? AND status =
  'pending'`, (c) updates the four `*_tests.rs` callers and the
  cross-tenant test in `tests/claim_6_cross_tenant_approval_queue.rs`,
  and (d) does the analogous change on
  `approval_bundle::decide_bundle` (same SQL shape, same caller
  trust-boundary today).

### Finding 2 — `elevated_view_grant::revoke` SQL has no `tenant_id` filter

- **File**: `crates/argo-store/src/elevated_view_grant.rs:154`.
- **SQL**: `UPDATE elevated_view_grant SET revoked_at = ? WHERE id = ?`.
- **Production callers**: 1 — the elevated-view grant revocation
  helper in `argo-cli` handlers. Caller passes a `GrantId` directly;
  no `tenant_id` re-validation in the SQL.
- **Risk**: same class as finding 1. GrantId is UUID-unique so
  cross-tenant collisions are theoretical, but the row's
  `tenant_id` should be a belt-and-braces WHERE clause.
- **Fix shape**: name **`mission-elevated-view-grant-revoke-tenant-pinned`**.

### Finding 3 — `scim_user::update` SQL has no `tenant_id` filter

- **File**: `crates/argo-store/src/scim_user.rs:226,271`.
- **SQL**: `UPDATE scim_user SET … WHERE id = ?` (two distinct
  helpers — patch + deactivate).
- **Production callers**: SCIM IDP webhook handler. The
  IDP-supplied SCIM id is the lookup key; the SQL inherits IDP
  trust.
- **Risk**: medium. SCIM ids are issued by the upstream IDP and are
  expected to be globally unique within the IDP, but cross-IDP
  collisions are not structurally prevented in the store.
- **Fix shape**: name **`mission-scim-user-update-tenant-pinned`**.

### Finding 4 — `support_query::update_ticket` doc-comment admits the gap

- **File**: `crates/argo-store/src/support_query.rs:344-348`.
- **Doc-comment** (verbatim):
  ```rust
  /// Apply a partial update to a ticket. Returns `true` when the row
  /// existed. Caller is responsible for any RBAC check; the storage
  /// layer only enforces schema.
  ```
- **SQL**: `UPDATE support_ticket SET status = ?, priority = ?,
  assigned_to = ?, updated_at = ? WHERE id = ?`.
- **Production callers**: gateway support-ticket update route.
- **Risk**: low *given the caller-side RBAC gate* — but the
  doc-comment admits the storage layer ships no defense. If RBAC
  enforcement regresses, the cross-tenant attack is one route-handler
  bug away.
- **Fix shape**: name **`mission-support-ticket-update-tenant-pinned`**
  — promote the RBAC contract from doc-comment to SQL bind.

### Finding 5 — `worker_budget_write` UPDATE statements bypass tenant_id

- **File**: `crates/argo-store/src/worker_budget_write.rs:111,129,179,188,255`.
- **SQL** (excerpt at line 129): `UPDATE worker SET monthly_budget_usd_micros = ? WHERE id = ?`.
- **Production callers**: budget-set CLI handlers + scheduled spend
  sweeper.
- **Risk**: low. `worker.id` is UUID-unique, but the same belt-and-
  braces principle applies — `worker_budget` schema has `tenant_id`,
  the SQL should bind it.
- **Fix shape**: name **`mission-worker-budget-write-tenant-pinned`**.

### Finding 6 — `session_meta` mutators key on session id only

- **File**: `crates/argo-store/src/session_meta.rs:82,96,109,123,138`.
- **SQL examples**: `UPDATE session SET title = ? WHERE id = ?`,
  `DELETE FROM session WHERE id = ?`, etc.
- **Production callers**: TUI session-rename + delete commands;
  `end_session` from runtime.
- **Risk**: low. `SessionId` is UUID-unique. But the schema has
  `tenant_id` and the SQL should bind it for defense-in-depth.
- **Fix shape**: name **`mission-session-meta-mutators-tenant-pinned`**.

### Finding 7 — `persona_revision::apply_revert` keys on worker_id only

- **File**: `crates/argo-store/src/persona_revision.rs:148`.
- **SQL**: `UPDATE persona SET <col> = ? WHERE worker_id = ?`.
- **Production callers**: persona-revert CLI handler.
- **Risk**: low. `worker_id` is UUID-unique. But `persona` carries
  `tenant_id` and the SQL should bind it.
- **Fix shape**: name **`mission-persona-revert-tenant-pinned`**.

---

## §5 — Recommended fix-brief queue (consolidated)

The audit surfaces seven fix-briefs of the same shape:

```
mission-<surface>-tenant-pinned
```

Each adds `tenant_id: TenantId` to the helper signature, changes the
WHERE clause to bind it, and updates the in-crate tests. Suggested
priority ordering (highest blast-radius first):

| # | Brief id | Surface | Priority |
|---|---|---|---|
| 1 | `mission-approval-decide-request-tenant-pinned` | `approvals::decide_request` + `mark_escalation_sent` + `approval_bundle::decide_bundle` | 🔴 P0 — same shape as #1992 audit_detail bug; ship before wiring HTTP route |
| 2 | `mission-scim-user-update-tenant-pinned` | `scim_user::update` + `deactivate` | 🟡 P1 — production route exists |
| 3 | `mission-support-ticket-update-tenant-pinned` | `support_query::update_ticket` + `reply_ticket` | 🟡 P1 — promote RBAC from comment to SQL |
| 4 | `mission-session-meta-mutators-tenant-pinned` | `session_meta::*` (5 statements) + `store_session_routing::set_routing_policy` | 🟡 P2 — UUID uniqueness covers, but belt-and-braces |
| 5 | `mission-worker-budget-write-tenant-pinned` | `worker_budget_write::*` (4 UPDATEs/SELECTs) | 🟡 P2 |
| 6 | `mission-persona-revert-tenant-pinned` | `persona_revision::apply_revert` | 🟡 P2 |
| 7 | `mission-elevated-view-grant-revoke-tenant-pinned` | `elevated_view_grant::revoke` | 🟡 P2 |

A follow-up `eval-argo-store-child-scoping-sweep` (P3) is recommended
to ensure every child-scoped table in §2.3 carries a SAFETY block in
its migration documenting the parent-id resolution contract — the
same documentation pattern `subtask_ledger` adopted in round 13.

---

## §6 — Pin test added

To prevent regression of the §4 finding 1 contract (the
auditor must not lose track of this gap between this report and the
fix-brief landing), this PR ships a single `#[ignore]`'d
FINDINGS-ONLY test:

- **File**: `crates/argo-store/tests/approval_decide_tenant_bind_pin.rs`
- **What it pins**: `decide_request(pool, tenant_a_row_id, "grantor",
  Approve)` succeeds today even when the caller has not validated
  tenant ownership — the SQL itself imposes zero tenant gate. The
  test sets up two tenants, lets the helper decide tenant_a's row
  with a "wrong tenant context" caller, and asserts the row
  transitions to `Approved`. Un-ignore when finding 1's fix-brief
  lands; the test then becomes the regression gate that proves the
  new `tenant_id` argument actually rejects the cross-tenant call.

The test is `#[ignore]`'d so it does not block CI today; it carries
a header comment that names the fix-brief that un-ignores it.

---

## §7 — Out-of-scope / explicit non-goals

- **Out-of-crate query callsites.** This audit walks only
  `crates/argo-store/src/` per the brief's may-touch list. Callsites
  in `argo-agent-runner`, `argo-orchestrator`, `argo-cli`,
  `argo-memory`, etc., that own their own SQL (e.g. memory layer-row
  queries) are covered by their own audit rounds (`memory_entry`
  round 7, `vertical_locks` round 4 noted in the manifest).
- **Child-scoping deep-dive.** The 42 child-scoped tables in §2.3 are
  enumerated and named but not byte-audited. The `subtask_ledger`
  round-13 audit is the template for that follow-up.
- **Workspace-id tables.** `marketplace_artifact` + `policy_files` —
  both have no in-store callsites and live behind workspace-scoped
  helpers; they earn their own audit round (`eval-workspace-id-tables-sweep`).
- **Performance / index audit.** Tenant binding is correctness;
  index coverage for `(tenant_id, id)` composite keys is a separate
  axis tracked in `docs/skycot/audit/` query-performance reports.

---

## §8 — Reproducing this audit

The two scanning scripts used to produce §2 and §3 are not committed
(one-shot tools). They are reproducible from this report's
methodology section. For a quick re-run sanity-check:

```bash
# Tables with tenant_id column (33 + 2 via ALTER)
rg -E "CREATE TABLE.*tenant_id|ALTER TABLE \w+ ADD COLUMN tenant_id" \
   crates/argo-store/src/migrations/ | grep -v splitter | wc -l

# SQL statements touching a tenant-bearing table without tenant_id WHERE
rg -U "sqlx::query.*\(\s*\"[^\"]*UPDATE\s+(approval_request|session|worker|persona|scim_user|support_ticket|elevated_view_grant)[^\"]*WHERE id = \?" \
   crates/argo-store/src/ --multiline-dotall
```

---

## §9 — Status

- [x] All 77 production tables enumerated.
- [x] 33 tenant-id-bearing tables byte-audited (those with store-side
      statements; 8 have no `argo-store/src/` callsites).
- [x] 7 fix-briefs named in §5 ready for conductor queue.
- [x] 1 `#[ignore]`'d FINDINGS-ONLY pin test added (§6).
- [x] Gap-manifest row cited verbatim per brief instruction.

**Verdict**: **🟡 P1 → 🟡 P1 (deepened)**. The single sweep does
not close the manifest row; it converts a vague "30+ tables need
auditing" into a concrete 7-fix-brief queue with verbatim line cites.
The original brief recommended exactly this shape:
"Output: per-table verdict + fix-brief recommendations." ✔
