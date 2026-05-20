-- Per-target glob on session grants (mission-pm-approval-target-scope).
--
-- Today add_session_grant(sid, tool, ...) grants the entire (session,
-- tool) pair. Once a worker is approved to call e.g. update_roadmap
-- once, it can update any roadmap line for the rest of the session.
-- This migration adds an optional target_glob column so an operator
-- can scope a grant down to a payload-specific target (e.g.
-- "Q3-2026-*"). NULL = session-global, preserving back-compat.
ALTER TABLE session_grant ADD COLUMN target_glob TEXT;

-- Lookup at consume time is (session_id, tool, target_glob); a
-- non-unique compound index keeps the scan O(active grants) even on
-- sessions with many target-scoped grants.
CREATE INDEX IF NOT EXISTS idx_session_grant_target
    ON session_grant(session_id, tool_name, target_glob);
