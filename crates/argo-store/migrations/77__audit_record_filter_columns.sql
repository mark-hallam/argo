-- Stream `mission-enterprise-audit-full-text-search` — promote the
-- 7 J14 audit-feed filter dimensions from app-layer `payload_json`
-- parses to first-class columns, plus 3 composite indexes covering
-- the dominant filter combos.
--
-- Source of truth: `payload_json` stays canonical; the new columns
-- are denormalizations the write path populates from the same JSON
-- fields the J14 reader currently parses. NOT part of the hash chain
-- (chain inputs remain {id, session_id, kind, payload_json,
-- parent_hash}); `verify_chain` gains a denorm sanity check that the
-- two never drift.
--
-- Backfill posture: nullable columns; historical rows stay NULL so
-- the audit chain hashes never invalidate. The audit-feed handler
-- falls back to the existing app-layer `payload_json` parse for any
-- row with a NULL filter column (downgrade-safe).
--
-- Schema-version slot: Rust counter is v67 (next free after v66 on
-- current main); raw filename slot is 77 (65, 66, and 72-76 are
-- occupied on main).
-- Filename and version counters advance independently per the v55
-- convention.
--
-- Idempotent: `ADD COLUMN` (no IF NOT EXISTS on SQLite) is guarded
-- via PRAGMA-style migration ordering — once v67 commits, the
-- columns exist; re-running is prevented by the `schema_version`
-- gate in `migrations::run`. Indexes are IF NOT EXISTS for safety
-- on resumed migrations.
--
-- Down migration drops indexes in reverse order then drops the
-- columns (see `migrations/v67.rs`).

ALTER TABLE audit_record ADD COLUMN worker_id TEXT;
ALTER TABLE audit_record ADD COLUMN channel TEXT;
ALTER TABLE audit_record ADD COLUMN action_type TEXT;
ALTER TABLE audit_record ADD COLUMN outcome_status TEXT;
ALTER TABLE audit_record ADD COLUMN approver_id TEXT;
ALTER TABLE audit_record ADD COLUMN cost_band TEXT;
ALTER TABLE audit_record ADD COLUMN data_classification TEXT;

-- Composite `(created_at, <dim>)` indexes covering the dominant
-- filter combos. J14 endpoint hits these in time-window-then-dim
-- order, so the leading `created_at` short-circuits before the
-- equality probe.
CREATE INDEX IF NOT EXISTS idx_audit_record_created_at_worker
    ON audit_record(created_at, worker_id);

CREATE INDEX IF NOT EXISTS idx_audit_record_created_at_action
    ON audit_record(created_at, action_type);

CREATE INDEX IF NOT EXISTS idx_audit_record_created_at_status
    ON audit_record(created_at, outcome_status);
