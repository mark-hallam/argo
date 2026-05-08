-- Stream `codex-handoff-resolved-at-schema` — terminal handoff latency support.
ALTER TABLE handoff ADD COLUMN resolved_at INTEGER;

CREATE INDEX IF NOT EXISTS idx_handoff_resolved_at
    ON handoff(resolved_at) WHERE resolved_at IS NOT NULL;
