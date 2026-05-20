# web-workflow-rollback-feature — ship the workflow rollback feature

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-workflow-rollback-feature`.
**Priority**: P1 — wave-5 close-the-loop found "One-tap rollback" was a broken promise on `/landing` + `/pricing`. Wave-5 fix softened landing copy to "Rollback shipping next" — this brief delivers the feature so we can promote the claim back.
**Sizing**: ~900-1100 LOC across argo-store (`rollback_to`) + argo-audit (1 new `AuditKind` variant) + argo-cli gateway (1 new route handler) + web (modal + viewer-page wiring + API client) + tests (8 unit + 1 E2E).

---

## Why this brief exists

From wave-5 close-the-loop audit (`docs/skycot/audit/close-the-loop/2026-05-20-wave-5-close-the-loop.md`):

> **"One-tap rollback"** — MISSING entirely. DB substrate exists in
> `argo-store/src/workflow_definition.rs` ("upcoming") but no API,
> no UI, no route. Promised on both /landing and /pricing.

The DB substrate exists; the feature is undelivered. Once this ships,
the marketing copy ("Rollback shipping next") can revert to "One-tap rollback."

This is also a **mission-claim-4** dependency: "every state transition is in
the audit chain" — without `AuditKind::WorkflowRolledBack`, rollback events
emit indistinguishably as `WorkflowHeadChanged`, and retroactive labeling of
rollback events vs head-promotion events becomes impossible.

---

## Pre-flight verification (run before authoring code)

```bash
# 1. Confirm the substrate is still where the brief expects.
rg "pub async fn (create|set_head|archive)" crates/argo-store/src/workflow_definition_write.rs
rg "WorkflowDefinitionCreated|WorkflowHeadChanged|WorkflowVersionArchived" crates/argo-audit/src/event.rs

# 2. Confirm no parallel "rollback" surface has been authored elsewhere.
rg "rollback_to|RolledBack|workflow_rolled_back" crates/

# 3. Confirm landing copy is still the softened version (so the revert step lands cleanly).
rg "Rollback shipping next" web/app/landing/

# 4. Confirm migration slot is free (currently v76 / SQL 125 are the highest).
ls crates/argo-store/src/migrations/ | sort -V | tail -3
ls crates/argo-store/migrations/ | sort -V | tail -3
```

If any of these greps return surprising hits, stop and reconcile with the
conductor — the brief may be stale.

---

## End-to-end flow

```
Viewer page lists versions ──▶  User clicks "Rollback to v3" on a prior row
                                       │
                                       ▼
                        <RollbackConfirmModal> shows diff (v3 vs current)
                                       │
                                       ▼
                  POST /api/v1/verticals/{vid}/workflows/{name}/rollback
                  Body: { target_version_id: "<wd_id_of_v3>" }
                                       │
                                       ▼  (in gateway handler, one DB tx)
                  1. Probe target row (cross-tenant guard → 404 if mismatch).
                  2. Call wd::rollback_to() which:
                       a. INSERT new row with target's TOML, parent_id =
                          target.id, lifecycle_state = Draft, next version.
                       b. Promote new row to head (re-uses set_head path —
                          flips to Active + writes to workflow_definition_index).
                  3. Emit AuditKind::WorkflowRolledBack with payload
                       { actor, tenant_id, vertical_id, name,
                         rolled_back_from_version_id, new_version_id,
                         new_version_number }.
                                       │
                                       ▼
                  200 OK { id, version }  ──▶  Web shows success toast,
                                                 reloads viewer at new head.
                                       │
                                       ▼
                  Audit feed (/audit) shows row with kind "workflow_rolled_back"
                  — distinguishable from a plain head-promote.
