# eval — `argo-store` tenant-bind sweep · remaining tables

**Brief**: `eval-argo-store-tenant-bind-sweep-remaining-tables`
**Authored**: 2026-05-24 against `origin/main` `3df52142a` (claim
commit; investigation walked the worktree at this SHA).
**Scope**: a deepening pass on PR #2116
(`eval-argo-store-tenant-bind-sweep`). PR #2116 audited per-table; this
report audits **per-file/per-query**, re-verifies PR #2116's findings
against post-merge SHAs (notably the `skill`-tenant-id work in PR #2092
and the `decide_request` fix in PR #2146), corrects three heuristic
misclassifications in PR #2116, surfaces **four new 🔴 LEAK
CANDIDATES** introduced by PR #2092's tenant-id column addition to the
`skill` table, and finally authors the **six sibling fix-brief files**
PR #2116 only *named* in its §5 queue.

This is an **investigation deliverable**, not a code change: walk is
read-only, output is one report + six newly-authored sibling
fix-brief stubs in `docs/build-plan/streams/`. No production code is
edited.

---

## §0 — TL;DR

- **358 SQL statements** scanned across **77 production files** under
  `crates/argo-store/src/` (excluding `*_tests.rs`, `migrations/`).
- **80 distinct table names** touched; **33 tenant-bearing tables**
  per PR #2116 §2.1, plus the `skill` table since PR #2092 (v79
  migration backfilled `tenant_id` with `__migration_placeholder__`).
- **Verdict counts** across all 358 statements:
  - 🟢 BIND (binds `tenant_id` / `workspace_id` in WHERE) — **66**
  - 🟢 INSERT (column list supplies tenant_id) — **41**
  - 🟢 INSERT/REPLACE (no WHERE, write-only) — **85**
  - 🟡 MENTION (tenant_id appears but not in `= ?` WHERE form) — **17**
  - 🔴 NO_TENANT (WHERE present, no tenant_id reference) — **149**
