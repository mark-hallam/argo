# eval-argo-store-tenant-bind-sweep-remaining-tables

**Owner**: claude.
**Depends on**: nothing.
**Authored against**: origin/main `746b0e213` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-store-tenant-bind-sweep-remaining-tables`.

🟡 **P1 SUBSTRATE SWEEP — finish the argo-store table audit; 78 of 91 tables still un-spot-checked for cross-tenant SQL leaks.**

## Source

Evaluator round 13 (SHARED.md handoff 2026-05-23T12:30Z) — first 13 of 91 tables spot-checked, surfaced 2 P1 leaks (skill_runs worker_budget + spend_anomaly DISTINCT — both fixed via #2094 / #2095) and 1 P1 sweep (subtask_ledger via #2093). Round 14 confirmed argo-memory's 3 internal tables clean. The remaining 78 tables in argo-store still need the same spot-check pass.

Round-13 next-session call-out:

> Next session: memory_embedding/shared_team_memory/workflow_run/skill_ab_test_outcome/policy_files (data lives in other crates — different audit pattern from argo-store-direct queries).

That note covers a handful of tables in other crates. This brief is the SISTER sweep covering the remaining argo-store tables not yet audited.

## What's wrong (the pattern)

Spot-check found 2 P1 cross-tenant leaks in the first 13 of 91 tables. Pro-rated, 78 remaining tables likely contain 8-15 more leaks. Each is a confidentiality + sometimes integrity risk. The earlier round identified the leak class — this brief finishes the sweep.

## What this brief must do

1. **Enumerate** every `*.rs` file in `crates/argo-store/src/` that contains `SELECT … FROM <table>` or `UPDATE/DELETE FROM <table>` where the table is NOT yet audited (i.e. not skill_runs / worker_budget / spend_anomaly / subtask_ledger / approval_request / audit_record + the 13 covered in round 13).
2. **For each file**, categorize each query:
   - 🟢 **TENANT-BOUND** — query includes `WHERE … tenant_id = ?` (or equivalent like `workspace_id` which transitively scopes to tenant per round-9 finding).
   - 🟡 **CALLER-TRUST** — query keys on a primary-key UUID alone; tenant scoping is upstream caller's responsibility. Flag for caller-audit.
   - 🔴 **LEAK** — query has no tenant scoping AND aggregates / enumerates across tenants (GROUP BY, DISTINCT, JOIN without tenant predicate).
3. **Report** at `docs/skycot/evals/scheduled/2026-05-23/eval-argo-store-tenant-bind-sweep-remaining-tables.md`:
   - Per-file/query table: file:line, table, query verb, verdict, brief candidate name if 🔴.
   - Final summary: N 🟢, N 🟡, N 🔴.
4. **For each 🔴 LEAK**, author a sibling fix-brief named `claude-<table>-<query-shape>-tenant-bind`. Conductor will pick up Step-8.
5. **For each 🟡 CALLER-TRUST**, add to the report's appendix listing call-site files to audit.

## Acceptance

- Report exists with full per-file table.
- All 78 remaining tables touched (or report explains why a table was skipped — e.g. internal-only / no production caller).
- ≥1 sibling fix-brief per 🔴 (zero is fine if no leaks found).
- No production code changes in this brief — it is read-only investigation + brief authoring.

## Sizing

~200-400 LOC (report + brief stubs). 3-5 hours.

## Why P1

Cross-tenant SQL leaks are the #1 audit-derived finding class this week. The sweep is high-leverage — every 🔴 found prevents a potential P0 incident.