```

---

## Phase 1 — argo-audit: add `WorkflowRolledBack` variant

**File**: `crates/argo-audit/src/event.rs`

Add one variant alongside the existing 3 Workflow* kinds (look for
`WorkflowDefinitionCreated`, `WorkflowHeadChanged`, `WorkflowVersionArchived`).
Pattern is established — copy the docstring shape exactly.

```rust
/// A workflow-definition version was reverted-to via
/// `POST /api/v1/verticals/{vertical_id}/workflows/{name}/rollback`
/// (`web-workflow-rollback-feature`). Rollback is implemented as an
/// additive new version copied from a prior one — the new row's
/// `parent_id` is the rolled-back-from row's id. Payload carries
/// `actor`, `tenant_id`, `vertical_id`, `name`,
/// `rolled_back_from_version_id`, `new_version_id`, and
/// `new_version_number`. Pairs with [`Self::WorkflowHeadChanged`]
/// (the head-flip half of the rollback transaction) but is
/// distinguishable so audit consumers can filter rollback vs ordinary
/// head-promote events.
WorkflowRolledBack,
```

Then update three sibling tables in the same file:

1. `label()` match arm — add `Self::WorkflowRolledBack => "workflow_rolled_back"`.
2. `from_label()` match arm — add `"workflow_rolled_back" => Self::WorkflowRolledBack`.
3. The `pub const ALL: &[Self]` array (test exhaustiveness check) — add
   `Self::WorkflowRolledBack` alongside the other Workflow* entries.

**No migration needed** — `AuditKind` is serialised by label only; SQLite stores
the string and the enum is read-only from existing audit rows.

---

## Phase 2 — argo-store: add `rollback_to` op

**File**: `crates/argo-store/src/workflow_definition_write.rs` (add after `archive`)

`rollback_to` is a composite write that wraps `create` + `set_head` inside one
transaction so a crash mid-way cannot leave the head pointing at a draft.

```rust
/// Rollback `(tenant, vertical, name)` to a prior version. Implemented
/// as an ADDITIVE write — produces a new version row whose TOML body
/// is byte-identical to the target row, then promotes the new row to
/// head. The original target row is left untouched (no history
/// rewrite). The new row's `parent_id` is the target row's id, so the
/// version chain reads naturally as "v5 was branched from v3."
///
/// # Errors
///
/// - [`ArgoError::NotFound`] — `target_id` does not exist, OR it
///   exists but belongs to a different `(tenant, vertical, name)`
///   triple (cross-tenant guard).
/// - [`ArgoError::Invariant`] — the target row is already head
///   (rollback to current head is a no-op; callers should not invoke
///   this — guard on the gateway side too for a friendlier 4xx).
/// - [`ArgoError::Provider`] — sqlite errors.
///
/// Returns the new row's id + version number so the gateway can emit
/// them on the audit row.
pub async fn rollback_to(
    pool: &Pool<Sqlite>,
    tenant_id: TenantId,
    vertical_id: &str,
    name: &str,
    target_id: &WorkflowDefinitionId,
    modified_by: &str,
) -> ArgoResult<(WorkflowDefinitionId, i64)> {
    let tenant_str = tenant_id.to_string();
    let mut tx = pool.begin().await
        .map_err(|e| ArgoError::Provider(format!("rollback_to tx: {e}")))?;

    // 1. Probe target row + cross-tenant guard.
    let row = sqlx::query(
        "SELECT tenant_id, vertical_id, name, definition_toml \
         FROM workflow_definition WHERE id = ?",
    )
    .bind(target_id.as_str())
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to probe: {e}")))?;
    let row = row.ok_or_else(|| ArgoError::NotFound(format!(
        "workflow_definition: target id '{}' does not exist", target_id.0
    )))?;
    let row_tenant: String = row.try_get("tenant_id")
        .map_err(|e| ArgoError::Provider(format!("rollback_to decode tenant: {e}")))?;
    let row_vertical: String = row.try_get("vertical_id")
        .map_err(|e| ArgoError::Provider(format!("rollback_to decode vertical: {e}")))?;
    let row_name: String = row.try_get("name")
        .map_err(|e| ArgoError::Provider(format!("rollback_to decode name: {e}")))?;
    let target_toml: String = row.try_get("definition_toml")
        .map_err(|e| ArgoError::Provider(format!("rollback_to decode toml: {e}")))?;
    if row_tenant != tenant_str || row_vertical != vertical_id || row_name != name {
        return Err(ArgoError::NotFound(format!(
            "workflow_definition: target id does not match ({tenant_str}, {vertical_id}, {name})"
        )));
    }

    // 2. Guard: rollback to current head is a no-op (return error so caller
    //    surfaces a friendlier 409/400).
    let current_head: Option<String> = sqlx::query_scalar(
        "SELECT head_id FROM workflow_definition_index \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ?",
    )
    .bind(&tenant_str).bind(vertical_id).bind(name)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to head probe: {e}")))?;
    if current_head.as_deref() == Some(target_id.as_str()) {
        return Err(ArgoError::Invariant(format!(
            "rollback_to: target id is already current head"
        )));
    }

    // 3. Compute next version + insert new row.
    let next_version: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(version), 0) + 1 FROM workflow_definition \
         WHERE tenant_id = ? AND vertical_id = ? AND name = ?",
    )
    .bind(&tenant_str).bind(vertical_id).bind(name)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to next-version: {e}")))?;

    let new_id = WorkflowDefinitionId::new();
    let created_at = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO workflow_definition \
            (id, tenant_id, vertical_id, name, definition_toml, version, \
             parent_id, lifecycle_state, created_at, modified_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id.as_str())
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .bind(&target_toml)
    .bind(next_version)
    .bind(target_id.as_str())        // parent_id = the rolled-back-from row
    .bind(WorkflowLifecycleState::Active.as_str())  // straight to Active
    .bind(&created_at)
    .bind(modified_by)
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to insert: {e}")))?;

    // 4. Flip head pointer to the new row.
    sqlx::query(
        "INSERT INTO workflow_definition_index (tenant_id, vertical_id, name, head_id) \
         VALUES (?, ?, ?, ?) \
         ON CONFLICT(tenant_id, vertical_id, name) DO UPDATE SET head_id = excluded.head_id",
    )
    .bind(&tenant_str)
    .bind(vertical_id)
    .bind(name)
    .bind(new_id.as_str())
    .execute(&mut *tx)
    .await
    .map_err(|e| ArgoError::Provider(format!("rollback_to head flip: {e}")))?;

    tx.commit().await
        .map_err(|e| ArgoError::Provider(format!("rollback_to commit: {e}")))?;
    Ok((new_id, next_version))
}
```

**Re-export from parent**: edit `crates/argo-store/src/workflow_definition.rs`
to add `rollback_to` to the `pub use workflow_definition_write::{...}` line.

**LOC budget check**: `workflow_definition_write.rs` is currently ~200 LOC.
Adding `rollback_to` (~80 LOC) keeps it under the 500-LOC ceiling, but if
you hit the budget split into `workflow_definition_rollback.rs` per the
established `_read` / `_write` split pattern.

---

## Phase 3 — argo-cli gateway: rollback route + handler

### 3a. Add route variant

**File**: `crates/argo-cli/src/handlers/gateway/routes_workflows.rs`

In the `Route<'_>` enum, add a new variant:

