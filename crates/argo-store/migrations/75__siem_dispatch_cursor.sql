-- Stream `mission-siem-integration` (MISSION_BACKLOG Section 7 #227) —
-- persistent dispatcher cursor for the new `argo-siem` crate.
--
-- The SIEM dispatcher tails `audit_record` ordered by `seq`, batches
-- rows by sink-specific size limits, and ships them to a configured
-- Splunk HEC / Elasticsearch `_bulk` / Datadog Logs endpoint. The
-- cursor records the highest `audit_record.seq` that has been
-- *successfully* shipped per sink, so a crash + restart resumes from
-- `last_seq + 1` rather than replaying the table from zero (which
-- would multiply enterprise SIEM ingest cost) or dropping rows that
-- were buffered but never shipped (which would break the compliance
-- claim).
--
-- A single row per `(backend, endpoint)` tuple is sufficient: each
-- configured sink advances its own cursor independently. v1 ships
-- exactly one sink per gateway (operator picks `backend` in
-- `argo.toml`) but the schema supports multiple cursors so a later
-- fan-out mode is additive.
--
-- Schema-version slot: this file is the canonical reference. The
-- table is created at dispatcher startup via `CREATE TABLE IF NOT
-- EXISTS` from `argo-siem::dispatcher` (the brief's may-touch list
-- does not include `crates/argo-store/src/migrations/` so we
-- intentionally do NOT register a `vN` migration — the dispatcher's
-- idempotent ensure-schema is what actually creates the table at
-- runtime; this SQL file remains the canonical reference and the
-- next versioned migration can fold it in if a future brief
-- centralises the schema).
--
-- Raw filename slot 75 follows slots 73 (`approval_escalation`) and
-- 74 (`roster_policy_override`); see the v55 convention note in
-- v60.rs / 72__retro.sql for why the filename counter and
-- `schema_version` counter advance independently.

CREATE TABLE IF NOT EXISTS siem_dispatch_cursor (
    backend         TEXT NOT NULL,
    endpoint        TEXT NOT NULL,
    last_seq        INTEGER NOT NULL,
    last_shipped_at INTEGER NOT NULL,
    PRIMARY KEY (backend, endpoint)
);
