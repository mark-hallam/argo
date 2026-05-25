-- Schema migration v81 — add tenant_id to routing_decision.
--
-- Existing rows are not load-bearing yet; the all-zero UUID sentinel
-- keeps the migration additive while new writes supply the real
-- session tenant.
ALTER TABLE routing_decision
  ADD COLUMN tenant_id BLOB NOT NULL DEFAULT X'00000000000000000000000000000000';

CREATE INDEX IF NOT EXISTS idx_routing_decision_tenant_decided_at
  ON routing_decision(tenant_id, decided_at DESC);
