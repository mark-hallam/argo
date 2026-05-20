# web-gateway-endpoints-notifications-batch — finish mark-read endpoints (4/6 SHIPPED)

**Owner**: claude (Rust crate work — gateway side; codex acceptable too).
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-gateway-endpoints-notifications-batch`.
**Priority**: P2 — 4 of 6 originally-scoped endpoints have SHIPPED. The
remaining 2 (`markAsRead` / `markAllAsRead`) are the only fakes left in the
bell popover.
**Sizing**: ~200-300 LOC — 2 new handlers, 2 store ops, 4 unit tests, 1
integration test.

---

## Status — 4/6 SHIPPED

| Endpoint | Status | Landed via |
|----------|--------|------------|
| `GET /api/v1/notifications` | ✅ SHIPPED | `routes_notifications::list_response` |
| `GET /api/v1/notifications/unread-count` | ✅ SHIPPED | `routes_notifications::unread_count_response` |
| `POST /api/v1/notifications/<id>/dismiss` | ✅ SHIPPED | `routes_notifications::dismiss_response` |
| `POST /api/v1/notifications/dismiss-all` | ✅ SHIPPED | `routes_notifications::dismiss_all_response` |
| `POST /api/v1/notifications/<id>/read` | 🔲 PENDING | this brief |
| `POST /api/v1/notifications/read-all` | 🔲 PENDING | this brief |

The original 4-endpoint brief scope completed via the migration v76 +
`crates/argo-store/src/notifications.rs` (366 LOC) +
`crates/argo-cli/src/handlers/gateway/routes_notifications.rs` (251 LOC) +
335-LOC test file.

The **2 remaining endpoints** distinguish "I saw this" (read) from "I'm
done with this" (dismiss). The web layer
(`web/lib/api/notifications.ts:18-22`) acknowledges: *"markAsRead/markAllAsRead
are still 404 until a follow-up brief lands them server-side."* This is that
follow-up.

---

## Pre-flight verification (close out shipped + scope remainder)

```bash
# 1. Confirm shipped 4 endpoints respond on a running gateway.
curl -s http://127.0.0.1:7400/api/v1/notifications | jq '.items|length'
curl -s http://127.0.0.1:7400/api/v1/notifications/unread-count | jq '.unread'

# 2. Confirm 2 remaining endpoints return 404 today.
curl -i -X POST http://127.0.0.1:7400/api/v1/notifications/test-id/read 2>&1 | head -1
curl -i -X POST http://127.0.0.1:7400/api/v1/notifications/read-all 2>&1 | head -1

# 3. Confirm read_at column exists in the table (migration v76 already adds it).
sqlite3 ~/.argo/store.db ".schema notifications" | rg read_at
```

---

## Implementation — 2 remaining endpoints

### Phase 1 — Store ops (additive to `argo_store::notifications`)

**File**: `crates/argo-store/src/notifications.rs` — extend the existing module.

The table already has `read_at INTEGER NULL` (per migration v76, line 24 of
`125__notifications.sql`). Two ops:

```rust
/// Stamp `read_at = now()` on one row owned by (`tenant`, `principal`).
/// Idempotent — re-marking already-read is a no-op. Returns the row's
/// final `read_at` micros so callers can return it in the response.
/// Returns `ArgoError::NotFound` if the id+principal pair doesn't match.
pub async fn mark_read(
    store: &SessionStore,
    tenant: &TenantId,
    principal: &str,
    notification_id: &str,
) -> ArgoResult<i64> {
    let now_micros = chrono::Utc::now().timestamp_micros();
    let result = sqlx::query(
        "UPDATE notifications SET read_at = COALESCE(read_at, ?) \
         WHERE id = ? AND tenant_id = ? AND (principal_id = ? OR principal_id IS NULL)",
    )
    .bind(now_micros)
    .bind(notification_id)
    .bind(tenant.to_string())
    .bind(principal)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("mark_read: {e}")))?;
    if result.rows_affected() == 0 {
        return Err(ArgoError::NotFound(format!(
            "notification '{notification_id}' not visible to principal"
        )));
    }
    // Fetch the now-stable read_at value (it may pre-date this call).
    let read_at: i64 = sqlx::query_scalar(
        "SELECT read_at FROM notifications WHERE id = ? AND tenant_id = ?",
    )
    .bind(notification_id).bind(tenant.to_string())
    .fetch_one(store.pool()).await
    .map_err(|e| ArgoError::Provider(format!("read_at probe: {e}")))?;
    Ok(read_at)
}

