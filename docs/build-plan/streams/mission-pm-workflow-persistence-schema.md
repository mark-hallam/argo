# Stream P1-5 — workflow definition persistence layer

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-workflow-persistence-schema`.
**Source**: skycot-pm-worker REPORT.md §3.3 + §11 P1 #5.
**Contract status**: this is a CONTRACT brief. P1 #6 (edit API), P1 #7 (rollback), P2 #9 (diff view), P2 #10 (conversational author) all build against this schema. **Get the shape right.**

## Scope

Workflow definitions today live as TOML files in `verticals/<id>/workflows/*.workflow.toml`, loaded once at session bootstrap and frozen for the session's lifetime (`crates/argo-vertical/src/loader.rs:35-80`). The only DB-side workflow primitive is `workflow_run` (execution history, migration 18). There is no `workflow_definition` table, no version chain, no edit ledger.

This brief lands the persistence layer. New tables:

1. `workflow_definition (id, tenant_id, vertical_id, name, definition_toml, version, parent_id, lifecycle_state, created_at, modified_by)` — one row per workflow version.
2. `workflow_definition_index (tenant_id, vertical_id, name, head_id)` — the "currently active" pointer per `(tenant, vertical, name)` triple.

And a CRUD module `argo-store/src/workflow_definition.rs` exposing:
- `create(tenant, vertical, name, toml, parent_id, modified_by) -> WorkflowDefinitionId`
- `get(id) -> Option<WorkflowDefinition>`
- `get_head(tenant, vertical, name) -> Option<WorkflowDefinition>`
- `list_versions(tenant, vertical, name) -> Vec<WorkflowDefinition>`
- `set_head(tenant, vertical, name, id)`
- `archive(id)` (lifecycle_state → Archived)

**This brief ships schema + CRUD only.** The edit-API surface (P1 #6) and CLI (P1 #7) come next. The loader change to consult the DB when bootstrapping is out of scope here — files remain authoritative for v1; the DB layer is additive until the edit API lands.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-store/migrations/<next-slot>__workflow_definition.sql` (NEW) — schema. Two tables + indexes on `(tenant_id, vertical_id, name)` and `(parent_id)`. Verify slot with `scripts/check-migration-slots.sh`.
- `crates/argo-store/src/migrations/v<next>.rs` (NEW) — versioned wrapper.
- `crates/argo-store/src/workflow_definition.rs` (NEW) — CRUD module. ≤ 500 LOC.
- `crates/argo-store/src/workflow_definition_tests.rs` (NEW sibling) — unit + integration coverage.
- `crates/argo-store/src/lib.rs` — narrowly scoped: add `pub mod workflow_definition;`. No other edits.

## Files you must NOT touch

- `crates/argo-vertical/src/loader.rs` — loader continues to read files; DB read-back is a follow-up brief.
- `crates/argo-vertical/src/workflow.rs` — parser is unchanged; only how we persist the parsed result.
- Any handler in `crates/argo-cli/src/handlers/` — CLI surface is the next brief.
- Anything in `web/`.

## Acceptance criteria

- [ ] Two new tables created via additive migration; idempotent (`CREATE TABLE IF NOT EXISTS`); registered in `crates/argo-store/src/migrations/registry.rs`.
- [ ] `workflow_definition` columns: `id TEXT PRIMARY KEY`, `tenant_id TEXT NOT NULL`, `vertical_id TEXT NOT NULL`, `name TEXT NOT NULL`, `definition_toml TEXT NOT NULL`, `version INTEGER NOT NULL`, `parent_id TEXT`, `lifecycle_state TEXT NOT NULL DEFAULT 'draft'`, `created_at TEXT NOT NULL`, `modified_by TEXT NOT NULL`.
- [ ] `lifecycle_state` accepts `draft`, `active`, `archived`. Validated at the CRUD layer (no CHECK constraint — keep migration additive).
- [ ] Indexes: `(tenant_id, vertical_id, name)` and `(parent_id)`.
- [ ] CRUD API matches the 6 functions listed in scope; each parses the TOML via `argo_vertical::workflow::parse_workflow` and rejects malformed input.
- [ ] `set_head` is transactional: updates the `workflow_definition_index` row atomically; failure on a non-existent definition_id returns `ArgoError::NotFound`.
- [ ] Tests: 10 cases — (1) create + get round-trip; (2) version increments per `(tenant, vertical, name)`; (3) parent_id chain; (4) list_versions returns newest-first; (5) get_head returns None when no head set; (6) set_head + get_head; (7) archive flips lifecycle_state and clears head pointer if it was head; (8) tenant isolation (tenant A cannot read tenant B's rows); (9) duplicate `(tenant, vertical, name, version)` is rejected; (10) migration idempotency (run twice — no error).
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-store` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-workflow-persistence-schema`
- PR title: `store: workflow_definition + workflow_definition_index tables + CRUD (mission-pm-workflow-persistence-schema)`

## Notes for the implementer

- **This is a CONTRACT brief.** Downstream briefs (edit API, rollback CLI, diff view, conversational author) all depend on the column shape. Do not rename `lifecycle_state` → `state`, do not collapse `created_at` + `modified_by` into a single audit subquery, do not split `definition_toml` into per-step rows. Keep the shape as specified.
- `version` is a monotonic INTEGER per `(tenant, vertical, name)`. The CRUD layer computes the next version by `SELECT MAX(version) + 1 FROM workflow_definition WHERE tenant_id=? AND vertical_id=? AND name=?`.
- `definition_toml` stores the full TOML body. This is a workflow definition file, not a structured graph — keep it human-readable, version-controllable, and copy-paste portable.
- `modified_by` is a `Principal`-serialised string (use `principal.to_canonical()` if it exists; otherwise `format!("{:?}", principal)`).
- TOFU (trust-on-first-use) is not in scope for this brief. Tenant ownership is determined by the `tenant_id` of the calling principal at create-time.
- No gateway routes, no CLI command, no auth check in this brief — those are P1 #6.

## Out of scope (follow-up briefs)

- Gateway routes (`POST/PUT/DELETE /verticals/<id>/workflows/<name>`) — P1 #6.
- CLI commands (`argo workflow rollback`, `argo workflow diff`) — P1 #7 + P2 #9.
- Loader change to prefer DB over filesystem — separate follow-up.
- Cross-tenant workflow publishing (marketplace) — P3 #15.
