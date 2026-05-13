-- Stream `mission-escalation-on-no-reply` — one-shot approval
-- no-reply reminder stamp.
--
-- `escalation_sent_at` is NULL until the reminder loop successfully
-- dispatches the pre-expiry escalation. Failed dispatches leave it
-- NULL so the next scan can retry. Expiry itself remains owned by
-- the timeout sweeper (separate brief).
--
-- Slot rationale: latest committed raw slot on origin/main is 72
-- (`retro`). Conductor reservations cover 63-70 (audit + planner
-- mission cluster); 71 is pinned to `mission-approval-bundle-ui`
-- (#1494 IN_REVIEW). 73 is the next free raw slot. Versioned
-- migration counter is v62 — next free after v61
-- (`audit_record.tenant_id`). Filename + version counters advance
-- independently per the v55 module's documented convention.
--
-- The supporting index keys the scan query
-- (`list_pending_near_deadline`) on (status, escalation_sent_at,
-- deadline_at) so the predicate matches an index prefix without
-- pulling every approval row.

ALTER TABLE approval_request
    ADD COLUMN escalation_sent_at INTEGER NULL;

CREATE INDEX IF NOT EXISTS idx_approval_request_pending_escalation
    ON approval_request(status, escalation_sent_at, deadline_at);
