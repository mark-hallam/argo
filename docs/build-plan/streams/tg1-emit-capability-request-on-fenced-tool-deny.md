# tg1-emit-capability-request-on-fenced-tool-deny

**Owner**: claude or codex — capability fence / tiered gating; conductor review before merge.
**Depends on**: none (fence + audit shipped).
**Authored against**: origin/main `e6a52811f` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh tg1-emit-capability-request-on-fenced-tool-deny

## Why (operator-directed 2026-06-11: tiered-gating backend)

The locked connect-on-demand UI (`web-capability-grant-card-connect-on-demand`,
design `option-G2-elevated.html`) needs a SIGNAL: when a teammate's run hits a
tool/connector its capability fence denies, TODAY the deny is silent (verified
2026-06-11: `rg 'capability_request|CapabilityRequest' crates/` → zero hits).
The user never learns "Nora needed Salesforce", so connect-on-demand can't
happen. This brief makes the fence deny **legible**: persist a pending
`capability_request` and emit an audit row.

## Scope

1. **Persist the request.** New tenant-scoped table `capability_request`
   (additive migration, next free slot — run `scripts/check-migration-slots.sh`):
   `id, tenant_id, worker_id, tool_or_connector, requested_access (read|write),
   run_id?, status (pending|granted|dismissed), created_utc`. Store accessor in
   `argo-store` or `argo-identity` (follow where `worker`-adjacent tables live;
   grep first — do NOT fork a parallel store).
2. **Emit at the deny point.** Where the runtime fence refuses a tool for a
   worker (the `permits_tool` deny path consumed by the agent-runner turn —
   `crates/argo-capability/src/worker_profile.rs` is the policy; the
   ENFORCEMENT call-site in `argo-agent-runner` is what emits; grep
   `permits_tool(` consumers), insert a pending `capability_request`
   (dedup: one open row per (worker, tool) — re-denies bump a `last_seen_utc`,
   not new rows) + an `AuditKind` row (extend the audit event kind enum ONLY if
   no fitting kind exists — grep `event_kind.rs` first).
3. **List endpoint.** `GET /api/v1/workers/{id}/capability-requests` (gateway),
   tenant-scoped exactly like `routes_worker_outputs.rs` (JWT tenant →
   `(tenant, worker_id)` load → uniform 404 cross-tenant), returns pending
   requests — this is what the SPA grant card polls/reads.
4. NO auto-grant anywhere. Emission is observe-only; granting is tg3.

## Acceptance

- A fenced tool call creates exactly ONE pending row + audit row; second deny
  of the same tool bumps, not duplicates.
- `GET .../capability-requests` returns it for the owning tenant; cross-tenant
  / unknown worker → uniform 404 (test pins both, mirroring
  `routes_worker_outputs_tests.rs`).
- Migration additive + slot-checked; no behavior change for runs that never
  hit the fence.

## Parallel-surface check

`rg 'capability_request|CapabilityRequest|pending_grant' crates/ web/` → zero
today. Reuse the audit sink + the routes_worker_outputs tenant-resolution
pattern; do not fork either.
