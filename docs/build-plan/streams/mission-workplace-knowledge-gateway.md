# mission-workplace-knowledge-gateway — 6 RBAC-gated gateway endpoints for the wiki

**Owner**: claude.
**Depends on**: `mission-workplace-knowledge-substrate` MERGED (provides the CRUD layer this brief wraps) + `mission-workplace-knowledge-audit-kinds` MERGED (provides the AuditKind variants this brief emits).
**Authored against**: origin/main `c613d922` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-workplace-knowledge-gateway`.
**Priority**: P1 — Wave A brief 3 of 6 from `docs/skycot/design/workplace-onboarding-architecture.md`.
**Sizing**: ~800 LOC across new routes module + handlers + tests + dispatch wire-up.

## Why this brief exists

Read design doc §2 (Pillar 1), §5 brief 3 (the spec), and §7 Pillar 1 acceptance.

The wiki substrate (brief 1) exists in `argo-workplace-knowledge`. The AuditKind variants (brief 2) exist in `argo-audit`. This brief connects them to the world by adding 6 HTTP endpoints, gated by the RBAC layer that landed in #1901 (`role_gate`).

Without this brief, the Next.js editor (brief 4) has nothing to hit. With this brief, the wiki has a real read/write surface.

## The 6 endpoints

| Method | Path | RBAC | Behavior |
|--------|------|------|----------|
| `GET` | `/api/v1/knowledge` | all roles read | List head docs for the caller's tenant. Optional `?category=brand` filter. |
| `GET` | `/api/v1/knowledge/<slug>` | all roles read | Fetch head doc + current version. 404 if slug not found in tenant. |
| `POST` | `/api/v1/knowledge` | owner + admin write | Create new doc. Body: `{ slug, title, category, body_markdown }`. 409 if slug already exists. Emits `WorkplaceKnowledgeDocCreated`. |
| `PUT` | `/api/v1/knowledge/<slug>` | owner + admin write | Update doc body, increments version, writes revision row. Body: `{ body_markdown, edit_reason? }`. Emits `WorkplaceKnowledgeDocUpdated`. |
| `DELETE` | `/api/v1/knowledge/<slug>` | owner only | Soft delete (head row removed; revisions retained). Emits `WorkplaceKnowledgeDocDeleted`. |
| `GET` | `/api/v1/knowledge/<slug>/revisions` | all roles read | Version history for a doc. Returns array sorted version DESC. |

All cross-tenant access returns **404** (not 403 — don't leak existence).

## Approach (concrete)

### Phase 1 — Route module

1. **NEW** `crates/argo-cli/src/handlers/gateway/routes_knowledge.rs` (~280 LOC) — the routes module mirroring the pattern of `routes_workflows.rs` (read existing for the exact shape):
   - `pub(super) async fn dispatch_get_api(req: &str, query: &str, store: &SessionStore) -> Option<(&'static str, String)>`
   - `pub(super) async fn dispatch_post(req: &str, body: &str, store: &SessionStore) -> Option<(&'static str, String)>`
   - `pub(super) async fn dispatch_put(req: &str, body: &str, store: &SessionStore) -> Option<(&'static str, String)>`
   - `pub(super) async fn dispatch_delete(req: &str, store: &SessionStore) -> Option<(&'static str, String)>`

   Each `dispatch_*` parses the route, derives `(principal, tenant)` via `authenticate_request` (the helper from `routes_workflows.rs` that landed in #1865), gates via `role_gate::enforce_role(req, Action::KnowledgeWrite)` for write paths, calls into `argo_workplace_knowledge::doc::{create, update, delete, get, list}` or `revision::list_for_doc`, emits the audit row, and returns the JSON.

2. **NEW** `crates/argo-cli/src/handlers/gateway/routes_knowledge_handlers.rs` (~200 LOC) — handler functions that the 4 dispatchers call:
   - `async fn handle_list(...) -> (status, json_body)`
   - `async fn handle_get(...)`
   - `async fn handle_create(...)`
   - `async fn handle_update(...)`
   - `async fn handle_delete(...)`
   - `async fn handle_revisions(...)`

   The split keeps each file under 500 LOC. Cross-tenant guard lives at the handler entry (return 404 if `argo_workplace_knowledge::doc::get(tenant, slug)` returns None).

3. **NEW** `crates/argo-cli/src/handlers/gateway/routes_knowledge_tests.rs` (~250 LOC) — 14+ integration tests:
   - `list_returns_empty_for_fresh_tenant`
   - `create_then_list_returns_doc`
   - `create_then_get_returns_doc`
   - `create_with_duplicate_slug_returns_409`
   - `update_increments_version_and_emits_audit_row`
   - `update_with_wrong_tenant_returns_404`
   - `delete_emits_audit_row_and_subsequent_get_returns_404`
   - `delete_with_user_role_returns_403` (only owner+admin can delete)
   - `delete_with_admin_role_succeeds` (per design doc §5 brief 3: "DELETE → soft-delete (owner only)" — adjust if operator confirms admin also allowed; default is owner-only per design)
   - `create_with_guest_role_returns_403`
   - `revisions_endpoint_returns_three_after_three_edits`
   - `cross_tenant_get_returns_404_not_403`
   - `category_filter_returns_only_matching_docs`
   - `audit_row_payload_includes_diff_on_update` (links to design doc §7 Pillar 1 acceptance "diff in payload")

### Phase 2 — Dispatch wire-up

4. **MODIFY** `crates/argo-cli/src/handlers/gateway/mod.rs` — register the 3 new modules:
   ```rust
   mod routes_knowledge;
   mod routes_knowledge_handlers;
   #[cfg(test)]
   #[path = "routes_knowledge_tests.rs"]
   mod routes_knowledge_tests;
   ```

5. **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api.rs` — add a match arm for `/api/v1/knowledge` (and `/api/v1/knowledge/<slug>` + `/api/v1/knowledge/<slug>/revisions`) that calls `routes_knowledge::dispatch_get_api`. Mirror the existing pattern (the file routes by URL prefix).

6. **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs` — match arm for `POST /api/v1/knowledge` → `routes_knowledge::dispatch_post`.

7. **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_dispatch_put.rs` — match arm for `PUT /api/v1/knowledge/<slug>` → `routes_knowledge::dispatch_put`. (Or `routes_dispatch_patch.rs` if PUT is dispatched alongside PATCH — check the on-disk file layout at claim time.)

8. **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_dispatch_delete.rs` (or wherever DELETE dispatches — verify with `grep -n 'DELETE' crates/argo-cli/src/handlers/gateway/routes_dispatch*.rs`) — match arm for DELETE.

### Phase 3 — RBAC capability

9. **MODIFY** `crates/argo-cli/src/handlers/gateway/role_gate.rs` — add 2 new `Action` enum variants:
   ```rust
   KnowledgeRead,   // all roles
   KnowledgeWrite,  // owner + admin
   KnowledgeDelete, // owner only (per design doc §5 brief 3)
   ```
   Plus the corresponding capability-map entries (the file has a `pub fn role_for_action(action: Action) -> Role` style table; mirror that).

   Note: design doc §5 brief 3 says "owner+admin write" for create/update but "owner only" for delete. Encode as 3 separate Actions if cleaner, or use `KnowledgeWrite` for create/update and a `KnowledgeDelete` separately. The 3-Action split is cleanest.

10. **MODIFY** `crates/argo-cli/Cargo.toml` — add `argo-workplace-knowledge = { path = "../argo-workplace-knowledge" }` to `[dependencies]`.

### Phase 4 — Audit emission

11. Inside each mutating handler in `routes_knowledge_handlers.rs`, after the store call succeeds, emit an audit row using `argo_audit::AuditSink` (the existing pattern in adjacent route modules):
    ```rust
    use argo_audit::{AuditKind, AuditSink};
    use serde_json::json;

    AuditSink::new_with_tenant(Arc::clone(store), tenant_id.to_string())
        .write(
            Some(principal_id),
            AuditKind::WorkplaceKnowledgeDocUpdated,
            json!({
                "slug": slug,
                "version": new_doc.version,
                "diff_summary": short_diff,  // first 200 chars of unified-diff
            }),
            None,
        )
        .await?;
    ```
    For `WorkplaceKnowledgeDocUpdated`, the payload MUST include a `diff` field per the design doc §7 acceptance ("diff in payload"). Use a tiny similar-line diff (first 500 chars is fine; the full body is in `workplace_knowledge_revision.body_markdown` for the heavy view).

## Files you may touch (verified to exist on `c613d922`)

- **NEW** `crates/argo-cli/src/handlers/gateway/routes_knowledge.rs`.
- **NEW** `crates/argo-cli/src/handlers/gateway/routes_knowledge_handlers.rs`.
- **NEW** `crates/argo-cli/src/handlers/gateway/routes_knowledge_tests.rs`.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/mod.rs` — register modules.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api.rs`.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs`.
- **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_dispatch_put.rs` (or wherever PUT lives — verify).
- **MODIFY** `crates/argo-cli/src/handlers/gateway/routes_dispatch_delete.rs` (or equivalent).
- **MODIFY** `crates/argo-cli/src/handlers/gateway/role_gate.rs` — 3 new Action variants + capability map entries.
- **MODIFY** `crates/argo-cli/Cargo.toml` — add `argo-workplace-knowledge` dep.

## Files you must NOT touch

- `crates/argo-workplace-knowledge/*` — substrate is brief 1's domain. If you find that the CRUD API needs an extension (e.g. category filter at SQL level), STOP, write a sibling brief, do not modify in this PR.
- `crates/argo-audit/src/event.rs` — variants are brief 2's domain. They must already exist when you claim this brief; if they don't, do not modify — wait for brief 2 to merge.
- `crates/argo-store/migrations/*` — no migration here.
- Anything under `web/` — frontend editor is brief 4's domain.
- `crates/argo-cli/src/handlers/gateway/role_gate_tests.rs` — extend the test if needed, but do NOT change existing assertions.

## Pre-flight verification

```bash
# Confirm both deps have merged.
ls crates/argo-workplace-knowledge/src/lib.rs
grep -n 'WorkplaceKnowledgeDocCreated' crates/argo-audit/src/event.rs
# Both must succeed before claiming. Otherwise wait.

# Reference patterns to clone from.
ls crates/argo-cli/src/handlers/gateway/routes_workflows.rs            # pattern for dispatch shape
ls crates/argo-cli/src/handlers/gateway/routes_notifications.rs        # pattern for handler split (helpers file)
ls crates/argo-cli/src/handlers/gateway/role_gate.rs                   # add KnowledgeRead/Write/Delete Actions
ls crates/argo-cli/src/handlers/gateway/routes_dispatch_get_api.rs     # add URL match arms
ls crates/argo-cli/src/handlers/gateway/routes_dispatch_post.rs

# Confirm authenticate_request helper is reachable from a new routes module.
rg 'pub\(super\) (async )?fn authenticate_request' crates/argo-cli/src/handlers/gateway/

# Confirm AuditSink::new_with_tenant pattern is the canonical emit pattern.
rg 'AuditSink::new_with_tenant' crates/argo-cli/src/handlers/gateway/ | head -3
```

## Acceptance criteria

- [ ] All 6 endpoints implemented + dispatched (see the table above).
- [ ] RBAC matrix matches design doc §5 brief 3: read=all roles, write=owner+admin, delete=owner only.
- [ ] Cross-tenant access returns 404 (not 403). Negative test enforces.
- [ ] Audit row emitted on every successful Create/Update/Delete; payload includes `diff` on Update.
- [ ] `cargo test -p argo-cli routes_knowledge` green (≥ 14 tests).
- [ ] `cargo clippy -p argo-cli -p argo-workplace-knowledge -p argo-audit --all-targets -- -D warnings` clean.
- [ ] `bash scripts/conductor-pre-merge-gate.sh` PASS — security-sensitive (RBAC + audit) Tier-3 review applies.
- [ ] Every modified file < 500 LOC. (The handler split into `routes_knowledge_handlers.rs` keeps each route file lean.)
- [ ] No `unwrap` / `expect` outside `#[cfg(test)]`.
- [ ] No new `pub struct` / `pub enum` / `pub type` that collides with an existing name. Verify with `rg 'pub (struct|enum|type) Knowledge' crates/`.
- [ ] BUILD_STATUS.md row flipped to IN_REVIEW with PR number on push.

## Branch + PR

- Branch: `claude-mission-workplace-knowledge-gateway`
- PR title: `argo-cli(gateway): 6 RBAC-gated endpoints for tenant-scoped wiki (mission-workplace-knowledge-gateway)`
- PR body must call out: this is **security-sensitive code** (RBAC + audit emission). Conductor will Tier-3 review (workspace clippy + diff-shape).

## Gotchas

- **Slug uniqueness is tenant-scoped**: the `UNIQUE(tenant_id, slug)` constraint is on the table. Two tenants can BOTH have a doc called `brand-voice`. Tests must cover the cross-tenant slug-collision case (no 409 — independent slugs).
- **DELETE is soft**: head row removed, revisions retained. So later, `GET /<slug>/revisions` for a deleted doc still returns the history. Add a test for this.
- **PUT vs PATCH semantics**: brief uses PUT, which conventionally means "replace the whole resource". The body is just `{ body_markdown, edit_reason? }` — that's a partial update of the head doc. Either:
  - (a) Strictly PUT — the body must contain `{ slug, title, category, body_markdown, edit_reason? }` and replaces all fields.
  - (b) Allow PATCH semantics under PUT — accept partial body, only update `body_markdown` if present.

  Recommend (b) for editor ergonomics — but pick one, declare in PR body, and stick with it. The existing pattern in `routes_workflows.rs` allows partial body, so mirror that.
- **Diff in audit payload**: keep it small. A 500-char unified diff in the audit row is fine; the full `body_markdown` lives in `workplace_knowledge_revision` for diff-viewer rendering. Don't blow up audit row sizes.
- **`role_gate` extension may break existing tests**: when adding `Action::KnowledgeRead/Write/Delete`, ensure exhaustive `match` in `enforce_role` covers them. Clippy `-D warnings` catches non-exhaustive matches if there are any wildcard arms.
- **Audit-chain stability**: emitting NEW audit-kind strings is fine; the chain hashes the string label, so as long as variant labels are STABLE (don't rename a kind after merge), the chain stays intact.

## References

- Design doc: `docs/skycot/design/workplace-onboarding-architecture.md` §2, §5 brief 3, §7 Pillar 1.
- Reference patterns:
  - `routes_workflows.rs` — dispatch shape.
  - `routes_notifications.rs` (from #1906) — handler-split pattern for keeping files under 500 LOC.
  - `routes_audit_export.rs` (from #1913) — security-sensitive route module with audit emission.
  - `role_gate.rs` (from #1901) — Action enum + capability map.
- Sibling briefs in Wave A:
  - **brief 1** (`mission-workplace-knowledge-substrate`) — provides `argo_workplace_knowledge::doc::{create, update, delete, get, list}` + revision module.
  - **brief 2** (`mission-workplace-knowledge-audit-kinds`) — provides the 3 AuditKind variants.
  - **brief 4** (`web-workplace-knowledge-editor`) — consumes this gateway's endpoints.
