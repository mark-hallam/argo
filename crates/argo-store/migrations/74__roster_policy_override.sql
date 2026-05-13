-- Stream `claude-roster-store-policy-override-column` — precursor
-- for `mission-roster-resolve-policy` v2.
--
-- Adds a nullable `policy_override_json` column on the `roster`
-- table to carry a sparse `RosterPolicyOverride` payload. Existing
-- rows keep `NULL` (semantically: "no override; resolve via the
-- existing chain"). The column is read by the new
-- `RosterStore::lookup_for_worker` single-row helper and is written
-- by the v2 mission once it lands.
--
-- Additive only: ALTER TABLE ADD COLUMN. SQLite tolerates this on
-- a populated table without a rewrite. Slot rationale: raw SQL
-- filename slots 64, 66-71 are reserved or held for in-flight
-- briefs; 73 is the previous tail (`approval_escalation`), so 74
-- is the next free slot.

ALTER TABLE roster ADD COLUMN policy_override_json TEXT;