```rust
Rollback { vertical: &'a str, name: &'a str },
```

Update `parse_route` to recognise:

```
POST /api/v1/verticals/{vertical_id}/workflows/{name}/rollback
```

Mirror the existing `Head { vertical, name }` parse arm — same shape minus the
trailing `/head` (here it's `/rollback`).

### 3b. Add handler

**File**: `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs`

Add after the existing mutating handlers (`promote_head`, `archive_version`):

```rust
pub(super) async fn rollback(
    store: &SessionStore,
    tenant: TenantId,
    vertical_id: &str,
    name: &str,
    body: &str,
    principal: &Principal,
    sink: &dyn AuditSink,
) -> (&'static str, String) {
    // Parse body { target_version_id: "<wd_id>" }.
    let target_id_str = match serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.get("target_version_id").and_then(|t| t.as_str()).map(str::to_string))
    {
        Some(s) if !s.is_empty() => s,
        _ => return super::routes_workflows::bad_request(
            "rollback: body must be { \"target_version_id\": \"<id>\" }"
        ),
    };
    let target_id = WorkflowDefinitionId(target_id_str.clone());

    // Call store op.
    match wd::rollback_to(
        store.pool(),
        tenant.clone(),
        vertical_id,
        name,
        &target_id,
        &principal.to_string(),
    ).await {
        Ok((new_id, new_version)) => {
            // Emit audit row.
            let payload = json!({
                "actor": principal.to_string(),
                "tenant_id": tenant.to_string(),
                "vertical_id": vertical_id,
                "name": name,
                "rolled_back_from_version_id": target_id_str,
                "new_version_id": new_id.as_str(),
                "new_version_number": new_version,
            });
            // sink.emit() pattern matches the other Workflow* handlers exactly.
            if let Err(e) = sink.emit(AuditKind::WorkflowRolledBack, payload).await {
                tracing::warn!("audit emit WorkflowRolledBack failed: {e}");
            }
            let body = json!({ "id": new_id.as_str(), "version": new_version });
            ("200 OK", body.to_string())
        }
        Err(ArgoError::NotFound(msg)) =>
            super::routes_workflows::not_found(&msg),
        Err(ArgoError::Invariant(msg)) =>
            super::routes_workflows::bad_request(&msg),
        Err(e) =>
            super::routes_workflows::internal(&format!("rollback_to: {e}")),
    }
}
```

### 3c. Capability + dispatch wire-up

**File**: `crates/argo-cli/src/handlers/gateway/routes_workflows.rs`

In `handle_mutate`, add a Rollback arm:

```rust
Route::Rollback { vertical, name } => {
    if let Err(resp) = check_capability(&principal, role, &workflow_edit_capability()) {
        return resp;
    }
    let tenant = match resolve_tenant(store).await { Ok(t) => t, Err(r) => return r };
    let body = extract_body(req);
    hh::rollback(store, tenant, vertical, name, body, &principal, store.audit_sink()).await
}
```

(`workflow_edit_capability()` already exists for create/promote/archive — reuse,
do not add a new capability.)

---

## Phase 4 — web: API client + viewer wiring + confirm modal

### 4a. API client

**File**: `web/lib/api/workflow-rollback.ts` (new)

```ts
import { gatewayFetch } from "./client";

export interface RollbackResponse {
  id: string;
  version: number;
}

export async function rollbackWorkflow(args: {
  verticalId: string;
  name: string;
  targetVersionId: string;
}): Promise<RollbackResponse> {
  const res = await gatewayFetch(
    `/api/v1/verticals/${encodeURIComponent(args.verticalId)}/workflows/${encodeURIComponent(args.name)}/rollback`,
    {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ target_version_id: args.targetVersionId }),
    },
  );
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`Rollback failed (${res.status}): ${text}`);
  }
  return res.json();
}
```

### 4b. Confirm modal

**File**: `web/components/skycot/workflows/RollbackConfirmModal.tsx` (new)

A11y requirements:
- Focus trapped inside modal (use existing pattern from `<ConfirmDeleteModal>` if
  present — `rg "ConfirmDeleteModal\|focus-trap\|FocusLock" web/components/`).
- `Esc` closes.
- `Enter` confirms (only when "Confirm rollback" button has focus — don't auto-
  confirm on form-control focus).
- `<dialog>` element with `aria-labelledby` + `aria-describedby`.
- Body must include a **diff view** showing TOML keys changed between current
  head and target version. If the diff view component doesn't exist, the
  brief allows a simple **side-by-side `<pre>` block** in v1 — versioning is
  the primary signal, polished diff comes in a follow-up.

Props:
```tsx
interface Props {
  open: boolean;
  onClose: () => void;
  onConfirm: () => Promise<void>;
  currentVersion: number;
  targetVersion: number;
  currentToml: string;
  targetToml: string;
}
```

### 4c. Viewer page wiring

**File**: `web/app/(app)/workflows/[verticalId]/[name]/page.tsx`

In the version-history table, every row that is NOT the current head gets a
"Rollback to v\<N>" button. Click → opens `<RollbackConfirmModal>`. Confirm →
calls `rollbackWorkflow(...)` → toast on success → `router.refresh()` so the
version list reloads and the new head is at the top.

Error path: surface error message in the modal (red banner inside the modal,
keep modal open so user can retry or cancel).

---

## Phase 5 — Tests

### 5a. argo-store unit tests (new in `workflow_definition_tests.rs`)

1. `rollback_to_creates_new_version_with_target_toml`
2. `rollback_to_sets_parent_id_to_target`
3. `rollback_to_flips_head_to_new_row`
4. `rollback_to_cross_tenant_returns_not_found` (the negative test —
   create v1 under tenant A, attempt rollback under tenant B → expect
   `ArgoError::NotFound`, NOT `Invariant`, so we don't leak existence)
5. `rollback_to_target_id_is_head_returns_invariant`

### 5b. argo-audit test (extend existing exhaustiveness check)

The `pub const ALL` array will fail compilation if the new variant is missed.
Add a one-line assertion that `from_label("workflow_rolled_back")` round-trips.

### 5c. Gateway integration test

**File**: `crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs` (or
the established `*_tests.rs` location for that handler family — `rg
"#\[tokio::test\]" crates/argo-cli/src/handlers/gateway/routes_workflows*` to
confirm).

Recipe:
1. POST `/api/v1/verticals/{vid}/workflows/{name}` to create v1.
2. POST again to create v2 (via re-edit; head moves to v2).
3. POST `/api/v1/verticals/{vid}/workflows/{name}/rollback` with
   `{ target_version_id: <v1_id> }`.
4. Assert response is 200 + body `{ id, version: 3 }`.
5. GET `/api/v1/verticals/{vid}/workflows/{name}` (head) → assert TOML matches v1.
6. GET `/api/v1/audit/events?kind=workflow_rolled_back` → assert exactly one row
   exists with the expected payload fields.

### 5d. E2E smoke (Playwright)

**File**: `web/tests/workflow-rollback.spec.ts` (new) — only if Playwright is
already wired in `web/playwright.config.ts` (it is, as of #1620).

Recipe:
1. Sign in as test tenant owner.
2. Navigate to `/workflows/pr_checker/standard_review`.
3. Click "Rollback to v1" on the v1 row.
4. Verify modal opens with diff visible.
5. Click "Confirm rollback".
6. Wait for toast `"Rolled back to v1 (now v3)"`.
7. Assert viewer page reloads and top of version history shows "v3 · just now".
8. Navigate to `/audit` → assert most recent row has `kind: workflow_rolled_back`.

---

## may-touch

- `crates/argo-audit/src/event.rs` — add `WorkflowRolledBack` variant (3 sites: enum body, `label()`, `from_label()`, `ALL` array).
- `crates/argo-store/src/workflow_definition.rs` — re-export `rollback_to`.
- `crates/argo-store/src/workflow_definition_write.rs` — add `rollback_to` op (~80 LOC).
- `crates/argo-store/src/workflow_definition_tests.rs` — add 5 unit tests.
- `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — add `Route::Rollback` variant + parse arm + dispatch arm.
- `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs` — add `rollback()` handler.
- `crates/argo-cli/src/handlers/gateway/routes_workflows_tests.rs` (or established sibling) — add integration test.
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` — wire button + modal.
- `web/components/skycot/workflows/RollbackConfirmModal.tsx` (new) — modal component.
- `web/lib/api/workflow-rollback.ts` (new) — API client.
- `web/tests/workflow-rollback.spec.ts` (new) — E2E smoke.
- `web/app/landing/page.tsx` — final commit: revert "Rollback shipping next" → "One-tap rollback".

## must-NOT-touch

- `workflow_definition` / `workflow_definition_index` schema — rollback is purely additive at the row level, NO schema change.
- The builder page (`/workflows/[verticalId]/[name]/build/page.tsx`) — rollback is a viewer-page feature, not a builder feature.
- `workflow_edit_capability` / role presets — re-use, do not add new capability.
- File loaders in `crates/argo-vertical/src/loader.rs` — DB-first cutover is `mission-pm-workflow-edit-api` scope, not this brief.

---

## Acceptance criteria

- [ ] `POST /api/v1/verticals/{vid}/workflows/{name}/rollback` returns 200 + `{ id, version }`.
- [ ] Cross-tenant attempt returns 404 (not 403, not 500 — see existing `routes_workflows` pattern).
- [ ] Rollback-to-current-head returns 400 with a clear error.
- [ ] Web viewer: "Rollback to v\<N>" button visible on every prior-version row, absent on the current-head row.
- [ ] Confirmation modal shows a diff (side-by-side `<pre>` is acceptable in v1).
- [ ] Confirm modal is keyboard-accessible: focus trap + Esc close + Enter confirm.
- [ ] After confirm: success toast, page reloads, new version appears at top of history.
- [ ] Audit row visible in `/audit` with `kind: "workflow_rolled_back"` and payload includes both `rolled_back_from_version_id` and `new_version_id`.
- [ ] `cargo clippy --workspace -- -D warnings` clean.
- [ ] `cargo test -p argo-store --test workflow_definition_tests` passes (5 new tests).
- [ ] `cargo test -p argo-cli --lib routes_workflows` passes (1 new integration test).
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean in `web/`.
- [ ] Playwright spec `workflow-rollback.spec.ts` passes locally with the gateway running.
- [ ] **Final step in the merge commit**: revert `web/app/landing/page.tsx:46` softened copy from "Rollback shipping next" back to "One-tap rollback".

---

## Gotchas

1. **`AuditKind::WorkflowRolledBack` exhaustiveness**: the `ALL: &[Self]` array
   in `event.rs` is the canary — forgetting to add the new variant there will
   pass type-check but fail the exhaustiveness round-trip test. Add it to
   all four sites (enum, `label`, `from_label`, `ALL`).
2. **Cross-tenant guard must return `NotFound`, not `Invariant`**: leaking
   "this id exists but in a different tenant" is a tenant-isolation bug. The
   `rollback_to` op returns `NotFound` for both "id doesn't exist" and "id
   belongs to different tenant" — preserve that.
3. **Head-pointer flip must be in the same transaction as the insert**: if you
   split them, a crash between insert + head-flip leaves an orphan Draft row
   labelled Active. Keep both inside `tx`, commit once.
4. **TOML body is copied by value, not by reference**: rollback must produce a
   self-contained new row that doesn't depend on the target row continuing to
   exist. Future archive of the target row must not break the rollback.
5. **Audit payload must include BOTH ids**: consumers querying "what rolled
   back from what" need the `rolled_back_from_version_id` AND the
   `new_version_id` on the same row. Don't pair-emit two rows.
6. **Don't reuse `WorkflowHeadChanged`**: even though rollback internally
   moves the head pointer, the brief explicitly forbids emitting
   `WorkflowHeadChanged` for rollback events — the distinct kind is what makes
   `/audit?kind=workflow_rolled_back` filtering work.
7. **Web modal SSR**: the modal is a client component (uses focus + keyboard);
   mark `"use client"` at the top and guard with `if (!open) return null` so
   it doesn't render server-side.

---

## References

- Audit: `docs/skycot/audit/close-the-loop/2026-05-20-wave-5-close-the-loop.md`
- Promise source: `web/app/landing/page.tsx:46` (softened "Rollback shipping next")
- Substrate: `crates/argo-store/src/workflow_definition.rs` (DB layer doc comments)
- Audit kind pattern: `crates/argo-audit/src/event.rs` — search `WorkflowDefinitionCreated`
- Sibling route pattern: `crates/argo-cli/src/handlers/gateway/routes_workflows.rs` — `Head` route
- Capability already exists: `argo_capability::workflow_edit_capability`
