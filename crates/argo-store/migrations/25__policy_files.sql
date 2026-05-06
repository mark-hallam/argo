-- Stream `claude-policy-as-code` — track declarative policy
-- files registered to a workspace via `argo policy apply`.
--
-- Filesystem is still the source of truth (the YAML lives at
-- `<workspace>/policies/<name>.yaml`); this table records WHEN
-- each file was last applied and a body sha so operators can
-- spot drift between disk and the registered version.

CREATE TABLE IF NOT EXISTS policy_files (
    -- Workspace this policy file is registered under (BLOB id
    -- format matching every other workspace_id column).
    workspace_id BLOB NOT NULL,
    -- Policy file name (basename without `.yaml`).
    name TEXT NOT NULL,
    -- SHA-256 of the YAML body at the moment of apply (hex).
    body_sha256 TEXT NOT NULL,
    -- UTC microseconds at which the file was applied. Most
    -- recent apply wins on conflict (UPSERT pattern).
    applied_at_micros INTEGER NOT NULL,
    PRIMARY KEY (workspace_id, name)
);

CREATE INDEX IF NOT EXISTS idx_policy_files_applied_at
    ON policy_files(applied_at_micros DESC);
