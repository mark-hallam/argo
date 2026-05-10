-- Stream `mvp-audit-principal-stamp` (Tier 1 MVP — closes mission
-- claim "everything is auditable").
--
-- Adds `audit_record.principal_id` so multi-tenant deployments can
-- attribute every audit row to the resolved gateway caller. Bearer
-- auth landed in #946 (`mission-gateway-bearer-auth`); this column
-- is the persistence side of that flow. NULL is allowed for
-- system-internal writes (e.g. background jobs that don't run inside
-- a gateway request).
ALTER TABLE audit_record ADD COLUMN principal_id TEXT;

CREATE INDEX IF NOT EXISTS idx_audit_principal
    ON audit_record(principal_id);
