-- Stream `claude-operator-feedback-channel` (V2.0
-- self-improvement) — operator-supplied feedback per action.
--
-- One row per (action_id, given_at). Multiple votes on the
-- same action are allowed and addressed by the most-recent
-- row (operators sometimes change their mind). The
-- skill-learning loop reads the most-recent vote when
-- weighting candidate emission:
--   vote=up   → bonus weight (lowers the proposal threshold)
--   vote=down → blacklist (skip candidate emission entirely)

CREATE TABLE IF NOT EXISTS operator_feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    action_id TEXT NOT NULL,
    -- `up` or `down` — checked at the application layer
    -- (sqlite has no native enum).
    vote TEXT NOT NULL,
    -- Optional — `''` if the operator skipped the comment.
    comment TEXT NOT NULL DEFAULT '',
    -- Microseconds since epoch.
    given_at INTEGER NOT NULL,
    -- Optional worker id for filtering (`argo feedback list
    -- --worker <id>`). May be NULL for actions whose worker is
    -- unknown.
    worker_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_operator_feedback_action
    ON operator_feedback(action_id, given_at DESC);

CREATE INDEX IF NOT EXISTS idx_operator_feedback_worker
    ON operator_feedback(worker_id, given_at DESC)
    WHERE worker_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_operator_feedback_vote
    ON operator_feedback(vote, given_at DESC);