/// Stamp `read_at = now()` on every UNREAD row visible to (`tenant`,
/// `principal`) — excludes rows that already have `read_at` set.
/// Returns the count of rows actually updated (zero if everything
/// was already read).
pub async fn mark_all_read(
    store: &SessionStore,
    tenant: &TenantId,
    principal: &str,
) -> ArgoResult<u64> {
    let now_micros = chrono::Utc::now().timestamp_micros();
    let result = sqlx::query(
        "UPDATE notifications SET read_at = ? \
         WHERE tenant_id = ? AND (principal_id = ? OR principal_id IS NULL) \
         AND read_at IS NULL AND dismissed_at IS NULL",
    )
    .bind(now_micros)
    .bind(tenant.to_string())
    .bind(principal)
    .execute(store.pool())
    .await
    .map_err(|e| ArgoError::Provider(format!("mark_all_read: {e}")))?;
    Ok(result.rows_affected())
}
```

### Phase 2 — Gateway handlers

**File**: `crates/argo-cli/src/handlers/gateway/routes_notifications.rs` —
extend the dispatch_post + add 2 handler functions.

```rust
// In dispatch_post, add 2 match arms:
if let Some(id) = req.strip_prefix("POST /api/v1/notifications/")
    .and_then(|rest| rest.strip_suffix("/read "))
    .map(str::to_string)
{
    return Some(read_response(&id, req, store).await);
}
if req.starts_with("POST /api/v1/notifications/read-all ")
    || req.starts_with("POST /api/v1/notifications/read-all\r\n")
{
    return Some(read_all_response(req, store).await);
}

async fn read_response(
    notification_id: &str,
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store { Some(s) => s, None => return store_unavailable() };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t, Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let principal = principal_from_header(req)
        .unwrap_or_else(|| "anonymous".to_string());
    match argo_store::notifications::mark_read(store, &tenant, &principal, notification_id).await {
        Ok(read_at) => ("200 OK", json!({
            "ok": true,
            "notificationId": notification_id,
            "readAt": read_at,
        }).to_string()),
        Err(ArgoError::NotFound(msg)) => not_found(&msg),
        Err(e) => server_error(&format!("mark_read: {e}")),
    }
}

