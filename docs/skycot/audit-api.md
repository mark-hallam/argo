# Audit API — canonical endpoint contract

The SPA's audit surfaces (feed, drill-in, replay) talk to the gateway
through a single canonical set of paths. They are exported as
`AUDIT_ENDPOINT_PATHS` from
[`web/lib/api/audit-detail.ts`](../../web/lib/api/audit-detail.ts) and
pinned by
[`web/tests/surface_contract_audit_paths.test.ts`](../../web/tests/surface_contract_audit_paths.test.ts).

## Canonical paths (3-row contract)

| Method | Path                              | Purpose                       |
| ------ | --------------------------------- | ----------------------------- |
| GET    | `/api/v1/audit`                   | Paginated audit feed          |
| GET    | `/api/v1/audit/{actionId}`        | Single-action detail drill-in |
| POST   | `/api/v1/audit/{actionId}/replay` | Deterministic action replay   |

`{actionId}` is always URL-encoded by `AUDIT_ENDPOINT_PATHS.detail()` /
`AUDIT_ENDPOINT_PATHS.replay()`.

## Deprecated paths (must never reappear)

These older paths were never canonical. They are listed in
`DEPRECATED_AUDIT_ENDPOINT_PATHS` and the surface-contract test fails if
either string reappears as a runtime call path anywhere in the SPA API
client (`web/lib/api/*.ts`).

| Deprecated path         | Replaced by                |
| ----------------------- | -------------------------- |
| `/api/v1/audit/actions` | `/api/v1/audit` (the feed) |
| `/api/v1/audit/events`  | `/api/v1/audit` (the feed) |

## Related contract: the `/api/v1/workers` team envelope

The team roster endpoint returns the canonical envelope
`{ workers: Worker[], total_count: number }`. As of this contract,
`fetchTeam()` in [`web/lib/api/team.ts`](../../web/lib/api/team.ts)
**rejects** a bare `Worker[]` array (the legacy back-compat shape) and any
object missing the `workers` array, throwing `InvalidTeamEnvelope`
(`code: "invalid_team_envelope"`). A legacy gateway or a buggy redeploy
shipping a non-canonical payload is a contract violation, surfaced as an
error rather than silently tolerated.
