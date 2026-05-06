-- Stream F: three-layer memory storage.
--
-- `argo-memory` applies this additively at runtime so this SQL file can
-- coexist with the Rust migration runner until argo-store adopts SQL-file
-- migrations globally.

ALTER TABLE memory_entry ADD COLUMN layer TEXT NOT NULL DEFAULT 'recent';
ALTER TABLE memory_entry ADD COLUMN tenant_id BLOB;
ALTER TABLE memory_entry ADD COLUMN worker_id BLOB;
ALTER TABLE memory_entry ADD COLUMN memory_scope TEXT;

CREATE INDEX IF NOT EXISTS idx_memory_entry_layer
ON memory_entry(workspace_id, layer, learned_at);

CREATE INDEX IF NOT EXISTS idx_memory_entry_ns_layer
ON memory_entry(tenant_id, worker_id, memory_scope, layer, learned_at);

CREATE VIRTUAL TABLE IF NOT EXISTS memory_entry_fts USING fts5(
  entry_id UNINDEXED,
  body,
  tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TABLE IF NOT EXISTS memory_embedding (
  entry_id BLOB PRIMARY KEY REFERENCES memory_entry(id) ON DELETE CASCADE,
  model TEXT NOT NULL,
  dims INTEGER NOT NULL,
  vector_json TEXT NOT NULL,
  created_at INTEGER NOT NULL
);

-- When the sqlite-vec extension is registered on a connection, argo-memory
-- also attempts to create this virtual table for native vector KNN.
CREATE VIRTUAL TABLE IF NOT EXISTS memory_vec_index USING vec0(
  entry_id TEXT PRIMARY KEY,
  embedding FLOAT[64]
);
