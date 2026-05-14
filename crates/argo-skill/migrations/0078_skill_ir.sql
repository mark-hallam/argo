-- Track L Phase 1 — skill IR storage (L2 brief).
--
-- Persists the canonical SkillGraph IR (L1) for every (tenant, skill,
-- version) triple, plus a hash chain so downstream readers can detect
-- tampering: every write records both its own canonical hash
-- (`ir_hash` = sha256(canonical_json(graph))) and the `prev_hash` of
-- the previous row in the same (tenant, skill) family. The chain is
-- broken iff any row's `ir_hash` is recomputed from its `ir_json` and
-- doesn't match the `prev_hash` stored in the next row.
--
-- Multi-tenant invariant (CLAUDE.md): every row carries `tenant_id`,
-- and the primary key prefixes on `tenant_id` so cross-tenant reads
-- cannot collide.
--
-- The migration is idempotent (`IF NOT EXISTS`) so it applies cleanly
-- both to a fresh DB and on top of an existing populated DB. Slot 78
-- is the next free filename above `crates/argo-store/migrations/77`
-- on 2026-05-14; the schema is loaded by the SkillIrStore via
-- `include_str!`, not by the argo-store migration registry, so it
-- doesn't claim a `vNN.rs` counter slot.

CREATE TABLE IF NOT EXISTS skill_ir (
    tenant_id   BLOB NOT NULL,
    skill_id    BLOB NOT NULL,
    version     TEXT NOT NULL,
    ir_json     TEXT NOT NULL,
    ir_hash     BLOB NOT NULL,
    prev_hash   BLOB,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (tenant_id, skill_id, version)
);

CREATE INDEX IF NOT EXISTS idx_skill_ir_latest
    ON skill_ir(tenant_id, skill_id, created_at DESC);
