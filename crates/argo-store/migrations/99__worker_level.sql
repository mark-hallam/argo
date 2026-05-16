-- Stream `claude-worker-promotion-workflow` (mission claim 2): adds the
-- per-worker tenure `level` column. Workers start at `entry`; the
-- `argo worker promote` CLI walks them up the entry → junior → senior
-- → lead ladder once tenure / sessions / success-rate gates clear.
--
-- The brief originally reserved migration 41 but slot 41 landed as
-- `41__roster_role_removed_at.sql` (#1076), 42 as worker_presence
-- (#1056), 43+44 as worker_objective + worker_key_result (#1070), so
-- this rolls forward to slot 45.
--
-- Additive + idempotent via the matching `ensure_worker_level_columns`
-- helper in `argo-identity` (mirrors the charter / OKR pattern).
ALTER TABLE worker ADD COLUMN level TEXT NOT NULL DEFAULT 'entry';

CREATE INDEX IF NOT EXISTS idx_worker_level ON worker(level);
