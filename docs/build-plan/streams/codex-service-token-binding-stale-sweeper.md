# codex-service-token-binding-stale-sweeper — periodic cleanup of stale `service_token_binding` rows

**Owner**: codex. **Depends on**: nothing (uses infrastructure shipped in #1865 which is on `main`). **Tier 1 — single-purpose mechanical clone.**

**Authored against**: origin/main `e23a4b09` on 2026-05-21.

Suggest fast-mode in the worker header — this is a pure clone-and-adapt task with no architectural decisions.

## Why

#1865 shipped `crates/argo-store/src/service_token_binding.rs` + `migrations/124__service_token_binding.sql`. The `service_token_binding` table stores `(token_hash, principal_id, tenant_id, label, created_at)` rows but has no TTL or cleanup path. Long-lived deployments will accumulate rows forever — including rows for tokens that were rotated, revoked, or whose principal was deleted. This brief adds a periodic sweeper that deletes rows older than a configurable TTL, mirroring the existing `approval_expiry_sweeper` pattern in the same directory.

Operator risk class: **low** (defensive housekeeping; no exposed API change; opt-in via env var with safe default).

## Approach (concrete, mirrors `approval_expiry_sweeper.rs`)

1. **NEW** `crates/argo-cli/src/handlers/gateway/service_token_binding_sweeper.rs` — clone the 65-LOC `approval_expiry_sweeper.rs` line-for-line, swapping:
   - `spawn_approval_expiry_sweeper` → `spawn_service_token_binding_sweeper`
   - `ARGO_APPROVAL_EXPIRY_TICK_SECS` → `ARGO_SERVICE_TOKEN_BINDING_TTL_TICK_SECS`
   - `DEFAULT_TICK_SECS: u64 = 30` → `DEFAULT_TICK_SECS: u64 = 3600` (hourly, not 30s — binding cleanup is not latency-critical)
   - `MIN_TICK_SECS: u64 = 5` → `MIN_TICK_SECS: u64 = 60`
   - `DECIDED_BY` → not needed; pass `"system:service-token-binding-sweeper"` directly to the new AuditKind
   - `approvals::expire_overdue(...)` → `service_token_binding::delete_stale_before(...)` (helper to be added in step 2)
   - `AuditKind::ApprovalExpiredByTimeout` → `AuditKind::ServiceTokenBindingExpired` (new variant added in step 3)
   - Audit payload `"kind": "approval.expired_by_timeout"` → `"kind": "service_token_binding.expired"`
   - The new TTL is read from env: `ARGO_SERVICE_TOKEN_BINDING_TTL_SECS` (default `86400 * 90` = 90 days; minimum 24h enforced via the same parse-with-warn pattern as the tick interval)

2. **MODIFY** `crates/argo-store/src/service_token_binding.rs` — add a single new helper, immediately after `delete_binding`:
   ```rust
   /// Delete bindings whose `created_at` is older than `cutoff_iso`.
   /// Returns the number of rows deleted.
   pub async fn delete_stale_before(
       pool: &SqlitePool,
       cutoff_iso: &str,
   ) -> ArgoResult<u64> { /* sqlx `DELETE FROM service_token_binding WHERE created_at < ?` */ }
   ```
   Keep the file under 500 LOC (it's well under today). No schema change; cutoff uses the existing `created_at TEXT` ISO-8601 column.

3. **MODIFY** `crates/argo-audit/src/event.rs` — add a single new `AuditKind` variant:
   - Add `ServiceTokenBindingExpired,` to the enum (alphabetical-ish neighbour: between `ServiceTokenBindingCreated` if it exists else near `ApprovalExpiredByTimeout`).
   - Add `Self::ServiceTokenBindingExpired => "service_token_binding.expired",` to the `as_kind_str` `match`.
   - Add `"service_token_binding.expired" => Self::ServiceTokenBindingExpired,` to the `from_kind_str` `match`.
   - Add `Self::ServiceTokenBindingExpired,` to the `ALL_KINDS` slice / iterator if present.
   Grep first to confirm exact match-arm layout: `rg 'ApprovalExpiredByTimeout' crates/argo-audit/src/`.

4. **MODIFY** `crates/argo-cli/src/handlers/gateway/mod.rs` — add `mod service_token_binding_sweeper;` immediately under the existing `mod approval_expiry_sweeper;` line.

5. **MODIFY** `crates/argo-cli/src/handlers/gateway/boot_serve.rs` — 3 small additions mirroring the approval sweeper wiring at lines 24, 148, 168-169:
   - Add `service_token_binding_sweeper` to the `use super::{...}` import on line 24.
   - Add `_service_token_binding_sweeper: Option<JoinHandle<()>>,` to the `_BackgroundTasks` struct near line 148.
   - Add `_service_token_binding_sweeper: bg_store.and_then(|s| service_token_binding_sweeper::spawn_service_token_binding_sweeper(Arc::clone(s))),` in the init block near line 168.

6. **NEW** `crates/argo-cli/src/handlers/gateway/service_token_binding_sweeper_tests.rs` — clone `approval_expiry_sweeper_tests.rs` (91 LOC) and adapt. Must cover:
   - **`sweep_zero_when_table_empty`** — fresh DB, sweep returns Ok(0), no audit row written.
   - **`sweep_deletes_stale_rows_and_emits_audit`** — insert 3 rows with `created_at` older than the TTL + 2 rows newer; sweep returns Ok(3); only the 2 fresh rows remain; exactly one audit row written with `kind=service_token_binding.expired` + `count=3`.
   - **`tick_secs_from_env_valid_value`** + **`tick_secs_from_env_too_small_clamped`** + **`tick_secs_from_env_garbage_falls_back`** — mirror the existing tick env-var tests.
   - **`ttl_secs_from_env_valid_value`** + **`ttl_secs_from_env_below_minimum_clamped`** + **`ttl_secs_from_env_garbage_falls_back`** — three more tests for the new TTL env var.

## Files you may touch (verified to exist on `e23a4b09`)

- `crates/argo-cli/src/handlers/gateway/service_token_binding_sweeper.rs` — NEW (~70 LOC).
- `crates/argo-cli/src/handlers/gateway/service_token_binding_sweeper_tests.rs` — NEW (~120 LOC).
- `crates/argo-cli/src/handlers/gateway/mod.rs` — MODIFY (1 line: `mod service_token_binding_sweeper;`).
- `crates/argo-cli/src/handlers/gateway/boot_serve.rs` — MODIFY (3 small additions).
- `crates/argo-store/src/service_token_binding.rs` — MODIFY (one new `pub async fn delete_stale_before`).
- `crates/argo-audit/src/event.rs` — MODIFY (one new `AuditKind` variant + 3 `match` arms).

Anything else is out of scope. Total LOC budget: ~250 net new.

## Verification

- `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- `cargo clippy -p argo-audit --all-targets -- -D warnings` clean.
- `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- `cargo test -p argo-store service_token_binding` green.
- `cargo test -p argo-cli service_token_binding_sweeper` green (7 unit tests).
- Final `bash scripts/workspace-clippy-fast-fail.sh` sanity check clean.

## Acceptance

- [ ] All 6 files touched, paths match the may-touch list.
- [ ] Each touched file `<500 LOC` (HARD RULE 6).
- [ ] No new crate added; no new migration slot consumed; no public-API rename.
- [ ] The new `AuditKind::ServiceTokenBindingExpired` round-trips through `as_kind_str` ↔ `from_kind_str`.
- [ ] Sweeper is opt-in via env var with safe default (90-day TTL, 1h tick); no behaviour change unless an operator sets the env vars OR a deployment runs > 90d.
- [ ] PR body claims crate-scoped clippy/tests + final `workspace-clippy-fast-fail.sh` run.
- [ ] PR body declares any boundary deviations.
- [ ] No `unwrap`/`expect` outside `#[cfg(test)]` (CI invariant).

## Why this is a codex-sized brief

- Single architectural pattern, already used in this exact directory.
- 2 crates touched, narrow may-touch (6 specific files).
- No new pub types beyond one trivial AuditKind variant.
- No multi-system reasoning; the spawner wire-up is a 3-line addition mirroring an adjacent existing call.
- Worker can complete in 1-2h including all 7 tests + verification.
