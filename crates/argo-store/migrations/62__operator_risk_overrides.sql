-- Stream `mission-operator-specific-risk-overrides`
-- (MISSION_BACKLOG Section 3 Claim 6 #122) — per-operator overrides
-- of the manifest-driven default risk classification for an action.
--
-- The default policy classifies every action via the bound vertical's
-- `[actions.<name>].risk` declaration. An operator can override that
-- default for one `(action_kind, vertical_id)` pair by writing a row
-- here. `vertical_id == ''` means "apply for every vertical I touch";
-- a non-empty `vertical_id` is a per-vertical override that wins over
-- the all-verticals scope.
--
-- `allow_downgrade INTEGER` is the safety flag: when 0, the resolver
-- refuses any override that would lower the risk below the manifest
-- default. The unconditional accept of upgrades plus the explicit
-- opt-in for downgrades makes a compromised operator account require
-- BOTH the credentials AND the `--allow-downgrade` ack to silently
-- relax approvals.
--
-- Schema-version slot: Rust counter is v58 (next free after v57 =
-- `worker_role_name`); raw filename slot 62 follows the standalone
-- counter (61 = `worker_role_name`). Filename + version counters
-- advance independently per the v55 module's documented convention.

CREATE TABLE IF NOT EXISTS operator_risk_overrides (
    operator_id      TEXT    NOT NULL,
    action_kind      TEXT    NOT NULL,
    vertical_id      TEXT    NOT NULL DEFAULT '',
    risk_level       TEXT    NOT NULL,
    allow_downgrade  INTEGER NOT NULL DEFAULT 0,
    created_at       INTEGER NOT NULL,
    PRIMARY KEY (operator_id, action_kind, vertical_id)
);

CREATE INDEX IF NOT EXISTS idx_operator_risk_overrides_operator
    ON operator_risk_overrides(operator_id);
