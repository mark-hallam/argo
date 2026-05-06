-- Stream F follow-up: tenant-scoped shared-team memory.

CREATE TABLE IF NOT EXISTS shared_team_memory (
  id               BLOB PRIMARY KEY DEFAULT (randomblob(16)),
  tenant_id        BLOB NOT NULL DEFAULT (zeroblob(16)),
  author_worker_id BLOB NOT NULL DEFAULT (zeroblob(16)),
  content          TEXT NOT NULL DEFAULT '',
  written_at       INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_shared_team_memory_tenant_written
ON shared_team_memory(tenant_id, written_at, id);

CREATE INDEX IF NOT EXISTS idx_shared_team_memory_author
ON shared_team_memory(tenant_id, author_worker_id, written_at);