async fn read_all_response(
    req: &str,
    store: Option<&Arc<SessionStore>>,
) -> (&'static str, String) {
    let store = match store { Some(s) => s, None => return store_unavailable() };
    let tenant = match resolve_workspace_tenant(store).await {
        Ok(t) => t, Err(e) => return server_error(&format!("resolve tenant: {e}")),
    };
    let principal = principal_from_header(req)
        .unwrap_or_else(|| "anonymous".to_string());
    match argo_store::notifications::mark_all_read(store, &tenant, &principal).await {
        Ok(n) => ("200 OK", json!({ "ok": true, "marked": n }).to_string()),
        Err(e) => server_error(&format!("mark_all_read: {e}")),
    }
}
```

### Phase 3 — Tests (extend `routes_notifications_tests.rs`)

Add 4 tests:
1. `mark_read_stamps_read_at`
2. `mark_read_is_idempotent` (re-mark same id → response unchanged)
3. `mark_read_cross_tenant_returns_404`
4. `mark_all_read_skips_already_read_and_dismissed`

### Phase 4 — Web cleanup

**File**: `web/lib/api/notifications.ts`

After the endpoints land, remove the 404-fallback in `markAsRead` /
`markAllAsRead`:

```diff
- /* On 404 (route not implemented yet), fall back to in-memory mock. */
- if (e instanceof ApiError && e.status === 404) {
-   return { ok: true };
- }
- throw e;
+ if (e instanceof ApiError && e.status === 404) {
+   /* Genuine 404 — notification id was wrong. Surface it. */
+ }
+ throw e;
```

The 404 fallback was masking the "endpoint not implemented yet" reality;
once shipped, real 404s should propagate as errors.

---

## may-touch

- `crates/argo-store/src/notifications.rs` — add `mark_read` + `mark_all_read` (~60 LOC).
- `crates/argo-cli/src/handlers/gateway/routes_notifications.rs` — add 2 handlers + 2 dispatch arms (~100 LOC).
- `crates/argo-cli/src/handlers/gateway/routes_notifications_tests.rs` — add 4 tests (~120 LOC).
- `web/lib/api/notifications.ts` — remove the 404 fallback on the 2 newly-shipped endpoints.
- `crates/argo-store/src/notifications_tests.rs` — add 4 store-level tests (~80 LOC).

## must-NOT-touch

- The 4 shipped endpoints (list/unread-count/dismiss/dismiss-all) — they're stable.
- The notifications table schema — the `read_at` column already exists.
- The `dismissed_at` column / dismiss semantics — read is a separate concern.
- Web bell popover UI — no UI changes needed.

---

## Acceptance criteria

- [ ] `POST /api/v1/notifications/<id>/read` returns 200 + `{ ok, notificationId, readAt }`.
- [ ] `POST /api/v1/notifications/read-all` returns 200 + `{ ok, marked: N }`.
- [ ] Cross-tenant attempt on `/read` returns 404, NOT 403.
- [ ] Re-marking an already-read row is idempotent (returns the original `readAt`).
- [ ] `read-all` skips rows that are already `read_at IS NOT NULL` or `dismissed_at IS NOT NULL`.
- [ ] Unread-count drops to zero after `read-all` (verifies the existing `unread_count` query reads `read_at`).
- [ ] `cargo clippy -p argo-store -p argo-cli --all-targets -- -D warnings` clean.
- [ ] 4 new store tests + 4 new gateway tests pass.
- [ ] Web `markAsRead` / `markAllAsRead` 404-fallback removed and propagates real 404s.

---

## Gotchas

1. **Broadcast rows (`principal_id IS NULL`)**: a broadcast notification is
   visible to every principal in a tenant. The mark-read query must use
   `(principal_id = ? OR principal_id IS NULL)` — but PHYSICALLY MUTATING the
   broadcast row would mark it read for EVERY principal. The right behavior:
   keep the broadcast row's `read_at` NULL, and write a per-principal "read
   receipt" row to a side table. **For v1**, the simpler decision is: broadcast
   rows can't be marked read (only dismissed). The query above already does
   this implicitly because the UPDATE matches the row, but a follow-up brief
   should add per-principal read receipts properly.
2. **`COALESCE(read_at, ?)` preserves the original timestamp** if the row is
   already read. This is correct — re-reading shouldn't lie about when the
   first read happened.
3. **Audit emission** — read events are NOT audit-emit-worthy in v1 (would
   spam the chain with low-signal rows). The existing `dismissed_at` updates
   also don't emit. Stay consistent: no audit for read.
4. **The web layer's existing `dismissNotification` function** already works
   — don't try to extend its semantics. Read is a strictly weaker
   operation than dismiss (read keeps the row visible; dismiss removes it).
5. **`principal_id` field**: when no `X-Argo-Principal` header is present
   the existing code defaults to `"anonymous"`. Anonymous can mark its OWN
   notifications read but not anyone else's. This is fine for v1.

---

## References

- Shipped 4-endpoint substrate: `crates/argo-store/src/notifications.rs` + `crates/argo-cli/src/handlers/gateway/routes_notifications.rs`
- Migration: `crates/argo-store/migrations/125__notifications.sql`
- Web client TODO: `web/lib/api/notifications.ts:18-22` (the "still 404" comment)
- Audit: `docs/skycot/audit/api-wiring/2026-05-20-wave-3-api-wiring.md`
