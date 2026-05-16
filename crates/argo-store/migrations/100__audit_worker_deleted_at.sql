-- Stream `mission-identity-delete-worker` (mission claim 1: identity
-- delete + GDPR cascade alignment).
--
-- Adds a nullable `worker_deleted_at` tombstone column on `audit_record`
-- so `worker_archive::delete_worker` can mark every audit row that
-- referenced a now-purged worker without breaking the chain hashes.
-- The hashed columns (`id`, `session_id`, `kind`, `payload_json`,
-- `parent_hash`, `self_hash`, `created_at`) are NOT touched, so
-- tamper-evidence holds end-to-end.
--
-- The tombstone is stamped via a join through `session.worker_id`
-- (added in `38__session_worker_id.sql`). Rows whose `session_id`
-- is NULL or whose session has a different worker stay untouched.
-- The partial index supports per-deleted-worker GDPR reports.
--
-- Applied lazily via `worker_archive::ensure_audit_tombstone_column`
-- so the file does NOT need a versioned-slot wrapper in
-- `src/migrations/` — mirrors the additive-column pattern used by
-- `41__roster_role_removed_at.sql` (`team_roster::ensure_team_roster_columns`).

ALTER TABLE audit_record ADD COLUMN worker_deleted_at INTEGER;

CREATE INDEX IF NOT EXISTS idx_audit_worker_deleted_at
    ON audit_record(worker_deleted_at) WHERE worker_deleted_at IS NOT NULL;
