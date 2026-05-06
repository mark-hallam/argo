-- Stream `codex-vertical-archive-cli` — vertical lifecycle metadata.
--
-- `vertical_lifecycle.archived_at` is a soft-delete marker for
-- installed verticals. Archived verticals stay on disk so existing
-- sessions and workers can finish, but default operator listings
-- hide them and runtime triggers installed by the CLI reject new
-- bindings.

CREATE TABLE IF NOT EXISTS vertical_lifecycle (
    vertical_id TEXT PRIMARY KEY,
    archived_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_vertical_lifecycle_archived
    ON vertical_lifecycle(archived_at);
