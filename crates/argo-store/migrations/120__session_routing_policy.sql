-- mission-claim-3-runtime-wiring-v1x — wire TieredRouter into the
-- agent loop. Sessions opt into tier routing by setting this column
-- to a serialized argo_types::RoutingPolicy (JSON); NULL means
-- "no policy, use session.model directly" (back-compat for every
-- existing row).
--
-- Slot 120: reference DDL — applied at runtime by
-- `argo_store::store_open::ensure_session_routing_policy_column`
-- (PRAGMA-detect-then-ALTER, mirroring slot 38 worker_id). The
-- versioned migration runner caps at v70 so first-touch callers
-- bootstrap on demand. Slot 117 was claimed by #1843
-- (`approval_draft_content`); slot 118 by #1846
-- (`workflow_definition`); slot 119 is conductor-reserved for the
-- pending rebase of #1845 (`mission-pm-approval-target-scope`).
-- This brief takes slot 120 to avoid the cascading collision.

ALTER TABLE session ADD COLUMN routing_policy TEXT;
