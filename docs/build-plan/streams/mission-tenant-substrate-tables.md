# mission-tenant-substrate-tables — `tenant` + `user_account` + `user_tenant` row tables (Wave 2 P0 substrate)

**Owner**: claude. **Depends on**: none — this is the precursor that unblocks `claude-tenant-provisioning-endpoint` and `claude-stripe-subscription-integration` (both currently BLOCKED on this substrate).
**Authored against**: origin/main `382001ff` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-tenant-substrate-tables`.
**Source**: BLOCKED notes on PRs 720 + 723 — workers correctly identified that tenancy is UUID-only on main; row-backed tables don't exist; brief premises require them.

## Mission alignment

- **Multi-tenant by construction** (CLAUDE.md core invariant #2) — completes the substrate so tenancy is row-anchored, not just UUID-identity.
- **Commercial usability** (priority #5) — self-serve signup needs a tenant row to attach billing + tier + name to.
- **Reliability** (priority #1) — operator + audit surface needs to enumerate tenants, not just look them up by ID.

## Scope

P0. Add three row-backed tables to substrate; ID-keyed today + name/email/role metadata next to them.

### Tables (3 new migrations: slots 106, 107, 108 — next free after #1759's 104/105)

**`106__tenant.sql`**:
```sql
CREATE TABLE tenant (
    id              BLOB PRIMARY KEY,             -- TenantId UUID
    name            TEXT NOT NULL,                -- operator-facing, e.g. "Acme Co"
    billing_email   TEXT NOT NULL,                -- where invoices + alerts go
    status          TEXT NOT NULL DEFAULT 'active', -- 'active' | 'suspended' | 'deleted'
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);
CREATE INDEX idx_tenant_billing_email ON tenant(billing_email);
CREATE INDEX idx_tenant_status        ON tenant(status);
```

**`107__user_account.sql`**:
```sql
CREATE TABLE user_account (
    id              BLOB PRIMARY KEY,             -- UserId UUID
    email           TEXT NOT NULL UNIQUE,         -- magic-link target
    display_name    TEXT,                          -- nullable until verify
    created_at      INTEGER NOT NULL,
    last_login_at   INTEGER,                       -- nullable; updated on verify
    status          TEXT NOT NULL DEFAULT 'active' -- 'active' | 'disabled'
);
CREATE INDEX idx_user_account_status ON user_account(status);
```

**`108__user_tenant.sql`** (join table):
```sql
CREATE TABLE user_tenant (
    user_id     BLOB NOT NULL,
    tenant_id   BLOB NOT NULL,
    role        TEXT NOT NULL,                    -- 'owner' | 'admin' | 'user' | 'guest'
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (user_id, tenant_id),
    FOREIGN KEY (user_id)   REFERENCES user_account(id),
    FOREIGN KEY (tenant_id) REFERENCES tenant(id)
);
CREATE INDEX idx_user_tenant_tenant ON user_tenant(tenant_id);
CREATE INDEX idx_user_tenant_user   ON user_tenant(user_id);
```

### Rust types (argo-identity)

```rust
// crates/argo-identity/src/tenant.rs (NEW)
pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    pub billing_email: String,
    pub status: TenantStatus,
    pub created_at: i64,
    pub updated_at: i64,
}
pub enum TenantStatus { Active, Suspended, Deleted }
impl Tenant {
    pub fn new(name: String, billing_email: String) -> Self { ... }
    pub fn set_status(&mut self, status: TenantStatus) { ... }
}

// crates/argo-identity/src/user.rs (NEW)
pub struct UserAccount {
    pub id: UserId,
    pub email: String,
    pub display_name: Option<String>,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
    pub status: UserStatus,
}
pub enum UserStatus { Active, Disabled }

// crates/argo-identity/src/user_tenant.rs (NEW)
pub struct UserTenant { /* link */ }
pub enum UserTenantRole { Owner, Admin, User, Guest }
```

### Query helpers (argo-store)

```rust
// crates/argo-store/src/tenant_query.rs — extend (file added by #1759):
pub fn insert_tenant(conn: &Connection, t: &Tenant) -> Result<()> ;
pub fn get_tenant(conn: &Connection, id: TenantId) -> Result<Option<Tenant>>;
pub fn list_tenants(conn: &Connection) -> Result<Vec<Tenant>>;
pub fn set_tenant_status(conn: &Connection, id: TenantId, status: TenantStatus) -> Result<()>;

// crates/argo-store/src/user_query.rs (NEW):
pub fn insert_user(...) ; get_user_by_email(...) ; update_last_login(...);

