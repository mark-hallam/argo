-- mission-data-export-customer-flow — customer-facing GDPR/CCPA
-- Article 15 data export.
--
-- One row per requested export. `status` walks queued → running →
-- ready → expired (or → failed on assembly error). `download_token`
-- is the SHA-256 hex of a 32-byte random token handed to the user
-- when the job lands; the raw value never persists. `expires_at` is
-- 7 days from job completion so abandoned links time out.
--
-- Slot 116: reference DDL — applied at runtime by
-- `argo_store::data_export_job_query::ensure_schema` (same pattern as
-- magic_link_tokens / outbound_message). The versioned migration
-- runner caps at v70 so first-touch callers bootstrap on demand.

CREATE TABLE IF NOT EXISTS data_export_job (
    job_id            TEXT    PRIMARY KEY,
    user_id           BLOB    NOT NULL,
    tenant_id         BLOB,
    status            TEXT    NOT NULL,
    download_token    TEXT,
    error_message     TEXT,
    requested_at      INTEGER NOT NULL,
    completed_at      INTEGER,
    expires_at        INTEGER,
    bundle_bytes      BLOB
);

CREATE INDEX IF NOT EXISTS idx_data_export_job_user
    ON data_export_job(user_id, requested_at);

CREATE INDEX IF NOT EXISTS idx_data_export_job_token
    ON data_export_job(download_token);
