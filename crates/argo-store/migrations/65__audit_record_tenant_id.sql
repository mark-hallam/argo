-- Stream `mission-enterprise-cross-tenant-audit` — add `tenant_id`
-- denorm column to `audit_record` so the audit feed can scope rows
-- per tenant without changing the hash-chain inputs.
--
-- The `__legacy__` sentinel grandfathers every existing row into one
-- bucket; the default workspace tenant continues to read that bucket
-- (see `routes_audit_feed.rs`) so existing audit visibility is
-- preserved without an env-flag flip. Operators running a real
-- multi-tenant deployment migrate `__legacy__` rows to real tenant
-- ids via a follow-up CLI (`mission-audit-legacy-tenant-migrate-cli`).
--
-- `tenant_id` is a denormalized column — the same value is also
-- stamped inside `payload_json` at write time. The chain hash inputs
-- (`parent_hash, kind, canonical(payload)`) are unchanged so every
-- historical `self_hash` keeps verifying. `verify_chain` cross-checks
-- the denorm column against `payload_json.tenant_id` for new rows.
--
-- Slot rationale: original brief targeted slot 65 anticipating two
-- ahead-of-this briefs (J16 elevated_view_grant + audit-fts) to land
-- first. J16 merged ahead taking slot 63 / v59 (#1426); audit-fts is
-- still IN_REVIEW so this brief takes the next free slot 64 and the
-- audit-fts brief bumps to 65 once it lands. Rust version counter is
-- v60 (next free after v59 = `elevated_view_grant`); filename +
-- version counters advance independently per the v55 module's
-- documented convention.

ALTER TABLE audit_record
    ADD COLUMN tenant_id TEXT NOT NULL DEFAULT '__legacy__';

CREATE INDEX IF NOT EXISTS idx_audit_record_tenant_created
    ON audit_record(tenant_id, created_at);
