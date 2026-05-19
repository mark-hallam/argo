-- Stream `mission-pm-trust-ratchet-ui` (P2 #11) — read-only view
-- that exposes the audit rows the trust-streak aggregator consumes.
-- The streak count itself is computed in Rust at read time
-- (`crates/argo-store/src/trust_streak.rs`) because SQLite has no
-- ergonomic gap-and-island primitive that handles "reset on denial
-- OR revocation" without a CTE chain that would dwarf the Rust pass.
--
-- The view is the SQL boundary: it filters the audit-record fan-out
-- to the four `approval.*` kinds the aggregator cares about and
-- projects only the columns the read path needs. Downstream
-- consumers MUST treat it as read-only — `payload_json` stays the
-- canonical source of truth, and the view's existence does not
-- change the chain-hash inputs.
--
-- Slot rationale: latest raw slot on origin/main is 121
-- (`skill_reeval_history`); slot 122 is the next free slot.
-- Versioned migration counter is v74, next free after v73
-- (`worker_freeze`). Filename and version counters advance
-- independently per the v55 convention.
--
-- Idempotent: `CREATE VIEW IF NOT EXISTS` keeps re-runs harmless;
-- the down migration drops the view.

CREATE VIEW IF NOT EXISTS trust_streak_event AS
SELECT
    seq,
    id,
    kind,
    worker_id,
    action_type,
    payload_json,
    created_at
FROM audit_record
WHERE kind IN (
    'approval.granted',
    'approval.revoked',
    'approval.consumed',
    'approval.replied'
)
  AND worker_id IS NOT NULL;
