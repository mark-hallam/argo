-- Stream `claude-worker-team-roster-cli` (V1.1) — additive
-- `role` and `removed_at` columns on `roster_member` to support
-- the org-chart-aware team CLI surface.
--
-- Migration 22 (`22__roster.sql`) created `roster_member` with
-- only `(roster_id, worker_id, added_at)`. The team CLI needs
-- two additions:
--   * `role` TEXT — operator-supplied label per (team, worker).
--     Examples: `lead`, `analyst`, `apprentice`. Free-form;
--     team CLI does not enforce a fixed vocabulary.
--   * `removed_at` INTEGER — soft-delete timestamp (seconds
--     since epoch). When non-null the row is treated as historic
--     and excluded from active-member counts. Hard-delete would
--     create dangling FKs against audit / KPI / review rows that
--     reference the `(team, worker)` pair; soft-delete preserves
--     history without the cleanup risk.
--
-- Both columns are NULLable so existing genesis-template rows
-- remain valid. The partial index on active rows speeds up the
-- `team list` / `team show` member-count queries.
--
-- Applied lazily via `team_roster::ensure_team_roster_columns`
-- (sibling-file pattern, mirrors `ensure_session_worker_id_column`
-- in `lib.rs`). The structured migration runner in
-- `src/migrations/mod.rs` is not extended here so the may-touch
-- list stays inside `src/team_roster.rs` + `src/lib.rs`.

ALTER TABLE roster_member ADD COLUMN role TEXT;
ALTER TABLE roster_member ADD COLUMN removed_at INTEGER;
CREATE INDEX IF NOT EXISTS idx_roster_member_active
    ON roster_member(roster_id) WHERE removed_at IS NULL;
