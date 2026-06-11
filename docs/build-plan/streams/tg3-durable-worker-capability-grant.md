# tg3-durable-worker-capability-grant

**Owner**: claude or codex — capability fence / tiered gating; conductor review + adversarial review before merge (loosens the fence, scoped).
**Depends on**: tg1 (capability_request rows exist to grant against). tg2 useful but not blocking.
**Authored against**: origin/main `e6a52811f` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh tg3-durable-worker-capability-grant

## Why (operator-directed 2026-06-11: tiered-gating backend)

Granting today is ONE-SHOT only: run-now `approve_tools` mints session grants
via `filter_to_worker_tool_set` (`routes_worker_run_exec.rs:173-204`,
verified) that die with the session. The locked grant card promises a DURABLE
decision ("Nora can now read Contacts — revocable any time"). Re-approving
every run is the assistant-not-teammate failure mode. This brief adds the
durable per-worker grant the card's **Connect** commits, and revoke.

## Scope

1. **Grant endpoint.** `POST /api/v1/workers/{id}/capability-grants` body
   `{tool_or_connector, access: "read"|"write", request_id?}`: owner-RBAC,
   tenant-scoped worker load (uniform-404 pattern), updates the worker's
   manifest allow-set — REUSE the same derivation the runtime trusts
   (`WorkerCapabilityProfile::derive` consumes `[tools].allowed/denied`):
   the grant APPENDS to `[tools].allowed` (and for connectors, the connector
   scope list) **after** which the ALWAYS-DENIED strip still applies — the
   deny-wins invariant from the Genesis materializer
   (`materialize_tools.rs`: `code_execute/delegate_task/file_patch/process/
   send_message/terminal` can NEVER be granted via this route; test pins it).
   Marks the tg1 `capability_request` row `granted`.
2. **Revoke endpoint.** `DELETE .../capability-grants/{tool}` removes the
   grant (back to fence-denied), marks any request row back to pending? No —
   `dismissed`. Audit row both ways (grantor principal recorded).
3. **Read/write semantics**: map `access` onto the real capability the
   profile derives (e.g. connector read vs write scope, file_read vs
   file_write) — grep `worker_profile.rs` for the existing read/write split;
   do NOT invent a new access taxonomy.
4. **Tier-3 (computer-use) is OUT of scope** — that path stays the #2825
   `executor_promotion` + DEC-051 risk-tier floor; this brief must NOT add a
   second cu-promotion path (parallel-surface rule). Tier-2/tool grants only.

## Acceptance

- Grant → a previously-fenced tool now passes `permits_tool` on the NEXT run
  (integration test: deny → grant → allowed), tenant-isolated.
- Always-denied tools rejected with explicit error even when requested
  (security pin).
- Revoke restores the deny; audit rows carry grantor + scope, never secrets.
- 🔴 ADVERSARIAL REVIEW BEFORE MERGE (fence-loosening surface).

## Parallel-surface check

`rg 'capability.?grant|CapabilityGrant' crates/` → zero today. Reuse
`filter_to_worker_tool_set`'s membership logic, `materialize_tools`'s
always-denied list, and the uniform-404 tenant resolution; do not fork any.
