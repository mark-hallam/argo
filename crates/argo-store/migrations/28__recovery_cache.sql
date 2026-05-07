-- Stream `claude-error-recovery-suggestions` — cache of LLM-
-- assisted post-mortem suggestions. Keyed on (session_id,
-- chain_hash) so a re-run with the same audit chain is free
-- (no LLM call). New audit rows ⇒ new chain_hash ⇒ cache miss.

CREATE TABLE IF NOT EXISTS recovery_cache (
    session_id TEXT NOT NULL,
    chain_hash TEXT NOT NULL,
    suggestions_json TEXT NOT NULL,
    failure_kind TEXT NOT NULL DEFAULT '',
    cached_at_micros INTEGER NOT NULL,
    PRIMARY KEY (session_id, chain_hash)
);

CREATE INDEX IF NOT EXISTS idx_recovery_cache_session
    ON recovery_cache(session_id);

CREATE INDEX IF NOT EXISTS idx_recovery_cache_cached_at
    ON recovery_cache(cached_at_micros DESC);