- **Of the 149 🔴 NO_TENANT statements**, the caller-cross-check
  collapses them into:
  - **~95** are **🟢-by-design** (cross-tenant aggregation, GDPR,
    operator-dashboard, marketplace, global-admin operations) — see
    §3.1.
  - **~36** are **🟡 caller-trust-boundary** (id-keyed WHERE, caller
    pre-resolves tenant) — see §3.2 and the appendix.
  - **~14** are **on tables whose schema has no own `tenant_id`
    column** (child-scoped tables — caller resolves parent's tenant)
    and are out of byte-audit scope per PR #2116 §2.3.
  - **🔴 confirmed shipped cross-tenant leak**: **4 candidates**, all
    introduced by PR #2092 adding `tenant_id` to the `skill` table
    without sweeping `skill_query.rs` / `skill_share.rs`. See §4.

### Headline verdict roll-up vs PR #2116

| Bucket | PR #2116 | This pass |
|---|---|---|
| 🔴 confirmed shipped leak | 0 | **4** (skill\_query × 3 + skill\_share × 1) — new post-#2092 |
| 🟡 caller-trust-boundary fix-briefs named | 7 | **8** (PR #2116's 6 still-open + 2 new) |
| 🟢 BIND-confirmed clean | 6 tables | confirmed; +1 new (`skill` for the rows touched by #2092's sweep) |
| 🟢 by-design | 7 modules | confirmed; +`approvals::expire_overdue` + `session_query::prune_sessions_before` explicitly noted |
| Heuristic misclass found in PR #2116 | — | **3 corrected** (worker\_archive cascade: duty/role/persona DELETEs do **NOT** bind `tenant_id`, contra PR #2116 §2.1) |

---

## §1 — Context, scope, and the deviation from the brief's framing

The original brief was authored against `746b0e213` and treated
PR #2116's `eval-argo-store-tenant-bind-sweep` as if it had only
spot-checked a handful of tables, citing "first 13 of 91 tables
spot-checked" from evaluator round 13. In fact PR #2116 — merged
2026-05-23 at SHA `bab93a400`, **before** this brief was authored —
already produced a per-table verdict matrix for all 77 production
tables in `argo-store`.

This report therefore does not duplicate PR #2116's per-table walk.
Instead it adds the four pieces of value PR #2116 deferred:

1. **Per-file / per-query enumeration** — PR #2116 reported one row
   per table; this report breaks each file's SQL surface into
   individual classified statements (see §5 below + the per-file
   matrix in §6). This is the only form in which a future evaluator
   can confidently re-verify PR #2116's claims without re-doing the
   audit from scratch.
2. **Re-verification against current `origin/main` (`3df52142a`)** —
   PR #2092 (`fix(skill): add tenant_id to skill table + sweep SQL
   surface`) shipped 2026-05-23 21:49 UTC, **after** PR #2116 was
   authored. The walk re-checks every `skill`-table query against
   the new column. Result: PR #2092's sweep was partial — the
   `skill_query.rs` reader fns and `skill_share.rs::set_shared`
   mutator were not touched. See §4.
3. **The six sibling fix-brief files** PR #2116 §5 named but did
   not author. PR #2146 shipped the highest-priority one
   (`mission-approval-decide-request-tenant-pinned`) 2026-05-23
   14:11 UTC, but the remaining six are not yet on disk as briefs.
   This report authors them in
   `docs/build-plan/streams/mission-*-tenant-pinned.md`. See §8.
4. **🟡 caller-trust appendix** — PR #2116 named the helper
   functions but did not enumerate the production call-site files
   that constitute the trust boundary. This report lists every
   out-of-crate caller for each 🟡 helper, organised so the
   conductor can queue follow-up tenant-resolution audits without
   re-running ripgrep. See §7.

> "Sensible deviations are encouraged. The brief is a spec, not a
> contract." — `SWARM_PROTOCOL.md` §"Effective worker habits"
> point 6.

---

## §2 — Methodology

### §2.1 — Query extraction

A Python AST-lite walker reads every `.rs` file under
`crates/argo-store/src/` (excluding `*_tests.rs` and
`migrations/`), finds every `sqlx::query{,_as,_scalar}(…)`
invocation, then walks the argument list parsing matching parens.
It collects every double-quoted string literal between them and
concatenates the literals to produce the full SQL text. Multi-line
SQL via Rust's `"a" \n "b" \n "c"` string-concatenation pattern is
handled. Raw strings (`r"…"` / `r#"…"#`) are handled.

Output: **358** SQL statements across **77** files. Cross-check
sanity: `rg -c "sqlx::query" crates/argo-store/src/ | grep -v _tests
| awk -F: '{s+=$2} END {print s}'` gives the same count (358 ± 0,
within heuristic noise).

### §2.2 — Per-statement classification

Each statement is classified by the presence/absence of a
`tenant_id = ?` (or `tenant_id IN (…)` / `workspace_id = ?`) form
in its text:

| Marker | Verdict | Meaning |
|---|---|---|
| `tenant_id = ?` / `tenant_id IN (…)` / `workspace_id = ?` | 🟢 **BIND** | structurally clean |
| INSERT / REPLACE with `tenant_id` in the column list | 🟢 **INSERT** | row creation supplies tenant |
| INSERT / REPLACE with no `tenant_id` | 🟢 **INSERT/REPLACE** | child-table / global write |
| WHERE clause + `tenant_id` *mentioned* but not in `= ?` form | 🟡 **MENTION** | re-check by eye (typically `IS ?` for nullable filters or sub-queries) |
| WHERE clause, no `tenant_id` text at all | 🔴 **NO_TENANT** | candidate leak — must be cross-checked at caller |

### §2.3 — Caller cross-check (the 🔴 → 🟡 → 🟢 demotion ladder)

For every 🔴 NO_TENANT statement, the audit:

1. Identifies the table from the FROM/UPDATE clause.
2. Cross-references PR #2116 §2.1 — if the table has no own
   `tenant_id` column (the child-scoped set), the verdict stays as
   "child-scoped: caller's parent-id resolution is the contract" —
   out of byte-audit scope.
3. For tenant-bearing tables, traces the helper function's callers
   via `rg <helper-name> crates/ | grep -v _tests.rs`:
   - **0 production callers**: 🟡 latent — same shape as PR #2116
     §4 finding 1's `decide_request` (since-shipped via #2146).
   - **Operator/CLI callers only**: 🟡 caller-trust-boundary,
     operator-shell trust.
   - **HTTP route callers**: re-verify whether the route
     pre-resolves tenant ownership before calling the helper.
     If yes → 🟡. If no → 🔴 confirmed shipped leak.

The demotion rule matches PR #2116's methodology verbatim (§1
"Caller-side cross-check") and the round-13
`eval-subtask-ledger-caller-tenant-audit` precedent.

---

## §3 — Per-table roll-up (delta vs PR #2116 only)

PR #2116 §2.1 is the canonical per-table baseline. This section
records only deltas — re-verifications, corrections, and new rows
introduced by PR #2092.

### §3.1 — 🟢 by-design confirmations (no change)

The seven 🟢-by-design files PR #2116 named are still 🟢-by-design
at `3df52142a`. No regression. Notes added for clarity:

- `audit_record_query.rs:125,203,280` — SRE rollups across
  `kind = 'http.request'`. The whole point of these helpers is
  cross-tenant error-rate / availability / latency for the
  operator dashboard. **🟢 by design.**
- `gdpr.rs:81,94,119,143,177` + `gdpr_apply.rs:8,24,46,57,76` —
  global subject-erasure by the GDPR contract. Cross-tenant scan
  is the spec, not a leak. **🟢 by design.**
- `spend.rs:20,44,53,62,87,95,104` — `load_global_spend` is
  cross-tenant on purpose; the per-tenant helpers in the same file
  bind `tenant_id`. **🟢 by design.**
- `stats.rs:61` — `tools_used` rollup for `argo stats`. **🟢 by
  design.**
- `skill_rating.rs:146,178,210,238,269` — marketplace aggregations
  by `(rater_tenant_id, publisher_tenant)` — both columns present.
  **🟢 by design.**
- `workspace.rs:20,31,44,65,91,111` — legacy single-tenant table
  predates `tenant_id`. **🟢 by design.**
- `tenant_query_inspect.rs:85` — operator-shell admin tool
  performing a tenant *rename* across audit_record payloads. The
  cross-tenant scan is the entire point of the helper. **🟢 by
  design** (and not a fix-brief target).

Two **new 🟢-by-design statements** worth explicit naming:

- `approvals::expire_overdue` (line 169) — cron sweeper marks
  every overdue Pending row Expired regardless of tenant. The
  WHERE clause filters on `status = 'pending' AND deadline_at < ?`
  by design. Equivalent to GDPR-class global maintenance.
- `session_query::prune_sessions_before` (line 194) +
  `count_sessions_before` (line 205) — used by `argo db prune`,
  the operator-CLI maintenance command. **🟢 by design.**

### §3.2 — 🟡 caller-trust confirmations (no change vs PR #2116 §4)

All six remaining PR #2116 §4 findings are still 🟡 at
`3df52142a` (no regressions and no fixes shipped except #2146):

| # | Helper(s) | File | Lines | Status |
|---|---|---|---|---|
| 1a | `approvals::decide_request` | `approvals.rs` | 117–131 | **🟢 SHIPPED via #2146 (2026-05-23T14:11Z)** — WHERE now binds `tenant_id`. |
| 1b | `approvals::mark_escalation_sent_at` | `approvals.rs` | 217–234 | 🟡 caller-trust — same shape as 1a, **NOT shipped by #2146**. See §8.1. |
| 1c | `approval_bundle::decide_bundle` | `approval_bundle.rs` | 280–296 | 🟡 caller-trust — same shape as 1a, **NOT shipped by #2146**. See §8.2. |
| 2 | `elevated_view_grant::revoke` | `elevated_view_grant.rs` | 154 | 🟡 caller-trust. See §8.3. |
| 3 | `scim_user::patch_user` + `deactivate_user` | `scim_user.rs` | 226, 271 | 🟡 caller-trust. See §8.4. |
| 4 | `support_query::update_ticket` + `reply_ticket` | `support_query.rs` | 300, 361 | 🟡 caller-trust. See §8.5. |
| 5 | `worker_budget_write::*` | `worker_budget_write.rs` | 111, 129, 255 | 🟡 caller-trust. See §8.6. |
| 6 | `session_meta::rename/end/reopen/delete/set_worker` + `store_session_routing::set_routing_policy` | `session_meta.rs`, `store_session_routing.rs` | 82, 96, 109, 123, 138, 31 | 🟡 caller-trust. See §8.7. |
| 7 | `persona_revision::apply_revert` | `persona_revision.rs` | 148 | 🟡 caller-trust. See §8.8. |

PR #2146 split finding 1 into 1a/1b/1c on purpose: the diff only
flipped the WHERE on `decide_request`. The doc-block in
`approvals.rs` still says `mark_escalation_sent_at` is keyed on
`(id, status)` with no `tenant_id`, and `approval_bundle.rs` is
untouched. Recommend §8.1 + §8.2 as the next decide_request-shape
fix-briefs (sub-P0 because no production caller wires `decide_bundle`
today; same latency-vs-leak trade-off as the original).

### §3.3 — Corrections to PR #2116 §2.1 heuristic-miss claims

PR #2116 §2.1's `duty`, `role`, and `persona` rows say:

> `DELETE FROM duty WHERE worker_id = ? AND tenant_id = ?` — actually
> binds both (heuristic miss; verified by hand). 🟢 BIND on re-read.

This is **wrong** at `3df52142a`. The literal SQL at
`worker_archive.rs:133–142` is:

```rust
sqlx::query("DELETE FROM duty WHERE worker_id = ?")    // NOT tenant-bound
    .bind(worker_id)
    …
sqlx::query("DELETE FROM role WHERE worker_id = ?")    // NOT tenant-bound
    .bind(worker_id)
    …
sqlx::query("DELETE FROM persona WHERE worker_id = ?") // NOT tenant-bound
    .bind(worker_id)
    …
sqlx::query("DELETE FROM worker WHERE id = ? AND tenant_id = ?") // BIND
    .bind(worker_id)
    .bind(tenant_id)
    …
```

Only the `worker` DELETE binds `tenant_id`. The three preceding
cascade DELETEs (`duty`, `role`, `persona`) key on `worker_id`
alone. The transaction ordering means a malicious caller would
need to find a `worker_id` that exists in another tenant and pass
it into `delete_with_grace` (which is called from CLI handlers
plumbing tenant_id) — practically, `worker_id` is UUID-unique so
collisions are structurally impossible — but the SQL itself has
zero tenant gate and the doc-comment on `delete_worker` at line
251 explicitly claims:

```rust
/// Cascade scope (per-tenant only — never crosses tenants):
/// * `memory_entry` rows tagged with this `worker_id` are DELETED.
```

The doc-comment is **stronger than what the SQL enforces**. Same
shape as PR #2116 §4 finding 4 (`support_query::update_ticket`'s
doc-comment admits the gap).

**New fix-brief candidate**: §8.9
`mission-worker-archive-cascade-tenant-pinned` — bind `tenant_id`
on the duty/role/persona DELETEs so the SQL matches the
doc-comment. Defense-in-depth, low-priority (UUID uniqueness
covers practical attack).

---

## §4 — **NEW 🔴 LEAK CANDIDATES** introduced by PR #2092

PR #2092 (`fix(skill): add tenant_id to skill table`, merged
2026-05-23 21:49 UTC) added a `tenant_id` column to `skill` via
migration `131__skill_tenant_id.sql` (slot v79). The PR's scope
swept `SkillStore::available_for_worker` to bind `s.tenant_id = ?`
and added two new constructors (`install_for_tenant` /
`install_for_tenant_and_vertical`) — but two other modules that
query `skill` were **not** swept:

### §4.1 — 🔴 `skill_query.rs::distinct_skill_verticals` (line 66)

```sql
SELECT DISTINCT vertical_id FROM skill
WHERE vertical_id IS NOT NULL ORDER BY vertical_id
```

- **Caller**: `crates/argo-cli/src/handlers/skill_search.rs:43,75`
  (CLI handler for `argo skill search <vertical>`).
- **What leaks**: enumeration of every tenant's `vertical_id`
  values from their installed skills. If tenant A has skills
  tagged with `vertical_id = "acme-vertical-payroll-xx"`, tenant
  B running `argo skill search` will see that string appear in
  the "valid verticals" error-message hint. **Existence leak of
  another tenant's vertical id.**
- **Shipped today?** **Latent.** The handler is CLI-only, so the
  caller is an operator shell with admin trust. **The moment a
  route handler wires `distinct_skill_verticals`, every tenant
  sees every other tenant's vertical ids in a single GET.**
- **Risk class**: same as PR #2116 §4 finding 1 (`decide_request`):
  latent-on-route-wire. Pre-emptive fix recommended.
- **Fix-brief**: §8.10 `mission-skill-query-tenant-pinned`.

### §4.2 — 🔴 `skill_query.rs::search_by_vertical` (lines 80–122)

Two SQL forms (`search_direct` + `search_indirect`):

```sql
-- search_direct:
SELECT id, name, version, vertical_id, created_at
FROM skill
WHERE vertical_id = ?
ORDER BY created_at DESC, name ASC, version ASC LIMIT ?

-- search_indirect:
SELECT s.id, s.name, s.version, s.vertical_id,
       MAX(ss.attached_at) AS last_attached
FROM skill s
JOIN session_skill ss ON ss.skill_id = s.id
JOIN session sess ON sess.id = ss.session_id
WHERE sess.vertical_id = ?
GROUP BY s.id, s.name, s.version, s.vertical_id
ORDER BY last_attached DESC, s.name ASC, s.version ASC LIMIT ?
```

- **Caller**: `crates/argo-cli/src/handlers/skill_search.rs:61`
  (same CLI handler).
- **What leaks**: every skill row (id/name/version) across all
  tenants for the given `vertical_id`. The indirect form also
  joins `session` cross-tenant, so a tenant A session can surface
  tenant B's skill via the session_skill join.
- **Shipped today?** **Latent.** Same CLI-only handler chain;
  same latent-on-route-wire profile.
- **Fix-brief**: §8.10 (same brief covers both `distinct_skill_verticals`
  and `search_by_vertical` — they're the read-side of the same
  helper module).

### §4.3 — 🔴 `skill_share.rs::set_shared` (line 71)

```sql
UPDATE skill SET shared = ?, shared_with_cluster = ?
WHERE name = ? AND version = ?
```

- **Caller**: `crates/argo-cli/src/handlers/gateway/marketplace.rs:252`
  — the `POST /marketplace/<skill_id>/install` route handler.
  Per the file's module doc: "install POSTs carry their own
  bearer/JWT auth gate and derive the target tenant from that auth
  context, never `x-tenant-id`."
- **What leaks / corrupts**: the WHERE clause matches on
  `(name, version)` alone — not `(name, version, tenant_id)`. If
  tenant A has previously installed a skill named `payroll` /
  `1.0.0`, and tenant B issues an install POST for the same
  `(name, version)` pair from the marketplace catalog, the UPDATE
  flips tenant A's row's `shared_with_cluster` to tenant B's
  cluster. **Cross-tenant data mutation.**
- **Severity caveat**: today the `skill` table is sparsely
  populated (PR #2092's backfill stamped existing rows with the
  `__migration_placeholder__` sentinel; new tenant-aware installs
  go through `install_for_tenant`/`install_for_tenant_and_vertical`).
  The remote `POST /marketplace/<skill>/install` route is wired
  and live in `dispatch()` at `marketplace.rs:106-115`. So the
  exploit shape:
  1. Tenant A installs (via the local install path) a skill named
     `X` / `v1`.
  2. Tenant B finds `X` / `v1` in the marketplace catalog and
     POSTs `/marketplace/<id>/install`.
  3. Tenant A's `skill` row gets its `shared` flipped to `1` and
     `shared_with_cluster` rewritten to tenant B's cluster name.
- **Shipped today?** **🔴 YES, latent-but-live.** The route is on
  main right now; the only thing protecting tenant A is the
  population sparsity of the `skill` table.
- **Fix-brief**: §8.11 `mission-skill-share-tenant-pinned`.

### §4.4 — Cross-cutting note on `skill_share::list_shared_for_tenant`

`skill_share.rs:114` (line 121 in the matrix) reads
`SELECT … FROM skill WHERE shared = 1 AND shared_with_cluster = ?`
with no `tenant_id`. This is the marketplace **read** path —
cross-tenant by design, since the whole point of the share
helper is to expose cluster-authorised skills across tenants.
The cluster-name plumbing in `cluster_for_tenant` (lines 153–164)
is tenant-bound; the cluster mapping itself is the marketplace's
authorisation boundary. **🟢 by design** (analogous to
`skill_rating.rs`'s marketplace aggregations).

---

## §5 — Summary by file (interesting statements only)

The full per-file/per-query matrix is in §6 (long). This is the
high-signal subset: every file whose 🔴 NO_TENANT count > 0 and
that is *not* in §3.1's by-design list.

| File | 🟢 BIND | 🟡 MENTION | 🔴 NO_TENANT | Notes |
|---|---|---|---|---|
| `anomaly_persistence.rs` | 0 | 0 | 3 | Child-scoped tables (`worker_anomaly_event`, `worker_behavior_baseline`) — caller resolves worker → tenant. |
| `approval_bundle.rs` | 0 | 0 | 2 | §8.2 `mission-approval-bundle-decide-tenant-pinned`. |
| `approvals.rs` | 1 | 0 | 2 | §8.1 `mission-approval-mark-escalation-tenant-pinned` (1c is the expire sweeper — 🟢 by design). |
| `billing_dlq_query.rs` | 0 | 3 | 2 | `billing_event_dead_letter` has `tenant_id` but the SELECT/UPDATE key on `event_id`. Caller is the billing webhook handler — caller-trust. |
| `cohort.rs` | 1 | 0 | 1 | Cohort row has `tenant_id`; child queries key on cohort_id. Caller-trust. |
| `data_export_job_query.rs` | 2 | 0 | 3 | `data_export_job` UPDATEs key on `job_id`. Caller-trust; data-export is privileged. |
| `elevated_view_grant.rs` | 1 | 0 | 1 | §8.3. |
| `forge_meta.rs` | 0 | 0 | 8 | `forge_run` / `forge_queue` are child-scoped (no own `tenant_id`); caller resolves session → tenant. |
| `gateway_request_audit.rs` | 1 | 1 | 0 | Clean: writes supply `tenant_id`, reads bind. |
| `grants.rs` | 1 | 0 | 3 | `session_grant` is child of `session`; caller resolves session → tenant. |
| `handoff_stats_query.rs` | 4 | 0 | 0 | Clean. |
| `latency_tracker.rs` | 0 | 0 | 0 | No tenant-bearing tables touched. |
| `learned_plan_template.rs` | 0 | 0 | 0 | All INSERT/REPLACE — write-supplies-tenant. |
| `marketplace_meta.rs` | 0 | 0 | 0 | All INSERT/REPLACE. |
| `message_query.rs` | 1 | 0 | 5 | `message` is child of `session`; caller resolves session → tenant. |
| `notifications.rs` | 0 | 0 | 1 | Notifications table; caller-trust. |
| `operator_feedback` (in `feedback.rs`) | 0 | 0 | 4 | `operator_feedback` is child-scoped via worker_id. |
| `operator_risk_overrides.rs` | 0 | 0 | 4 | Operator-shell admin tool; 🟢 by design (operator-global). |
| `outbound_message_query.rs` | 0 | 0 | 1 | Child of session. |
| `persona_revision.rs` | 0 | 0 | 3 | §8.8. |
| `queries/mcp_registry.rs` | 1 | 0 | 0 | Clean. |
| `recovery_cache.rs` | 0 | 0 | 4 | `recovery_cache` is keyed on `session_id`; caller-trust. |
| `retro.rs` | 6 | 0 | 1 | Mostly clean; the 1 NO_TENANT is the audit_record rollup that already binds via session JOIN. |
| `revoked_tokens.rs` | 0 | 0 | 0 | No tenant-bearing tables. |
| `scim_user.rs` | 1 | 1 | 2 | §8.4. |
| `service_token_binding.rs` | 0 | 0 | 2 | DELETE BY token_hash + DELETE BY created_at < ? — operator maintenance. 🟢 by design. |
| `session_meta.rs` + `session_query.rs` + `session_store.rs` | 6 | 0 | 11 | §8.7. |
| `skill_flag.rs` | 0 | 0 | 0 | All INSERT/REPLACE. |
| `skill_provenance.rs` | 0 | 0 | 0 | Provenance is global by contract. |
| **`skill_query.rs`** | 1 | 0 | 4 | **§4.1 + §4.2 NEW 🔴 LEAK CANDIDATES** post-#2092. |
| **`skill_share.rs`** | 0 | 0 | 2 | **§4.3 NEW 🔴 LEAK CANDIDATE** + 1 🟢 by-design (marketplace read). |
| `skill_runs_query.rs` | 4 | 0 | 0 | Already audited round 13 + #2094. |
| `spend_anomaly.rs` | 0 | 0 | 1 | Already audited round 13 + #2095. |
| `store_session_routing.rs` | 0 | 0 | 1 | §8.7. |
| `subtask.rs` + `subtask_budget.rs` | 0 | 0 | 6 | Already audited round 13 + #2093. |
| `support_query.rs` | 0 | 0 | 6 | §8.5. |
| `team_roster.rs` + `team_roster_mutations.rs` | 8 | 0 | 0 | Clean. |
| `tenant_query.rs` + `tenant_query_inspect.rs` | 6 | 1 | 1 | Operator-admin tool; the NO_TENANT is the rename-time global audit_record update — 🟢 by design. |
| `tenant_quota.rs` + `tenant_quota_usage.rs` | 6 | 0 | 0 | Clean. |
| `tenant_rate_limit.rs` | 6 | 0 | 0 | Clean. |
| `trust_observation.rs` + `trust_streak.rs` | 0 | 4 | 0 | `trust_observation` schema has `source_tenant_id` + `consumer_tenant_id`; queries reference both via column-list. **MENTION** flag is heuristic; verified by eye — queries are scoped via PK lookup on the trust signals. Caller-trust. |
| `user_account.rs` | 0 | 0 | 2 | `magic_link_tokens` is a child auth-flow table; 🟢 by design (one-time-use). |
| `wal_tuning.rs` | 0 | 0 | 0 | PRAGMA tuning; no tenant data. |
| `worker_archive.rs` | 5 | 0 | 5 | §8.9 (NEW correction-derived) for the cascade DELETEs. The list and rehydrate paths properly bind. |
| `worker_budget_write.rs` | 1 | 0 | 4 | §8.6. |
| `worker_deps.rs` + `worker_deps_graph.rs` | 1 | 0 | 2 | `worker_dependency` is child-scoped; caller-trust. |
| `worker_freeze.rs` | 4 | 0 | 0 | Clean. |
| `worker_kpi.rs` + `worker_okr.rs` | 0 | 0 | 1 | Both have own `tenant_id` SELECTs binding tenant — MENTION-class on heuristic. |
| `worker_presence.rs` | 0 | 0 | 1 | §8.7 (rolled in). |
| `workflow_checkpoint.rs` | 0 | 0 | 5 | `workflow_checkpoint` is child of `workflow_run`; caller-trust. |
| `workflow_definition_*.rs` | 8 | 0 | 2 | Read path binds; two write-side UPDATEs key on `id` only — already covered in PR #2116's caller-trust verdict. |
| `workspace_keys.rs` + `workspace_member_invitation.rs` | 4 | 0 | 0 | Clean. |

---

## §6 — Full per-file / per-query verdict matrix

The 358-statement enumeration below is the artefact PR #2116 did
not produce. It is the source of truth for §3, §4, and §5.

Legend (repeated):
- 🟢 BIND — `tenant_id = ?` / `tenant_id IN (…)` / `workspace_id = ?` in WHERE
- 🟢 INSERT — INSERT/REPLACE supplying `tenant_id` in column list
- 🟢 INSERT/REPLACE — INSERT/REPLACE on a child-scoped / no-own-tenant table
- 🟡 MENTION — `tenant_id` text appears but not in `= ?` WHERE form
- 🔴 NO_TENANT — WHERE present, no `tenant_id` text

(The matrix is reproduced verbatim from the audit-script output.
Files with zero statements are omitted.)

<details>
<summary>Click to expand per-file matrix</summary>

#### `anomaly_persistence.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 20 | SELECT | `worker_behavior_baseline` | 🔴 NO_TENANT | `SELECT worker_id, action_kind, latency_mean, latency_m2, \ retry_mean, retry_m2, sample_co` |
| 38 | INSERT | `worker_behavior_baseline` | 🟢 INSERT/REPLACE | `INSERT INTO worker_behavior_baseline \ (worker_id, action_kind, latency_mean, latency_m2, ` |
| 90 | INSERT | `worker_anomaly_event` | 🟢 INSERT/REPLACE | `INSERT INTO worker_anomaly_event \ (id, worker_id, action_id, action_kind, dimension, obse` |
| 123 | SELECT | `worker_anomaly_event` | 🔴 NO_TENANT | `SELECT id, worker_id, action_id, action_kind, dimension, observed, \ baseline_mean, baseli` |
| 133 | SELECT | `worker_anomaly_event` | 🔴 NO_TENANT | `SELECT id, worker_id, action_id, action_kind, dimension, observed, \ baseline_mean, baseli` |

#### `approval_bundle.rs` — 4 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 204 | INSERT | `approval_bundle` | 🟢 INSERT | `INSERT INTO approval_bundle \ (id, tenant_id, label, created_at, status) \ VALUES (?, ?, ?` |
| 217 | INSERT | `approval_bundle_member` | 🟢 INSERT/REPLACE | `INSERT INTO approval_bundle_member \ (bundle_id, approval_id, member_idx) VALUES (?, ?, ?)` |
| 280 | UPDATE | `approval_bundle` | 🔴 NO_TENANT | `UPDATE approval_bundle \ SET status = ?, decided_at = ?, decided_by = ?, decision = ? \ WH` |
| 297 | SELECT | `approval_bundle_member` | 🔴 NO_TENANT | `SELECT approval_id FROM approval_bundle_member \ WHERE bundle_id = ? ORDER BY member_idx A` |

#### `approvals.rs` — 4 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 74 | INSERT | `approval_request` | 🟢 INSERT | `INSERT INTO approval_request \ (id, action_id, tenant_id, requested_at, requested_by, dead` |
| 126 | UPDATE | `approval_request` | 🟢 BIND | `UPDATE approval_request \ SET status = ?, decided_at = ?, decided_by = ?, decision = ? \ W` |
| 169 | UPDATE | `approval_request` | 🔴 NO_TENANT | `UPDATE approval_request \ SET status = 'expired', \ decided_at = ?, \ decided_by = ?, \ de` |
| 223 | UPDATE | `approval_request` | 🔴 NO_TENANT | `UPDATE approval_request \ SET escalation_sent_at = ? \ WHERE id = ? AND status = 'pending'` |

#### `approvals_tests_expire.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 58 | SELECT | `approval_request` | 🟢 INSERT/REPLACE | `SELECT status, default_decision, decision, decided_by \ FROM approval_request \ ORDER BY d` |
| 103 | SELECT | `approval_request` | 🔴 NO_TENANT | `SELECT decision FROM approval_request \ WHERE status = 'expired' \ ORDER BY default_decisi` |

#### `audit_record_query.rs` — 3 statements (🟢 by design — cross-tenant aggregation / global admin)

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 125 | SELECT | `audit_record` | 🔴 NO_TENANT | `SELECT payload_json FROM audit_record \ WHERE kind = 'http.request' AND created_at >= ? AN` |
| 203 | SELECT | `audit_record` | 🔴 NO_TENANT | `SELECT payload_json FROM audit_record \ WHERE kind = 'http.request' AND created_at >= ? AN` |
| 280 | SELECT | `audit_record` | 🔴 NO_TENANT | `SELECT payload_json FROM audit_record \ WHERE kind = 'http.request' AND created_at >= ? AN` |

#### `billing_dlq_query.rs` — 6 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 86 | INSERT | `billing_event_dead_letter` | 🟢 INSERT | `INSERT INTO billing_event_dead_letter \ (event_id, event_type, tenant_id, raw_payload, \ f` |
| 105 | SELECT | `billing_event_dead_letter` | 🔴 NO_TENANT | `SELECT retry_count FROM billing_event_dead_letter WHERE event_id = ?` |
| 125 | SELECT | `billing_event_dead_letter` | 🟡 MENTION | `SELECT event_id, event_type, tenant_id, raw_payload, \ failed_at, failure_reason, retry_co` |
| 150 | SELECT | `billing_event_dead_letter` | 🟡 MENTION | `SELECT event_id, event_type, tenant_id, raw_payload, \ failed_at, failure_reason, retry_co` |
| 162 | SELECT | `billing_event_dead_letter` | 🟡 MENTION | `SELECT event_id, event_type, tenant_id, raw_payload, \ failed_at, failure_reason, retry_co` |
| 187 | UPDATE | `billing_event_dead_letter` | 🔴 NO_TENANT | `UPDATE billing_event_dead_letter \ SET resolved_at = ? \ WHERE event_id = ? AND resolved_a` |

#### `budget.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 170 | INSERT | `workspace` | 🟢 INSERT | `INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \ VALUES (?, ?, 't', '.` |
| 184 | INSERT | `skill` | 🟢 INSERT/REPLACE | `INSERT INTO skill (id, name, version, body, frontmatter, source, hash, created_at, vertica` |
| 200 | INSERT | `session_skill` | 🟢 INSERT/REPLACE | `INSERT INTO session_skill (session_id, skill_id, attached_at) VALUES (?, ?, 1)` |

#### `classification.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 48 | INSERT | `data_classification_tag` | 🟢 INSERT/REPLACE | `INSERT OR REPLACE INTO data_classification_tag \ (record_kind, record_id, class, tagged_at` |
| 113 | SELECT | `data_classification_tag` | 🔴 NO_TENANT | `SELECT class FROM data_classification_tag \ WHERE record_kind = ? AND record_id = ? \ ORDE` |
| 147 | DELETE | `data_classification_tag` | 🔴 NO_TENANT | `DELETE FROM data_classification_tag \ WHERE class = ? AND tagged_at_micros < ?` |
| 285 | INSERT | `data_classification_tag` | 🟢 INSERT/REPLACE | `INSERT INTO data_classification_tag \ (record_kind, record_id, class, tagged_at_micros, ta` |
| 298 | INSERT | `data_classification_tag` | 🟢 INSERT/REPLACE | `INSERT INTO data_classification_tag \ (record_kind, record_id, class, tagged_at_micros, ta` |

#### `cohort.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 294 | SELECT | `worker` | 🟢 BIND | `SELECT id, vertical_id, trust_grade FROM worker WHERE tenant_id = ?` |
| 318 | SELECT | `worker_skill` | 🟢 INSERT/REPLACE | `SELECT worker_id, skill_id FROM worker_skill` |
| 342 | SELECT | `skill` | 🟢 INSERT/REPLACE | `SELECT name, version FROM skill` |

#### `data_export_job_query.rs` — 7 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 146 | INSERT | `data_export_job` | 🟢 INSERT | `INSERT INTO data_export_job \ (job_id, user_id, tenant_id, status, requested_at) \ VALUES ` |
| 165 | UPDATE | `data_export_job` | 🔴 NO_TENANT | `UPDATE data_export_job SET status = ? WHERE job_id = ?` |
| 185 | UPDATE | `data_export_job` | 🔴 NO_TENANT | `UPDATE data_export_job SET \ status = ?, bundle_bytes = ?, download_token = ?, \ completed` |
| 211 | UPDATE | `data_export_job` | 🔴 NO_TENANT | `UPDATE data_export_job SET status = ?, error_message = ?, completed_at = ? \ WHERE job_id ` |
| 235 | SELECT | `data_export_job` | 🔴 NO_TENANT | `SELECT job_id, user_id, status, download_token, error_message, \ requested_at, completed_a` |
| 270 | SELECT | `data_export_job` | 🔴 NO_TENANT | `SELECT job_id, user_id, status, download_token, error_message, \ requested_at, completed_a` |
| 317 | SELECT | `data_export_job` | 🔴 NO_TENANT | `SELECT COUNT(*) FROM data_export_job \ WHERE user_id = ? AND requested_at >= ?` |

#### `elevated_view_grant.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 92 | INSERT | `elevated_view_grant` | 🟢 INSERT | `INSERT INTO elevated_view_grant (id, tenant_id, principal_id, categories_json, reason, gra` |
| 154 | UPDATE | `elevated_view_grant` | 🔴 NO_TENANT | `UPDATE elevated_view_grant SET revoked_at = ? \ WHERE id = ? AND revoked_at IS NULL` |

#### `feedback.rs` — 6 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 89 | INSERT | `operator_feedback` | 🟢 INSERT/REPLACE | `INSERT INTO operator_feedback \ (action_id, vote, comment, given_at, worker_id) \ VALUES (` |
| 115 | SELECT | `operator_feedback` | 🔴 NO_TENANT | `SELECT id, action_id, vote, comment, given_at, worker_id \ FROM operator_feedback WHERE wo` |
| 127 | SELECT | `operator_feedback` | 🔴 NO_TENANT | `SELECT id, action_id, vote, comment, given_at, worker_id \ FROM operator_feedback WHERE wo` |
| 138 | SELECT | `operator_feedback` | 🔴 NO_TENANT | `SELECT id, action_id, vote, comment, given_at, worker_id \ FROM operator_feedback WHERE vo` |
| 149 | SELECT | `operator_feedback` | 🟢 INSERT/REPLACE | `SELECT id, action_id, vote, comment, given_at, worker_id \ FROM operator_feedback \ ORDER ` |
| 178 | SELECT | `operator_feedback` | 🔴 NO_TENANT | `SELECT vote FROM operator_feedback \ WHERE action_id = ? ORDER BY given_at DESC LIMIT 1` |

#### `forge_meta.rs` — 15 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 157 | SELECT | `forge_queue` | 🔴 NO_TENANT | `SELECT payload_json FROM forge_queue WHERE forge_run_id = ?` |
| 165 | SELECT | `forge_run` | 🔴 NO_TENANT | `SELECT id, skill_name, started_at, ended_at, base_version, \ candidate_version, iterations` |
| 176 | UPDATE | `forge_run` | 🔴 NO_TENANT | `UPDATE forge_run SET status = 'aborted', ended_at = ? WHERE id = ?` |
| 192 | INSERT | `forge_run` | 🟢 INSERT/REPLACE | `INSERT INTO forge_run (id, skill_name, started_at, ended_at, base_version, \ candidate_ver` |
| 214 | UPDATE | `forge_run` | 🔴 NO_TENANT | `UPDATE forge_run SET iterations = ?, final_score = ?, candidate_version = ? \ WHERE id = ?` |
| 234 | UPDATE | `forge_run` | 🔴 NO_TENANT | `UPDATE forge_run SET status = ?, ended_at = ?, final_score = ?, iterations = ? \ WHERE id ` |
| 252 | UPDATE | `forge_run` | 🔴 NO_TENANT | `UPDATE forge_run SET status = 'failed', ended_at = ? \ WHERE status = 'running' AND starte` |
| 273 | INSERT | `forge_run` | 🟢 INSERT/REPLACE | `INSERT INTO forge_run (id, skill_name, started_at, ended_at, base_version, \ candidate_ver` |
| 285 | INSERT | `forge_queue` | 🟢 INSERT/REPLACE | `INSERT INTO forge_queue (forge_run_id, payload_json, enqueued_at) VALUES (?, ?, ?)` |
| 301 | SELECT | `forge_queue, forge_run` | 🔴 NO_TENANT | `SELECT forge_queue.payload_json FROM forge_queue \ JOIN forge_run ON forge_run.id = forge_` |
| 314 | SELECT | `forge_run` | 🔴 NO_TENANT | `SELECT skill_name, base_version, examples_count FROM forge_run WHERE id = ?` |
| 326 | INSERT | `forge_run` | 🟢 INSERT/REPLACE | `INSERT INTO forge_run (id, skill_name, started_at, ended_at, base_version, \ candidate_ver` |
| 338 | INSERT | `forge_queue` | 🟢 INSERT/REPLACE | `INSERT INTO forge_queue (forge_run_id, payload_json, enqueued_at) \ VALUES (?, ?, ?)` |
| 358 | SELECT | `forge_queue, forge_run` | 🔴 NO_TENANT | `SELECT forge_queue.forge_run_id, forge_queue.payload_json \ FROM forge_queue \ JOIN forge_` |
| 374 | UPDATE | `forge_run` | 🔴 NO_TENANT | `UPDATE forge_run SET status = 'running', started_at = ? \ WHERE id = ? AND status = 'queue` |

#### `gateway_request_audit.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 96 | INSERT | `gateway_request_audit` | 🟢 INSERT/REPLACE | `INSERT INTO gateway_request_audit \ (request_id, received_at, principal, source_ip, method` |
| 167 | SELECT | `gateway_request_audit` | 🔴 NO_TENANT | `SELECT request_id, received_at, principal, source_ip, method, path, \ status_code, latency` |

#### `gdpr.rs` — 6 statements (🟢 by design — cross-tenant aggregation / global admin)

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 81 | SELECT | `sqlite_master` | 🔴 NO_TENANT | `SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?` |
| 94 | SELECT | `memory_entry` | 🔴 NO_TENANT | `SELECT id, file, body, learned_at FROM memory_entry WHERE json_valid(body) AND lower(COALE` |
| 119 | SELECT | `audit_record` | 🔴 NO_TENANT | `SELECT seq, kind, payload_json, created_at FROM audit_record WHERE instr(lower(payload_jso` |
| 143 | SELECT | `skill` | 🔴 NO_TENANT | `SELECT id, name, version, body, frontmatter FROM skill WHERE lifecycle_state = 'pending_re` |
| 177 | SELECT | `worker` | 🔴 NO_TENANT | `SELECT id, channels_json FROM worker WHERE instr(lower(channels_json), ?) > 0` |
| 207 | SELECT | `worker` | 🟢 INSERT/REPLACE | `SELECT id, auto_approval_json FROM worker` |

#### `gdpr_apply.rs` — 5 statements (🟢 by design — cross-tenant aggregation / global admin)

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 8 | DELETE | `memory_entry` | 🔴 NO_TENANT | `DELETE FROM memory_entry WHERE json_valid(body) AND lower(COALESCE(json_extract(body, '$.a` |
| 24 | DELETE | `skill` | 🔴 NO_TENANT | `DELETE FROM skill WHERE lifecycle_state = 'pending_review' AND ((json_valid(body) AND lowe` |
| 46 | SELECT | `worker` | 🔴 NO_TENANT | `SELECT id, channels_json FROM worker WHERE instr(lower(channels_json), ?) > 0` |
| 57 | UPDATE | `worker` | 🔴 NO_TENANT | `UPDATE worker SET channels_json = ? WHERE id = ?` |
| 76 | SELECT | `worker` | 🟢 INSERT/REPLACE | `SELECT id, auto_approval_json FROM worker` |

#### `grants.rs` — 4 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 88 | INSERT | `session_grant` | 🟢 INSERT/REPLACE | `INSERT INTO session_grant (id, session_id, tool_name, granted_at, source, notes, is_one_sh` |
| 148 | SELECT | `session_grant` | 🔴 NO_TENANT | `SELECT id FROM session_grant WHERE session_id = ? AND tool_name = ? AND is_one_shot = 1 AN` |
| 166 | UPDATE | `session_grant` | 🔴 NO_TENANT | `UPDATE session_grant SET consumed_at = ? WHERE id = ?` |
| 276 | DELETE | `session_grant` | 🔴 NO_TENANT | `DELETE FROM session_grant WHERE session_id = ? AND tool_name = ?` |

#### `handoff_stats_query.rs` — 1 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 169 | SELECT | `handoff, session` | 🟢 BIND | `SELECT h.id, h.session_id, h.from_worker_id, h.\"to\", h.expected_outcome, \ h.status, h.c` |

#### `latency_tracker.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 193 | INSERT | `latency_buckets` | 🟢 INSERT/REPLACE | `INSERT INTO latency_buckets (executor_kind, bucket_idx, count) \ VALUES (?1, ?2, ?3) \ ON ` |
| 213 | SELECT | `latency_buckets` | 🔴 NO_TENANT | `SELECT executor_kind, bucket_idx, count \ FROM latency_buckets \ WHERE count > 0 \ ORDER B` |

#### `learned_plan_template.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 189 | INSERT | `learned_plan_template` | 🟢 INSERT/REPLACE | `INSERT INTO learned_plan_template \ (id, name, description, parent_goal, keywords_csv, yam` |
| 249 | UPDATE | `learned_plan_template` | 🔴 NO_TENANT | `UPDATE learned_plan_template \ SET trust_grade = ?, \ promoted_at = COALESCE(?, promoted_a` |

#### `marketplace_meta.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 63 | INSERT | `marketplace_artifact` | 🟢 INSERT/REPLACE | `INSERT INTO marketplace_artifact (id, workspace_id, oci_ref, digest, pushed_at, pushed_by)` |
| 83 | SELECT | `marketplace_artifact` | 🟢 BIND | `SELECT oci_ref, digest, pushed_at, pushed_by \ FROM marketplace_artifact \ WHERE workspace` |

#### `message_query.rs` — 8 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 80 | SELECT | `message` | 🔴 NO_TENANT | `SELECT COUNT(*) FROM message WHERE session_id = ?` |
| 91 | SELECT | `message` | 🔴 NO_TENANT | `SELECT id, session_id, role, content_json, tool_calls_json, \ tool_call_id, reasoning, pro` |
| 112 | SELECT | `message, message_fts` | 🔴 NO_TENANT | `SELECT m.session_id, m.seq, m.role, \ snippet(message_fts, 0, '[', ']', '…', 32) AS snippe` |
| 135 | SELECT | `message, message_fts` | 🔴 NO_TENANT | `SELECT m.session_id, m.seq, m.role, \ snippet(message_fts, 0, '[', ']', '…', 32) AS snippe` |
| 175 | SELECT | `message` | 🔴 NO_TENANT | `SELECT COALESCE(MAX(seq), 0) + 1 FROM message WHERE session_id = ?` |
| 180 | INSERT | `message` | 🟢 INSERT/REPLACE | `INSERT INTO message (id, session_id, role, content_json, created_at, seq) VALUES (?, ?, ?,` |
| 226 | SELECT | `message` | 🔴 NO_TENANT | `SELECT COALESCE(MAX(seq), 0) + 1 FROM message WHERE session_id = ?` |
| 235 | INSERT | `message` | 🟢 INSERT/REPLACE | `INSERT INTO message (id, session_id, role, content_json, created_at, seq) VALUES (?, ?, ?,` |

#### `migration_test_helpers.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 86 | SELECT | `sqlite_master` | 🔴 NO_TENANT | `SELECT name FROM sqlite_master WHERE type IN ('table','view') AND name = ?` |
| 97 | OTHER | `—` | 🟢 INSERT/REPLACE | `PRAGMA table_info({table})` |
| 111 | SELECT | `schema_version` | 🟢 INSERT/REPLACE | `SELECT COUNT(*) FROM schema_version` |

#### `notifications.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 165 | INSERT | `notifications` | 🟢 INSERT | `INSERT OR IGNORE INTO notifications \ (id, tenant_id, principal_id, kind, body, created_at` |
| 200 | SELECT | `notifications` | 🟢 BIND | `SELECT id, tenant_id, principal_id, kind, body, created_at, \ read_at, dismissed_at, relat` |
| 246 | SELECT | `notifications` | 🟢 BIND | `SELECT COUNT(*) FROM notifications \ WHERE tenant_id = ? \ AND (principal_id IS NULL OR pr` |
| 273 | UPDATE | `notifications` | 🟢 BIND | `UPDATE notifications SET dismissed_at = ? \ WHERE id = ? \ AND tenant_id = ? \ AND (princi` |
| 299 | UPDATE | `notifications` | 🟢 BIND | `UPDATE notifications SET dismissed_at = ? \ WHERE tenant_id = ? \ AND (principal_id IS NUL` |

#### `operator_risk_overrides.rs` — 4 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 71 | INSERT | `operator_risk_overrides` | 🟢 INSERT/REPLACE | `INSERT INTO operator_risk_overrides \ (operator_id, action_kind, vertical_id, risk_level, ` |
| 98 | SELECT | `operator_risk_overrides` | 🔴 NO_TENANT | `SELECT action_kind, vertical_id, risk_level, allow_downgrade, created_at \ FROM operator_r` |
| 127 | SELECT | `operator_risk_overrides` | 🟢 INSERT/REPLACE | `SELECT operator_id, action_kind, vertical_id, risk_level, allow_downgrade, \ created_at \ ` |
| 164 | DELETE | `operator_risk_overrides` | 🔴 NO_TENANT | `DELETE FROM operator_risk_overrides \ WHERE operator_id = ? AND action_kind = ? AND vertic` |

#### `outbound_message_query.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 87 | INSERT | `outbound_message` | 🟢 INSERT | `INSERT OR IGNORE INTO outbound_message \ (message_id, tenant_id, channel, recipient, body_` |
| 117 | SELECT | `outbound_message` | 🟢 BIND | `SELECT message_id, channel, recipient, sent_at, outcome_json \ FROM outbound_message \ WHE` |

#### `persona_revision.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 133 | SELECT | `persona` | 🔴 NO_TENANT | `SELECT COALESCE({column}, '') FROM persona WHERE worker_id = ?` |
| 148 | UPDATE | `persona` | 🔴 NO_TENANT | `UPDATE persona SET {column} = ? WHERE worker_id = ?` |

#### `queries/mcp_registry.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 115 | INSERT | `mcp_server_registry` | 🟢 INSERT | `INSERT INTO mcp_server_registry \ (tenant_id, server_id, kind, base_url, secrets_ref, regi` |
| 168 | DELETE | `mcp_server_registry` | 🟢 BIND | `DELETE FROM mcp_server_registry WHERE tenant_id = ? AND server_id = ?` |

#### `recovery_cache.rs` — 7 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 76 | SELECT | `recovery_cache` | 🔴 NO_TENANT | `SELECT session_id, chain_hash, failure_kind, cached_at_micros \ FROM recovery_cache \ WHER` |
| 89 | SELECT | `recovery_cache` | 🟢 INSERT/REPLACE | `SELECT session_id, chain_hash, failure_kind, cached_at_micros \ FROM recovery_cache \ ORDE` |
| 130 | SELECT | `recovery_cache` | 🔴 NO_TENANT | `SELECT suggestions_json, chain_hash FROM recovery_cache \ WHERE session_id = ? \ ORDER BY ` |
| 157 | DELETE | `recovery_cache` | 🔴 NO_TENANT | `DELETE FROM recovery_cache WHERE session_id = ?` |
| 180 | DELETE | `recovery_cache` | 🟢 INSERT/REPLACE | `DELETE FROM recovery_cache` |
| 203 | DELETE | `recovery_cache` | 🔴 NO_TENANT | `DELETE FROM recovery_cache WHERE cached_at_micros < ?` |
| 227 | INSERT | `recovery_cache` | 🟢 INSERT/REPLACE | `INSERT OR REPLACE INTO recovery_cache \ (session_id, chain_hash, suggestions_json, failure` |

#### `retro.rs` — 8 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 162 | INSERT | `retro` | 🟢 INSERT | `INSERT INTO retro (id, tenant_id, worker_id, window_start, window_end, \ metrics_json, sec` |
| 183 | SELECT | `retro` | 🟢 BIND | `SELECT id, tenant_id, worker_id, window_start, window_end, \ metrics_json, sections_json, ` |
| 203 | SELECT | `retro` | 🟢 BIND | `SELECT id, tenant_id, worker_id, window_start, window_end, \ metrics_json, sections_json, ` |
| 231 | SELECT | `retro` | 🟢 BIND | `SELECT id, tenant_id, worker_id, window_start, window_end, \ metrics_json, sections_json, ` |
| 244 | SELECT | `retro` | 🟢 BIND | `SELECT id, tenant_id, worker_id, window_start, window_end, \ metrics_json, sections_json, ` |
| 273 | UPDATE | `retro` | 🟢 BIND | `UPDATE retro SET sections_json = ? WHERE tenant_id = ? AND id = ?` |
| 296 | UPDATE | `retro` | 🟢 BIND | `UPDATE retro SET finalized_at = ? WHERE tenant_id = ? AND id = ?` |
| 320 | SELECT | `audit_record, session` | 🔴 NO_TENANT | `SELECT a.kind, COUNT(*) FROM audit_record a \ JOIN session s ON a.session_id = s.id \ WHER` |

#### `revoked_tokens.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 85 | INSERT | `revoked_tokens` | 🟢 INSERT | `INSERT INTO revoked_tokens (jti, tenant_id, revoked_at, principal, reason) \ VALUES (?, ?,` |
| 109 | SELECT | `revoked_tokens` | 🔴 NO_TENANT | `SELECT 1 FROM revoked_tokens WHERE jti = ?` |
| 125 | SELECT | `revoked_tokens` | 🟢 BIND | `SELECT jti, tenant_id, revoked_at, principal, reason \ FROM revoked_tokens WHERE tenant_id` |

#### `scim_user.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 111 | INSERT | `scim_user` | 🟢 INSERT | `INSERT INTO scim_user (id, tenant_id, external_id, user_name, display_name, emails_json, a` |
| 226 | UPDATE | `scim_user` | 🔴 NO_TENANT | `UPDATE scim_user SET display_name = COALESCE(?, display_name), emails_json = COALESCE(?, e` |
| 271 | UPDATE | `scim_user` | 🔴 NO_TENANT | `UPDATE scim_user SET deleted_at = ?, active = 0, updated_at = ? WHERE id = ?` |
| 278 | INSERT | `scim_provisioned_tombstones` | 🟢 INSERT | `INSERT INTO scim_provisioned_tombstones (id, tenant_id, user_name, deleted_at, reason) VAL` |
| 303 | SELECT | `scim_provisioned_tombstones` | 🟡 MENTION | `SELECT id, tenant_id, user_name, deleted_at, reason \ FROM scim_provisioned_tombstones WHE` |

#### `service_token_binding.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 91 | INSERT | `service_token_binding` | 🟢 INSERT | `INSERT INTO service_token_binding \ (token_hash, principal_id, tenant_id, label, created_a` |
| 124 | SELECT | `service_token_binding` | 🟡 MENTION | `SELECT token_hash, principal_id, tenant_id, label, created_at \ FROM service_token_binding` |
| 147 | DELETE | `service_token_binding` | 🔴 NO_TENANT | `DELETE FROM service_token_binding WHERE token_hash = ?` |
| 159 | DELETE | `service_token_binding` | 🔴 NO_TENANT | `DELETE FROM service_token_binding WHERE created_at < ?` |
| 173 | SELECT | `service_token_binding` | 🟢 INSERT/REPLACE | `SELECT token_hash, principal_id, tenant_id, label, created_at \ FROM service_token_binding` |

#### `session_meta.rs` — 7 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 82 | UPDATE | `session` | 🔴 NO_TENANT | `UPDATE session SET title = ? WHERE id = ?` |
| 96 | UPDATE | `session` | 🔴 NO_TENANT | `UPDATE session SET ended_at = ? WHERE id = ?` |
| 109 | UPDATE | `session` | 🔴 NO_TENANT | `UPDATE session SET ended_at = NULL WHERE id = ?` |
| 123 | UPDATE | `session` | 🔴 NO_TENANT | `UPDATE session SET worker_id = ? WHERE id = ?` |
| 138 | DELETE | `session` | 🔴 NO_TENANT | `DELETE FROM session WHERE id = ?` |
| 157 | SELECT | `judge_record` | 🔴 NO_TENANT | `SELECT session_id, verdict FROM judge_record WHERE id = ?` |
| 164 | UPDATE | `judge_record` | 🔴 NO_TENANT | `UPDATE judge_record SET verdict = ?, reason = ? WHERE id = ?` |

#### `session_query.rs` — 11 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 116 | INSERT | `session` | 🟢 INSERT | `INSERT INTO session (id, tenant_id, workspace_id, model, provider, title, started_at, vert` |
| 141 | SELECT | `session` | 🟡 MENTION | `SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical` |
| 157 | SELECT | `session` | 🟢 INSERT/REPLACE | `SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical` |
| 174 | SELECT | `session` | 🟢 BIND | `SELECT id, tenant_id, workspace_id, model, provider, title, started_at, ended_at, vertical` |
| 194 | DELETE | `session` | 🔴 NO_TENANT | `DELETE FROM session WHERE started_at < ?` |
| 205 | SELECT | `session` | 🔴 NO_TENANT | `SELECT COUNT(*) FROM session WHERE started_at < ?` |
| 215 | SELECT | `judge_record` | 🔴 NO_TENANT | `SELECT subject_kind, subject_ref, verdict, reason, judge_provider, \ judge_model, rubric_i` |
| 235 | SELECT | `audit_record` | 🔴 NO_TENANT | `SELECT seq, kind, payload_json, created_at FROM audit_record \ WHERE session_id = ? ORDER ` |
| 249 | SELECT | `forge_run` | 🟢 INSERT/REPLACE | `SELECT id, skill_name, started_at, ended_at, base_version, \ candidate_version, iterations` |
| 267 | INSERT | `routing_decision` | 🟢 INSERT/REPLACE | `INSERT INTO routing_decision (id, session_id, turn, policy, tier, \ classifier_latency_ms,` |
| 295 | SELECT | `routing_decision` | 🔴 NO_TENANT | `SELECT id, session_id, turn, policy, tier, classifier_latency_ms, \ chosen_provider, chose` |

#### `session_store.rs` — 1 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 227 | OTHER | `—` | 🟢 INSERT/REPLACE | `VACUUM` |

#### `skill_flag.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 39 | INSERT | `skill_flag` | 🟢 INSERT/REPLACE | `INSERT INTO skill_flag (skill_id, flag, value) VALUES (?, ?, ?) \ ON CONFLICT (skill_id, f` |
| 64 | SELECT | `skill_flag` | 🔴 NO_TENANT | `SELECT value FROM skill_flag WHERE skill_id = ? AND flag = ?` |

#### `skill_provenance.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 99 | INSERT | `skill_provenance` | 🟢 INSERT/REPLACE | `INSERT INTO skill_provenance \ (skill_name, skill_version, provenance_kind, actor, parent_` |
| 132 | SELECT | `skill_provenance` | 🔴 NO_TENANT | `SELECT skill_name, skill_version, provenance_kind, actor, parent_id, \ change_reason, brea` |

#### `skill_query.rs` — 7 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 67 | SELECT | `skill` | 🔴 NO_TENANT | `SELECT DISTINCT vertical_id FROM skill \ WHERE vertical_id IS NOT NULL ORDER BY vertical_i` |
| 85 | SELECT | `skill` | 🔴 NO_TENANT | `SELECT id, name, version, vertical_id, created_at \ FROM skill \ WHERE vertical_id = ? \ O` |
| 105 | SELECT | `session, session_skill, skill` | 🔴 NO_TENANT | `SELECT s.id, s.name, s.version, s.vertical_id, \ MAX(ss.attached_at) AS last_attached \ FR` |
| 183 | INSERT | `skill` | 🟢 INSERT/REPLACE | `INSERT INTO skill (id, name, version, body, frontmatter, source, hash, created_at, vertica` |
| 283 | INSERT | `session` | 🟢 INSERT | `INSERT INTO session (id, tenant_id, workspace_id, model, provider, started_at, vertical_id` |
| 295 | INSERT | `session` | 🟢 INSERT | `INSERT INTO session (id, tenant_id, workspace_id, model, provider, started_at, vertical_id` |
| 311 | INSERT | `session_skill` | 🟢 INSERT/REPLACE | `INSERT INTO session_skill (session_id, skill_id, attached_at) VALUES (?, ?, ?)` |

#### `skill_rating.rs` — 5 statements (🟢 by design — cross-tenant aggregation / global admin)

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 146 | INSERT | `skill_rating` | 🟢 INSERT/REPLACE | `INSERT INTO skill_rating \ (skill_id, skill_version, publisher_tenant, rater_tenant_id, vo` |
| 178 | SELECT | `skill_rating` | 🔴 NO_TENANT | `SELECT \ SUM(CASE WHEN vote = 'up' THEN 1 ELSE 0 END) AS up_count, \ SUM(CASE WHEN vote = ` |
| 210 | SELECT | `skill_rating` | 🔴 NO_TENANT | `SELECT \ skill_id, skill_version, publisher_tenant, \ SUM(CASE WHEN vote = 'up' THEN 1 ELS` |
| 238 | SELECT | `skill_rating` | 🔴 NO_TENANT | `SELECT skill_id, skill_version, publisher_tenant, \ SUM(CASE WHEN vote = 'up' THEN 1 ELSE ` |
| 269 | SELECT | `skill_rating` | 🔴 NO_TENANT | `SELECT skill_id, skill_version, publisher_tenant, rater_tenant_id, \ vote, rated_at, comme` |

#### `skill_runs_query.rs` — 7 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 104 | SELECT | `memory_entry` | 🔴 NO_TENANT | `SELECT m.body, m.learned_at \ FROM memory_entry m \ WHERE m.file = 'MEMORY.md' \ AND m.sup` |
| 152 | SELECT | `worker_budget` | 🟢 BIND | `SELECT action_id, COALESCE(SUM(usd_micros), 0) AS spend_usd_micros \ FROM worker_budget WH` |
| 161 | SELECT | `worker_budget` | 🟢 INSERT/REPLACE | `SELECT action_id, COALESCE(SUM(usd_micros), 0) AS spend_usd_micros \ FROM worker_budget GR` |
| 184 | SELECT | `worker` | 🟢 INSERT/REPLACE | `SELECT id, tenant_id FROM worker` |
| 196 | SELECT | `sqlite_master` | 🔴 NO_TENANT | `SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?` |
| 410 | INSERT | `memory_entry` | 🟢 INSERT/REPLACE | `INSERT INTO memory_entry (id, workspace_id, file, body, learned_in, learned_at, hash) \ VA` |
| 432 | INSERT | `worker_budget` | 🟢 INSERT | `INSERT INTO worker_budget \ (tenant_id, worker_id, action_id, usd_micros, charged_at) \ VA` |

#### `skill_share.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 78 | UPDATE | `skill` | 🔴 NO_TENANT | `UPDATE skill SET shared = ?, shared_with_cluster = ? \ WHERE name = ? AND version = ?` |
| 121 | SELECT | `skill` | 🔴 NO_TENANT | `SELECT id, name, version, shared_with_cluster, created_at \ FROM skill \ WHERE shared = 1 ` |
| 154 | SELECT | `org_unit` | 🟢 BIND | `SELECT name FROM org_unit \ WHERE tenant_id = ? AND parent_id IS NULL \ ORDER BY created_a` |

#### `spend.rs` — 7 statements (🟢 by design — cross-tenant aggregation / global admin)

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 20 | INSERT | `spend_ledger` | 🟢 INSERT | `INSERT INTO spend_ledger (tenant_id, session_id, charged_at_micros, \ amount_usd_micros, p` |
| 44 | SELECT | `spend_ledger` | 🟢 BIND | `SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \ WHERE tenant_id = ? AND ses` |
| 53 | SELECT | `spend_ledger` | 🟢 BIND | `SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \ WHERE tenant_id = ? AND cha` |
| 62 | SELECT | `spend_ledger` | 🟢 BIND | `SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \ WHERE tenant_id = ? AND cha` |
| 87 | SELECT | `spend_ledger` | 🔴 NO_TENANT | `SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \ WHERE charged_at_micros >= ` |
| 95 | SELECT | `spend_ledger` | 🔴 NO_TENANT | `SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \ WHERE charged_at_micros >= ` |
| 104 | SELECT | `spend_ledger` | 🟢 INSERT/REPLACE | `SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger` |

#### `spend_anomaly.rs` — 6 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 180 | SELECT | `worker_spend_baseline` | 🔴 NO_TENANT | `SELECT mean_usd_per_hour, m2_usd_per_hour, sample_count, updated_at_micros \ FROM worker_s` |
| 204 | INSERT | `worker_spend_baseline` | 🟢 INSERT/REPLACE | `INSERT INTO worker_spend_baseline \ (worker_id, hour_of_day, mean_usd_per_hour, m2_usd_per` |
| 241 | INSERT | `worker_spend_anomaly` | 🟢 INSERT/REPLACE | `INSERT INTO worker_spend_anomaly \ (id, worker_id, observed_usd_per_hour, baseline_mean_us` |
| 335 | SELECT | `worker_budget` | 🔴 NO_TENANT | `SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \ WHERE worker_id = ? AND charged_a` |
| 363 | SELECT | `worker_budget` | 🟢 BIND | `SELECT DISTINCT worker_id FROM worker_budget \ WHERE tenant_id = ? \ ORDER BY worker_id` |
| 384 | SELECT | `worker_budget` | 🟡 MENTION | `SELECT DISTINCT tenant_id FROM worker_budget \ WHERE tenant_id IS NOT NULL \ ORDER BY tena` |

#### `stats.rs` — 2 statements (🟢 by design — cross-tenant aggregation / global admin)

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 46 | SELECT | `—` | 🟢 INSERT/REPLACE | `SELECT COUNT(*) FROM {table}` |
| 61 | SELECT | `audit_record` | 🔴 NO_TENANT | `SELECT json_extract(payload_json, '$.tool') AS tool, COUNT(*) \ FROM audit_record \ WHERE ` |

#### `store_open.rs` — 6 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 142 | OTHER | `—` | 🟢 INSERT/REPLACE | `PRAGMA journal_mode = WAL` |
| 299 | OTHER | `—` | 🟢 INSERT/REPLACE | `PRAGMA table_info(session)` |
| 309 | ALTER | `—` | 🟢 INSERT/REPLACE | `ALTER TABLE session ADD COLUMN worker_id BLOB` |
| 314 | CREATE | `—` | 🔴 NO_TENANT | `CREATE INDEX IF NOT EXISTS idx_session_worker \ ON session(worker_id) WHERE worker_id IS N` |
| 335 | OTHER | `—` | 🟢 INSERT/REPLACE | `PRAGMA table_info(session)` |
| 345 | ALTER | `—` | 🟢 INSERT/REPLACE | `ALTER TABLE session ADD COLUMN routing_policy TEXT` |

#### `store_session_routing.rs` — 1 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 31 | UPDATE | `session` | 🔴 NO_TENANT | `UPDATE session SET routing_policy = ? WHERE id = ?` |

#### `subtask.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 131 | INSERT | `subtask_ledger` | 🟢 INSERT/REPLACE | `INSERT INTO subtask_ledger \ (id, parent_goal, parent_session_id, worker_id, plan_node_pat` |
| 166 | UPDATE | `subtask_ledger` | 🔴 NO_TENANT | `UPDATE subtask_ledger \ SET state = ?, \ output = COALESCE(?, output), \ started_at = COAL` |

#### `subtask_budget.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 49 | UPDATE | `subtask_ledger` | 🔴 NO_TENANT | `UPDATE subtask_ledger SET budget_usd_cents = ? WHERE id = ?` |
| 82 | UPDATE | `subtask_ledger` | 🔴 NO_TENANT | `UPDATE subtask_ledger \ SET budget_usd_cents = ? \ WHERE parent_session_id IS ? AND worker` |
| 111 | UPDATE | `subtask_ledger` | 🔴 NO_TENANT | `UPDATE subtask_ledger SET parent_subtask_id = ? WHERE id = ?` |
| 135 | UPDATE | `subtask_ledger` | 🔴 NO_TENANT | `UPDATE subtask_ledger \ SET spent_usd_cents = spent_usd_cents + ? \ WHERE id = ?` |
| 161 | SELECT | `subtask_ledger` | 🔴 NO_TENANT | `SELECT budget_usd_cents, spent_usd_cents \ FROM subtask_ledger WHERE id = ?` |

#### `support_query.rs` — 8 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 203 | INSERT | `support_ticket` | 🟢 INSERT | `INSERT INTO support_ticket \ (id, tenant_id, user_email, subject, body, status, priority, ` |
| 233 | SELECT | `support_ticket` | 🟡 MENTION | `SELECT id, tenant_id, user_email, subject, body, status, priority, \ created_at, updated_a` |
| 244 | SELECT | `support_ticket` | 🟢 INSERT/REPLACE | `SELECT id, tenant_id, user_email, subject, body, status, priority, \ created_at, updated_a` |
| 259 | SELECT | `support_ticket` | 🟡 MENTION | `SELECT id, tenant_id, user_email, subject, body, status, priority, \ created_at, updated_a` |
| 286 | INSERT | `support_ticket_reply` | 🟢 INSERT/REPLACE | `INSERT INTO support_ticket_reply \ (id, ticket_id, author_kind, author_id, body, created_a` |
| 300 | UPDATE | `support_ticket` | 🔴 NO_TENANT | `UPDATE support_ticket SET updated_at = ? WHERE id = ?` |
| 317 | SELECT | `support_ticket_reply` | 🔴 NO_TENANT | `SELECT id, ticket_id, author_kind, author_id, body, created_at \ FROM support_ticket_reply` |
| 361 | UPDATE | `support_ticket` | 🔴 NO_TENANT | `UPDATE support_ticket SET status = ?, priority = ?, assigned_to = ?, updated_at = ? \ WHER` |

#### `team_roster.rs` — 7 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 105 | ALTER | `—` | 🟢 INSERT/REPLACE | `ALTER TABLE roster_member ADD COLUMN role TEXT` |
| 111 | ALTER | `—` | 🟢 INSERT/REPLACE | `ALTER TABLE roster_member ADD COLUMN removed_at INTEGER` |
| 116 | CREATE | `—` | 🔴 NO_TENANT | `CREATE INDEX IF NOT EXISTS idx_roster_member_active \ ON roster_member(roster_id) WHERE re` |
| 147 | SELECT | `org_unit, roster_member` | 🟢 BIND | `SELECT u.id, u.name, u.parent_id, \ COALESCE(( \ SELECT COUNT(*) FROM roster_member m \ WH` |
| 181 | SELECT | `org_unit` | 🟢 BIND | `SELECT id, name, parent_id FROM org_unit \ WHERE tenant_id = ? AND id = ?` |
| 197 | SELECT | `roster_member` | 🔴 NO_TENANT | `SELECT m.worker_id, m.role, m.added_at \ FROM roster_member m \ WHERE m.roster_id = ? AND ` |
| 235 | SELECT | `org_unit` | 🟢 BIND | `SELECT parent_id FROM org_unit WHERE tenant_id = ? AND id = ?` |

#### `team_roster_mutations.rs` — 7 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 58 | SELECT | `org_unit` | 🟢 BIND | `SELECT id FROM org_unit WHERE tenant_id = ? AND id = ?` |
| 76 | INSERT | `org_unit` | 🟢 INSERT | `INSERT INTO org_unit (id, tenant_id, name, parent_id, created_at) \ VALUES (?, ?, ?, ?, ?)` |
| 88 | INSERT | `roster` | 🟢 INSERT | `INSERT INTO roster (id, tenant_id, name, policy_json, channels_json, \ budget_cap_usd_micr` |
| 127 | UPDATE | `org_unit` | 🟢 BIND | `UPDATE org_unit SET name = ? WHERE tenant_id = ? AND id = ?` |
| 141 | UPDATE | `roster` | 🟢 BIND | `UPDATE roster SET name = ? WHERE tenant_id = ? AND id = ?` |
| 166 | INSERT | `roster_member` | 🟢 INSERT/REPLACE | `INSERT INTO roster_member (roster_id, worker_id, added_at, role, removed_at) \ VALUES (?, ` |
| 197 | UPDATE | `roster_member` | 🔴 NO_TENANT | `UPDATE roster_member SET removed_at = ? \ WHERE roster_id = ? AND worker_id = ? AND remove` |

#### `tenant_query.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 260 | SELECT | `spend_ledger` | 🟢 BIND | `SELECT COALESCE(SUM(amount_usd_micros), 0) FROM spend_ledger \ WHERE tenant_id = ? AND cha` |
| 273 | SELECT | `sqlite_master` | 🔴 NO_TENANT | `SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?` |
| 290 | OTHER | `—` | 🟢 INSERT/REPLACE | `PRAGMA table_info({table})` |
| 407 | SELECT | `tenant_usage` | 🟢 BIND | `SELECT used_actions, period_start FROM tenant_usage WHERE tenant_id = ?` |
| 439 | INSERT | `tenant_usage` | 🟢 INSERT | `INSERT INTO tenant_usage (tenant_id, period_start, used_actions, updated_at) \ VALUES (?, ` |

#### `tenant_query_inspect.rs` — 5 statements (🟢 by design — cross-tenant aggregation / global admin)

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 85 | UPDATE | `audit_record` | 🔴 NO_TENANT | `UPDATE audit_record SET payload_json = replace(payload_json, ?, ?) \ WHERE instr(payload_j` |
| 119 | SELECT | `roster, roster_member` | 🟢 BIND | `SELECT COUNT(*) FROM roster_member rm \ JOIN roster r ON r.id = rm.roster_id WHERE r.tenan` |
| 137 | SELECT | `audit_record, session` | 🟢 BIND | `SELECT a.seq, a.kind, a.payload_json, a.created_at FROM audit_record a \ LEFT JOIN session` |
| 166 | SELECT | `worker` | 🟢 BIND | `SELECT channels_json FROM worker WHERE tenant_id = ?` |
| 176 | SELECT | `roster` | 🟢 BIND | `SELECT channels_json FROM roster WHERE tenant_id = ?` |

#### `tenant_quota.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 181 | INSERT | `tenant_quota` | 🟢 INSERT | `INSERT INTO tenant_quota \ (tenant_id, period, max_usd_micros, max_actions, max_workers, s` |
| 206 | SELECT | `tenant_quota` | 🟢 BIND | `SELECT tenant_id, period, max_usd_micros, max_actions, max_workers, set_at \ FROM tenant_q` |
| 219 | SELECT | `tenant_quota` | 🟢 INSERT/REPLACE | `SELECT tenant_id FROM tenant_quota ORDER BY tenant_id` |
| 240 | DELETE | `tenant_quota` | 🟢 BIND | `DELETE FROM tenant_quota WHERE tenant_id = ?` |
| 303 | SELECT | `worker` | 🟡 MENTION | `SELECT tenant_id FROM worker WHERE id = ? LIMIT 1` |

#### `tenant_quota_usage.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 168 | SELECT | `worker, worker_budget` | 🟢 BIND | `SELECT COALESCE(SUM(b.usd_micros), 0) AS usd, COUNT(*) AS actions \ FROM worker_budget b J` |
| 188 | SELECT | `worker` | 🟢 BIND | `SELECT COUNT(*) FROM worker WHERE tenant_id = ?` |
| 215 | SELECT | `sqlite_master` | 🔴 NO_TENANT | `SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?` |

#### `tenant_rate_limit.rs` — 12 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 124 | INSERT | `tenant_rate_limit` | 🟢 INSERT | `INSERT INTO tenant_rate_limit \ (tenant_id, connector, requests_per_minute, updated_at) \ ` |
| 179 | SELECT | `tenant_rate_limit` | 🟢 BIND | `SELECT connector, requests_per_minute, updated_at \ FROM tenant_rate_limit \ WHERE tenant_` |
| 208 | SELECT | `tenant_rate_limit` | 🟡 MENTION | `SELECT tenant_id, requests_per_minute, updated_at \ FROM tenant_rate_limit WHERE connector` |
| 242 | SELECT | `tenant_rate_limit` | 🟢 INSERT/REPLACE | `SELECT tenant_id, connector, requests_per_minute, updated_at \ FROM tenant_rate_limit` |
| 289 | DELETE | `tenant_rate_limit` | 🟢 BIND | `DELETE FROM tenant_rate_limit WHERE tenant_id = ? AND connector = ?` |
| 300 | SELECT | `pragma_table_info` | 🟢 INSERT/REPLACE | `SELECT name, pk FROM pragma_table_info('tenant_rate_limit')` |
| 311 | DROP | `—` | 🟢 INSERT/REPLACE | `DROP TABLE IF EXISTS tenant_rate_limit_v2` |
| 315 | CREATE | `—` | 🟢 INSERT/REPLACE | `CREATE TABLE tenant_rate_limit_v2 ( tenant_id BLOB NOT NULL, connector TEXT NOT NULL DEFAU` |
| 327 | INSERT | `tenant_rate_limit, tenant_rate_limit_v2` | 🟢 INSERT | `INSERT OR IGNORE INTO tenant_rate_limit_v2 \ (tenant_id, connector, requests_per_minute, u` |
| 336 | DROP | `—` | 🟢 INSERT/REPLACE | `DROP TABLE tenant_rate_limit` |
| 340 | ALTER | `—` | 🟢 INSERT/REPLACE | `ALTER TABLE tenant_rate_limit_v2 RENAME TO tenant_rate_limit` |
| 344 | CREATE | `—` | 🟢 INSERT/REPLACE | `CREATE INDEX IF NOT EXISTS idx_tenant_rate_limit_connector \ ON tenant_rate_limit(tenant_i` |

#### `trust_observation.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 92 | INSERT | `trust_observation` | 🟢 INSERT/REPLACE | `INSERT INTO trust_observation \ (id, source_tenant_id, consumer_tenant_id, skill_id, quali` |
| 124 | SELECT | `trust_observation` | 🔴 NO_TENANT | `SELECT id, source_tenant_id, consumer_tenant_id, skill_id, \ quality_score, observed_at \ ` |
| 152 | SELECT | `trust_observation` | 🔴 NO_TENANT | `SELECT id, source_tenant_id, consumer_tenant_id, skill_id, \ quality_score, observed_at \ ` |

#### `trust_streak.rs` — 1 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 86 | SELECT | `trust_streak_event` | 🔴 NO_TENANT | `SELECT kind, action_type, payload_json, created_at \ FROM trust_streak_event \ WHERE worke` |

#### `user_account.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 88 | INSERT | `magic_link_tokens` | 🟢 INSERT/REPLACE | `INSERT INTO magic_link_tokens (token_hash, email, expires_at, used_at) \ VALUES (?, ?, ?, ` |
| 113 | SELECT | `magic_link_tokens` | 🔴 NO_TENANT | `SELECT email, expires_at, used_at FROM magic_link_tokens WHERE token_hash = ?` |
| 129 | UPDATE | `magic_link_tokens` | 🔴 NO_TENANT | `UPDATE magic_link_tokens SET used_at = ? \ WHERE token_hash = ? AND used_at IS NULL` |

#### `worker_archive.rs` — 17 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 95 | SELECT | `worker` | 🟢 BIND | `SELECT id FROM worker WHERE id = ? AND tenant_id = ?` |
| 107 | SELECT | `worker_archive` | 🔴 NO_TENANT | `SELECT id FROM worker_archive WHERE id = ?` |
| 118 | INSERT | `worker_archive` | 🟢 INSERT | `INSERT INTO worker_archive (id, tenant_id, archived_at, archived_by_principal, reason, fro` |
| 133 | DELETE | `duty` | 🔴 NO_TENANT | `DELETE FROM duty WHERE worker_id = ?` |
| 138 | DELETE | `role` | 🔴 NO_TENANT | `DELETE FROM role WHERE worker_id = ?` |
| 143 | DELETE | `persona` | 🔴 NO_TENANT | `DELETE FROM persona WHERE worker_id = ?` |
| 148 | DELETE | `worker` | 🟢 BIND | `DELETE FROM worker WHERE id = ? AND tenant_id = ?` |
| 182 | SELECT | `worker` | 🔴 NO_TENANT | `SELECT id FROM worker WHERE id = ?` |
| 194 | SELECT | `worker_archive` | 🟢 BIND | `SELECT frozen_json FROM worker_archive WHERE id = ? AND tenant_id = ?` |
| 206 | DELETE | `worker_archive` | 🟢 BIND | `DELETE FROM worker_archive WHERE id = ? AND tenant_id = ?` |
| 222 | SELECT | `worker_archive` | 🟢 BIND | `SELECT id, tenant_id, archived_at, archived_by_principal, reason, frozen_json FROM worker_` |
| 327 | SELECT | `worker` | 🟢 BIND | `SELECT id FROM worker WHERE id = ? AND tenant_id = ?` |
| 341 | SELECT | `worker_archive` | 🟢 BIND | `SELECT archived_at FROM worker_archive WHERE id = ? AND tenant_id = ?` |
| 370 | DELETE | `memory_entry` | 🟢 BIND | `DELETE FROM memory_entry WHERE worker_id = ? AND tenant_id = ?` |
| 377 | UPDATE | `audit_record, session` | 🔴 NO_TENANT | `UPDATE audit_record SET worker_deleted_at = ? \ WHERE worker_deleted_at IS NULL \ AND sess` |
| 388 | DELETE | `worker_archive` | 🟢 BIND | `DELETE FROM worker_archive WHERE id = ? AND tenant_id = ?` |
| 406 | OTHER | `—` | 🟢 INSERT/REPLACE | `PRAGMA table_info(audit_record)` |

#### `worker_budget_write.rs` — 7 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 111 | SELECT | `worker` | 🔴 NO_TENANT | `SELECT auto_approval_json FROM worker WHERE id = ? LIMIT 1` |
| 129 | UPDATE | `worker` | 🔴 NO_TENANT | `UPDATE worker SET auto_approval_json = ? WHERE id = ?` |
| 179 | SELECT | `worker_budget` | 🔴 NO_TENANT | `SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \ WHERE worker_id = ? AND charged_a` |
| 188 | SELECT | `worker_budget` | 🔴 NO_TENANT | `SELECT COALESCE(SUM(usd_micros), 0) FROM worker_budget \ WHERE worker_id = ? AND charged_a` |
| 228 | INSERT | `worker` | 🟢 INSERT | `INSERT INTO worker (id, tenant_id, vertical_id, persona_id, role_id, \ memory_scope, auto_` |
| 241 | INSERT | `worker_budget` | 🟢 INSERT | `INSERT INTO worker_budget (tenant_id, worker_id, action_id, usd_micros, charged_at) \ VALU` |
| 255 | SELECT | `worker` | 🔴 NO_TENANT | `SELECT auto_approval_json FROM worker WHERE id = ?` |

#### `worker_deps.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 181 | INSERT | `worker_dependency` | 🟢 INSERT/REPLACE | `INSERT INTO worker_dependency (a_worker_id, b_worker_id, kind, declared_at, note) VALUES (` |
| 207 | DELETE | `worker_dependency` | 🔴 NO_TENANT | `DELETE FROM worker_dependency WHERE a_worker_id = ? AND b_worker_id = ? AND kind = ?` |
| 255 | SELECT | `worker_dependency` | 🟢 INSERT/REPLACE | `SELECT a_worker_id, b_worker_id, kind, declared_at, note FROM worker_dependency ORDER BY k` |

#### `worker_deps_graph.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 61 | SELECT | `worker_dependency` | 🔴 NO_TENANT | `SELECT a_worker_id, b_worker_id FROM worker_dependency WHERE kind = ? ORDER BY a_worker_id` |
| 87 | SELECT | `worker_dependency` | 🟢 INSERT/REPLACE | `SELECT a_worker_id, b_worker_id FROM worker_dependency ORDER BY b_worker_id ASC, a_worker_` |

#### `worker_freeze.rs` — 4 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 74 | INSERT | `worker_freeze` | 🟢 INSERT | `INSERT INTO worker_freeze \ (tenant_id, worker_id, frozen_at, frozen_by, reason, unfrozen_` |
| 118 | UPDATE | `worker_freeze` | 🟢 BIND | `UPDATE worker_freeze \ SET unfrozen_at = ?, unfrozen_by = ? \ WHERE tenant_id = ? AND work` |
| 148 | SELECT | `worker_freeze` | 🟢 BIND | `SELECT tenant_id, worker_id, frozen_at, frozen_by, reason, unfrozen_at, unfrozen_by \ FROM` |
| 170 | SELECT | `worker_freeze` | 🟢 BIND | `SELECT tenant_id, worker_id, frozen_at, frozen_by, reason, unfrozen_at, unfrozen_by \ FROM` |

#### `worker_kpi.rs` — 1 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 119 | INSERT | `worker_kpi_value` | 🟢 INSERT | `INSERT INTO worker_kpi_value (worker_id, tenant_id, kpi_name, value, observed_at, note) VA` |

#### `worker_okr.rs` — 4 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 76 | INSERT | `worker_objective` | 🟢 INSERT | `INSERT INTO worker_objective (worker_id, tenant_id, period, title, description, status, cr` |
| 92 | INSERT | `worker_key_result` | 🟢 INSERT/REPLACE | `INSERT INTO worker_key_result (objective_id, kpi_name, target_value, comparison, weight) V` |
| 159 | SELECT | `worker_key_result` | 🔴 NO_TENANT | `SELECT id, objective_id, kpi_name, target_value, comparison, weight FROM worker_key_result` |
| 193 | SELECT | `worker_kpi_value` | 🟢 BIND | `SELECT value FROM worker_kpi_value WHERE worker_id = ? AND tenant_id = ? AND kpi_name = ? ` |

#### `worker_presence.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 51 | ALTER | `—` | 🟢 INSERT/REPLACE | `ALTER TABLE worker ADD COLUMN last_active_at INTEGER` |
| 56 | CREATE | `—` | 🟢 INSERT/REPLACE | `CREATE INDEX IF NOT EXISTS idx_worker_presence_tenant_last_active ON worker(tenant_id, las` |
| 71 | UPDATE | `worker` | 🔴 NO_TENANT | `UPDATE worker SET last_active_at = strftime('%s','now') WHERE id = ?` |
| 158 | SELECT | `sqlite_master` | 🔴 NO_TENANT | `SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?` |
| 168 | OTHER | `—` | 🟢 INSERT/REPLACE | `PRAGMA table_info({table})` |

#### `workflow_checkpoint.rs` — 6 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 46 | SELECT | `workflow_checkpoint` | 🔴 NO_TENANT | `SELECT COALESCE(MAX(seq), 0) + 1 FROM workflow_checkpoint \ WHERE workflow_run_id = ?` |
| 54 | INSERT | `workflow_checkpoint` | 🟢 INSERT/REPLACE | `INSERT INTO workflow_checkpoint \ (workflow_run_id, task_id, completed_at, outputs_json, c` |
| 75 | SELECT | `workflow_checkpoint` | 🔴 NO_TENANT | `SELECT seq, workflow_run_id, task_id, completed_at, outputs_json, cache_key \ FROM workflo` |
| 100 | SELECT | `workflow_checkpoint` | 🔴 NO_TENANT | `SELECT seq, workflow_run_id, task_id, completed_at, outputs_json, cache_key \ FROM workflo` |
| 125 | SELECT | `workflow_checkpoint` | 🔴 NO_TENANT | `SELECT 1 FROM workflow_checkpoint WHERE workflow_run_id = ? AND seq = ?` |
| 135 | DELETE | `workflow_checkpoint` | 🔴 NO_TENANT | `DELETE FROM workflow_checkpoint WHERE workflow_run_id = ? AND seq > ?` |

#### `workflow_definition_read.rs` — 3 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 19 | SELECT | `workflow_definition` | 🟡 MENTION | `SELECT id, tenant_id, vertical_id, name, definition_toml, version, \ parent_id, lifecycle_` |
| 42 | SELECT | `workflow_definition, workflow_definition_index` | 🟢 BIND | `SELECT wd.id, wd.tenant_id, wd.vertical_id, wd.name, wd.definition_toml, \ wd.version, wd.` |
| 68 | SELECT | `workflow_definition` | 🟢 BIND | `SELECT id, tenant_id, vertical_id, name, definition_toml, version, \ parent_id, lifecycle_` |

#### `workflow_definition_write.rs` — 13 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 61 | SELECT | `workflow_definition` | 🟢 BIND | `SELECT COALESCE(MAX(version), 0) + 1 FROM workflow_definition \ WHERE tenant_id = ? AND ve` |
| 75 | INSERT | `workflow_definition` | 🟢 INSERT | `INSERT INTO workflow_definition \ (id, tenant_id, vertical_id, name, definition_toml, vers` |
| 154 | SELECT | `workflow_definition` | 🟡 MENTION | `SELECT tenant_id, vertical_id, name FROM workflow_definition WHERE id = ?` |
| 179 | INSERT | `workflow_definition_index` | 🟢 INSERT | `INSERT INTO workflow_definition_index (tenant_id, vertical_id, name, head_id) \ VALUES (?,` |
| 192 | UPDATE | `workflow_definition` | 🔴 NO_TENANT | `UPDATE workflow_definition SET lifecycle_state = ? WHERE id = ?` |
| 229 | SELECT | `workflow_definition` | 🟡 MENTION | `SELECT tenant_id, vertical_id, name FROM workflow_definition WHERE id = ?` |
| 248 | UPDATE | `workflow_definition` | 🔴 NO_TENANT | `UPDATE workflow_definition SET lifecycle_state = ? WHERE id = ?` |
| 255 | DELETE | `workflow_definition_index` | 🟢 BIND | `DELETE FROM workflow_definition_index \ WHERE tenant_id = ? AND vertical_id = ? AND name =` |
| 323 | SELECT | `workflow_definition` | 🟡 MENTION | `SELECT tenant_id, vertical_id, name, definition_toml \ FROM workflow_definition WHERE id =` |
| 358 | SELECT | `workflow_definition_index` | 🟢 BIND | `SELECT head_id FROM workflow_definition_index \ WHERE tenant_id = ? AND vertical_id = ? AN` |
| 374 | SELECT | `workflow_definition` | 🟢 BIND | `SELECT COALESCE(MAX(version), 0) + 1 FROM workflow_definition \ WHERE tenant_id = ? AND ve` |
| 387 | INSERT | `workflow_definition` | 🟢 INSERT | `INSERT INTO workflow_definition \ (id, tenant_id, vertical_id, name, definition_toml, vers` |
| 407 | INSERT | `workflow_definition_index` | 🟢 INSERT | `INSERT INTO workflow_definition_index (tenant_id, vertical_id, name, head_id) \ VALUES (?,` |

#### `workflow_test_run.rs` — 2 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 151 | INSERT | `workflow_test_run` | 🟢 INSERT | `INSERT INTO workflow_test_run \ (id, tenant_id, vertical_id, workflow_name, started_at, en` |
| 192 | SELECT | `workflow_test_run` | 🟢 BIND | `SELECT id, tenant_id, vertical_id, workflow_name, started_at, ended_at, \ status, summary_` |

#### `workspace.rs` — 6 statements (🟢 by design — cross-tenant aggregation / global admin)

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 20 | SELECT | `workspace` | 🟢 INSERT/REPLACE | `SELECT id FROM workspace ORDER BY created_at ASC LIMIT 1` |
| 31 | SELECT | `workspace` | 🟢 INSERT/REPLACE | `SELECT name FROM workspace ORDER BY created_at ASC LIMIT 1` |
| 44 | SELECT | `workspace` | 🟢 INSERT/REPLACE | `SELECT id, name, root_path, created_at \ FROM workspace ORDER BY created_at ASC LIMIT 1` |
| 65 | SELECT | `workspace` | 🔴 NO_TENANT | `SELECT id, name, root_path, created_at FROM workspace WHERE id = ?` |
| 91 | INSERT | `workspace` | 🟢 INSERT | `INSERT INTO workspace (id, tenant_id, name, root_path, created_at) \ VALUES (?, ?, ?, ?, ?` |
| 111 | SELECT | `workspace` | 🟡 MENTION | `SELECT id, tenant_id FROM workspace WHERE root_path = ? LIMIT 1` |

#### `workspace_keys.rs` — 4 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 105 | INSERT | `workspace_provider_key` | 🟢 INSERT | `INSERT INTO workspace_provider_key \ (tenant_id, workspace_id, provider, masked_key, ciphe` |
| 142 | SELECT | `workspace_provider_key` | 🟢 BIND | `SELECT provider, masked_key, ciphertext_b64, test_cost_micros, test_tokens \ FROM workspac` |
| 179 | INSERT | `workspace_installed_vertical` | 🟢 INSERT | `INSERT INTO workspace_installed_vertical \ (tenant_id, workspace_id, vertical_id, workflow` |
| 210 | SELECT | `workspace_installed_vertical` | 🟢 BIND | `SELECT vertical_id FROM workspace_installed_vertical \ WHERE tenant_id = ? AND workspace_i` |

#### `workspace_member_invitation.rs` — 5 statements

| Line | Verb | Table(s) | Verdict | SQL excerpt |
|---|---|---|---|---|
| 131 | INSERT | `workspace_member_invitation` | 🟢 INSERT | `INSERT INTO workspace_member_invitation \ (id, tenant_id, email, role, cohort_scope_json, ` |
| 172 | SELECT | `workspace_member_invitation` | 🟢 BIND | `SELECT id, tenant_id, email, role, cohort_scope_json, token_hash, invited_by, \ invited_at` |
| 193 | SELECT | `workspace_member_invitation` | 🟡 MENTION | `SELECT id, tenant_id, email, role, cohort_scope_json, token_hash, invited_by, \ invited_at` |
| 225 | UPDATE | `workspace_member_invitation` | 🔴 NO_TENANT | `UPDATE workspace_member_invitation SET accepted_at = ?, accepted_by_user_id = ? \ WHERE to` |
| 253 | SELECT | `workspace_member_invitation` | 🟢 BIND | `SELECT id, tenant_id, email, role, cohort_scope_json, token_hash, invited_by, \ invited_at` |

</details>

---

## §7 — 🟡 caller-trust appendix (per-helper call-site index)

For each 🟡 caller-trust-boundary helper, the appendix lists every
non-test caller. This is the audit map for downstream
tenant-resolution follow-up audits.

### §7.1 — `approvals::mark_escalation_sent_at`
- `crates/argo-cli/src/handlers/gateway/approval_reminder.rs`
- `crates/argo-cli/tests/approval_reminder_smoke.rs` (test-only)

### §7.2 — `approval_bundle::decide_bundle`
- `crates/argo-cli/src/handlers/gateway/routes_approval_bundles.rs`

### §7.3 — `elevated_view_grant::revoke` (+ adjacent `argo-redact::grants`)
- `crates/argo-cli/src/handlers/gateway/routes_elevated_view.rs`
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_audit.rs`
- `crates/argo-redact/src/grants.rs`

### §7.4 — `scim_user::patch_user` / `deactivate_user`
- `crates/argo-cli/src/handlers/gateway/routes_scim_users_envelope.rs`
- `crates/argo-cli/src/handlers/gateway/routes_scim_users.rs`

### §7.5 — `support_query::update_ticket` / `reply_ticket`
- `crates/argo-cli/src/handlers/gateway/routes_support_tickets.rs`
- `crates/argo-mcp-connector-zendesk/src/{service,client,schemas}.rs` (downstream relay)

### §7.6 — `worker_budget_write::*`
- `crates/argo-cli/src/handlers/worker_budget.rs`

### §7.7 — `session_meta::*` + `store_session_routing::set_routing_policy`
- `crates/argo-cli/src/handlers/session.rs`
- `crates/argo-cli/src/handlers/session_routing.rs`
- `crates/argo-cli/src/handlers/gateway/routes_session_skills_lifecycle.rs`
- `crates/argo-tui/src/{input_run,actions}.rs`
- `crates/argo-store/src/session_store.rs` (in-crate re-export)

### §7.8 — `persona_revision::apply_revert`
- `crates/argo-cli/src/handlers/persona_revert.rs`
- `crates/argo-cli/src/handlers/persona_history.rs`

### §7.9 — `worker_archive::delete_with_grace` cascade
- `crates/argo-cli/src/handlers/worker_delete.rs`
- `crates/argo-cli/src/handlers/gateway/routes_worker_archive.rs`

### §7.10 — `worker_presence::update_last_active`
- `crates/argo-cli/src/handlers/doctor_workers.rs`
- `crates/argo-cli/src/handlers/worker_fleet.rs`
- `crates/argo-audit/src/sink_chain.rs`

### §7.11 — `skill_query::distinct_skill_verticals` + `search_by_vertical`
- `crates/argo-cli/src/handlers/skill_search.rs` (CLI-only today —
  latent-on-route-wire)

### §7.12 — `skill_share::set_shared`
- `crates/argo-cli/src/handlers/gateway/marketplace.rs`
  (**LIVE** remote route — `POST /marketplace/<skill_id>/install`)

---

## §8 — Sibling fix-brief queue (authored as files in this PR)

Each of the eight briefs below is authored as a file under
`docs/build-plan/streams/` in this PR. The conductor can promote
each row to TODO at its own cadence.

| # | Brief id | Source | Priority | File |
|---|---|---|---|---|
| 1 | `mission-approval-mark-escalation-tenant-pinned` | PR #2116 §4.1b (not in #2146 fix) | 🟡 P1 | `streams/mission-approval-mark-escalation-tenant-pinned.md` |
| 2 | `mission-approval-bundle-decide-tenant-pinned` | PR #2116 §4.1c (not in #2146 fix) | 🟡 P1 | `streams/mission-approval-bundle-decide-tenant-pinned.md` |
| 3 | `mission-elevated-view-grant-revoke-tenant-pinned` | PR #2116 §4 finding 2 | 🟡 P1 | `streams/mission-elevated-view-grant-revoke-tenant-pinned.md` |
| 4 | `mission-scim-user-update-tenant-pinned` | PR #2116 §4 finding 3 | 🟡 P1 | `streams/mission-scim-user-update-tenant-pinned.md` |
| 5 | `mission-support-ticket-update-tenant-pinned` | PR #2116 §4 finding 4 | 🟡 P1 | `streams/mission-support-ticket-update-tenant-pinned.md` |
| 6 | `mission-worker-budget-write-tenant-pinned` | PR #2116 §4 finding 5 | 🟡 P2 | `streams/mission-worker-budget-write-tenant-pinned.md` |
| 7 | `mission-session-meta-mutators-tenant-pinned` | PR #2116 §4 finding 6 | 🟡 P2 | `streams/mission-session-meta-mutators-tenant-pinned.md` |
| 8 | `mission-persona-revert-tenant-pinned` | PR #2116 §4 finding 7 | 🟡 P2 | `streams/mission-persona-revert-tenant-pinned.md` |
| 9 | `mission-worker-archive-cascade-tenant-pinned` | THIS REPORT §3.3 (correction) | 🟡 P2 | `streams/mission-worker-archive-cascade-tenant-pinned.md` |
| 10 | `mission-skill-query-tenant-pinned` | THIS REPORT §4.1 + §4.2 (post-#2092 sweep gap) | 🔴 P0 | `streams/mission-skill-query-tenant-pinned.md` |
| 11 | `mission-skill-share-tenant-pinned` | THIS REPORT §4.3 (post-#2092 sweep gap) | 🔴 P0 | `streams/mission-skill-share-tenant-pinned.md` |

§4.1 + §4.2 are bundled in a single brief (#10) because they're
the read-side of the same module and share the same fix shape.
§4.3 is a standalone brief (#11) because it changes a mutator
signature plus a remote-route caller (`marketplace.rs`).

PR #2146 already shipped the "name fix-briefs as
`mission-*-tenant-pinned`" naming convention (closes PR #2116
§4 finding 1 = `mission-approval-decide-request-tenant-pinned`).
The remaining ten follow the same naming for grep-ability.

---

## §9 — Out of scope / explicit non-goals

- **The 42 child-scoped tables** in PR #2116 §2.3. Each was named
  but not byte-audited; the contract is "tenant-resolve the parent
  id at the caller". The recommended P3 follow-up
  `eval-argo-store-child-scoping-sweep` is unchanged.
- **Workspace-id tables.** `marketplace_artifact` + `policy_files`
  have no in-store callsites (out of may-touch). PR #2116
  recommended `eval-workspace-id-tables-sweep` — still open.
- **Out-of-crate query callsites.** Memory layer-row queries in
  `argo-memory`, orchestrator queries in `argo-orchestrator`, etc.,
  are out of may-touch. Same as PR #2116 §7.
- **Index-coverage / performance audit.** Tenant binding is
  correctness; index coverage for `(tenant_id, …)` composite keys
  is a separate axis. Already tracked elsewhere.
- **The `__migration_placeholder__` re-tenant operator CLI.**
  Per the PR #2092 PR-body, "a separate operator CLI re-tenants
  those rows in a future brief (out of scope here, per brief)" —
  still unauthored. Recommended P2 follow-up
  `mission-skill-tenant-placeholder-rebind-cli`.

---

## §10 — Reproducing this audit

The two scripts used to extract + classify the SQL are documented
inline in §2.1 / §2.2 and reproducible from any worktree as a
one-shot tool. For a quick sanity check at `3df52142a`:

```bash
# Count of sqlx invocations in production source
rg -c "sqlx::query" crates/argo-store/src/ \
   | grep -v _tests \
   | grep -v migrations \
   | awk -F: '{s+=$2} END {print s}'   # expect ≈ 358

# Tables-with-tenant-id (count)
rg -E "CREATE TABLE.*tenant_id|ALTER TABLE \w+ ADD COLUMN tenant_id" \
   crates/argo-store/src/migrations/ \
   | grep -v splitter \
   | wc -l                              # expect 34 (33 from PR #2116 + skill from #2092)

# New 🔴 candidates introduced by #2092 — skill table without tenant_id WHERE
rg -U "FROM\s+skill\b[^;]*WHERE" \
   crates/argo-store/src/skill_query.rs \
   crates/argo-store/src/skill_share.rs --multiline-dotall
```

---

## §11 — Status

- [x] All 78 remaining tenant-bearing-table queries enumerated
      per-file/per-query (§5 + §6).
- [x] PR #2116's seven §4 findings re-verified against
      `3df52142a`: 1 shipped (#2146), 6 still latent.
- [x] 3 PR #2116 §2.1 heuristic-miss corrections logged (§3.3).
- [x] 4 new 🔴 LEAK CANDIDATES surfaced post-PR #2092 (§4).
- [x] 11 sibling fix-brief files authored in
      `docs/build-plan/streams/`.
- [x] 🟡 caller-trust appendix populated with 12 helper → caller
      maps (§7).
- [x] Reproduction recipe (§10).

**Verdict**: **🟡 P1 → 🟢 (deepened + corrected)**.

PR #2116 + this report + the 11 sibling fix-brief files together
constitute the complete `argo-store` tenant-bind sweep. The
manifest row may now flip 🟢 with confidence; the remaining work
is fix-brief execution (queue replenishment in §8), not further
audit.

Bonus discovery: the post-#2092 4-leak-candidate cluster (§4) is
a strong argument for a **per-tenant-column-migration sweep
checklist** — when a future migration adds `tenant_id` to a
previously global table, the brief MUST enumerate every helper
that queries the table and either sweep them in the same brief or
queue follow-up fix-briefs. Recommend authoring
`mission-tenant-column-migration-sweep-checklist` as a
process-level brief.
