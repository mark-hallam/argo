-- Stream `mission-persona-evolution-audit-revert` (MISSION_BACKLOG
-- Section 3 Claim 1 #46 — per-worker persona evolution with
-- operator-visible diff history and one-step revert).
--
-- `persona_revision` is append-only. Every `argo persona edit` (or
-- programmatic mutation) emits one row per field touched. The
-- `audit_chain_hash` column carries a sha256-based tamper-evidence
-- chain in the same shape as `argo-audit::chain_hash` (parent ||
-- kind || 0x00 || payload), with kind=field and payload=rev_id||
-- 0x00||new_value. Modifying any row's hash breaks the next row's
-- prev_hash check (see `verify_chain` in
-- `crates/argo-store/src/persona_revision.rs`).
--
-- Slot rationale: SQL filename slot 51 reserved for in-flight
-- `mission-escalation-on-no-reply` (approval_request.escalation_
-- sent_at column), slot 50 already taken by `worker_trust_grade`.
-- This brief rolls forward to 52 per the brief's "verify slot ≥
-- v52" directive.
CREATE TABLE IF NOT EXISTS persona_revision (
    rev_id            TEXT PRIMARY KEY,
    worker_id         TEXT NOT NULL,
    revised_at        INTEGER NOT NULL,
    field             TEXT NOT NULL,
    old_value         TEXT NOT NULL,
    new_value         TEXT NOT NULL,
    actor             TEXT NOT NULL,
    audit_chain_hash  TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_persona_revision_worker_revised_at
    ON persona_revision(worker_id, revised_at DESC);