// crates/argo-store/src/user_tenant_query.rs (NEW):
pub fn link_user_to_tenant(...) ; list_tenants_for_user(...) ; list_users_in_tenant(...);
```

LOC: ~400.

## Files you may touch

**Migrations:**
- `crates/argo-store/migrations/106__tenant.sql` (NEW) ≤25 LOC.
- `crates/argo-store/migrations/107__user_account.sql` (NEW) ≤20 LOC.
- `crates/argo-store/migrations/108__user_tenant.sql` (NEW) ≤25 LOC.

**Rust types:**
- `crates/argo-identity/src/tenant.rs` (NEW) ≤120 LOC.
- `crates/argo-identity/src/user.rs` (NEW) ≤80 LOC.
- `crates/argo-identity/src/user_tenant.rs` (NEW) ≤60 LOC.
- `crates/argo-identity/src/lib.rs` — re-export. ≤6 modified LOC.
- `crates/argo-identity/src/tenant_tests.rs` (NEW) ≤80 LOC.
- `crates/argo-identity/src/user_tests.rs` (NEW) ≤60 LOC.

**Queries:**
- `crates/argo-store/src/tenant_query.rs` (extend existing — added by #1759) ≤100 modified LOC.
- `crates/argo-store/src/user_query.rs` (NEW) ≤120 LOC.
- `crates/argo-store/src/user_tenant_query.rs` (NEW) ≤80 LOC.
- `crates/argo-store/src/lib.rs` — re-export. ≤6 modified LOC.

**Tests:**
- `crates/argo-store/src/tenant_query_tests.rs` (extend) ≤80 modified LOC.
- `crates/argo-store/src/user_query_tests.rs` (NEW) ≤100 LOC.
- `crates/argo-store/src/user_tenant_query_tests.rs` (NEW) ≤80 LOC.

## Files you must NOT touch

- The existing `workspace` table (it stays UUID-keyed with FK to tenant_id; this brief does not change workspace).
- `argo-cli` / gateway handlers — `claude-tenant-provisioning-endpoint` (Wave 2 P0) consumes this substrate. Out of scope.
- The audit chain — tenants in audit rows continue to be `tenant_id` UUID only; the new tenant table is row-metadata, not audit-row metadata.
- Existing `tenant_quota`, `tenant_rate_limit`, `tenant_tier`, `tenant_usage` tables — they remain keyed by tenant_id UUID. No FK enforcement to the new `tenant` table (avoids cascade-delete risk on data-classification rows).

## Acceptance criteria

- [ ] Migrations 106/107/108 apply cleanly on a fresh DB.
- [ ] Migrations apply additively on a DB already populated with existing tenants (TenantId UUIDs in workspace/tenant_quota/etc.) — does NOT require backfill rows in new tenant table (no FK enforcement).
- [ ] `Tenant::new("Acme", "billing@acme.com")` produces a `Tenant` with `status=Active` + `created_at`/`updated_at` set to `now()`.
- [ ] `insert_tenant + get_tenant` round-trip.
- [ ] `list_tenants()` returns all rows.
- [ ] `set_tenant_status(id, Suspended)` updates the row + bumps `updated_at`.
- [ ] `insert_user + get_user_by_email` round-trip.
- [ ] `update_last_login(user_id)` updates `last_login_at` to now.
- [ ] `link_user_to_tenant(user, tenant, Owner)` creates a row.
- [ ] `list_tenants_for_user(user)` returns all tenants the user is linked to.
- [ ] `list_users_in_tenant(tenant)` returns all users with their role.
- [ ] Unique constraint on `user_account.email` enforced (insert duplicate → error).
- [ ] `cargo test -p argo-store -p argo-identity` passes.
- [ ] `cargo clippy -p argo-store -p argo-identity --all-targets -- -D warnings` clean.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `mission-tenant-substrate-tables`
- Title: `feat(substrate): tenant + user_account + user_tenant row tables (mission-tenant-substrate-tables)`
- Worker verification: crate-scoped clippy + tests + `cargo clippy --workspace` per SWARM_PROTOCOL.md.

## Hidden constraints

- **Strict additive migration.** Existing TenantId-UUID-only rows in `workspace`, `tenant_quota`, `tenant_rate_limit`, `tenant_tier`, `tenant_usage` MUST continue to work without a corresponding tenant table row. The new tenant table is **metadata, not authority** — multi-tenant code that touches workspace etc. continues to use UUID identity.
- **No FK from existing tables to `tenant`.** That would break the additive-migration rule.
- **Migration slot picks 106/107/108.** Don't accept slot reuse — `crates/argo-store/migrations/duplicate_slot_validator.rs` will reject duplicates at startup.
- **`tenant_tier` column on `workspace` (slot 104) stays** — tier-on-workspace is what `claude-pricing-plans-config` chose; consumers can JOIN tenant → workspace → tier or just read workspace.tier.
- **Status enum normalization.** Lowercase: `'active' | 'suspended' | 'deleted'`. Case-sensitive storage; case-insensitive comparison NOT required.
- **`billing_email` is NOT a foreign key to `user_account`.** Tenants can have a billing email distinct from any user (e.g. `billing@acme.com` going to an Acme accounting alias).
- **No cascade-delete.** Setting tenant status to 'deleted' is a soft delete; audit + data-classification rows persist.

## Unblocks

- `claude-tenant-provisioning-endpoint` (#720 BLOCKED) — backend can create tenant + workspace + user_tenant rows.
- `claude-stripe-subscription-integration` (#723 BLOCKED) — webhook can write `tenant.tier` via the `workspace.tier` column added by #1759.
- Foundation for future user-management briefs (invite flow, role changes, etc.).
