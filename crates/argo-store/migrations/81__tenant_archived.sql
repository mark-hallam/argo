-- Stream `codex-tenant-cli-extensions` — tenant lifecycle metadata.
--
-- `archived_at` is a soft-delete marker: archived tenants remain
-- queryable for audit/export, but default operator listings hide
-- them. `owner_email` records the operator of record and is
-- updated by `argo tenant transfer`.

ALTER TABLE workspace ADD COLUMN archived_at INTEGER;
ALTER TABLE workspace ADD COLUMN owner_email TEXT;

CREATE TRIGGER IF NOT EXISTS worker_reject_archived_tenant_insert
BEFORE INSERT ON worker
FOR EACH ROW
WHEN EXISTS (
    SELECT 1 FROM workspace
    WHERE archived_at IS NOT NULL
      AND lower(hex(tenant_id)) = replace(lower(NEW.tenant_id), '-', '')
)
BEGIN
    SELECT RAISE(ABORT, 'tenant is archived');
END;

CREATE TRIGGER IF NOT EXISTS worker_reject_archived_tenant_update
BEFORE UPDATE OF tenant_id ON worker
FOR EACH ROW
WHEN EXISTS (
    SELECT 1 FROM workspace
    WHERE archived_at IS NOT NULL
      AND lower(hex(tenant_id)) = replace(lower(NEW.tenant_id), '-', '')
)
BEGIN
    SELECT RAISE(ABORT, 'tenant is archived');
END;
