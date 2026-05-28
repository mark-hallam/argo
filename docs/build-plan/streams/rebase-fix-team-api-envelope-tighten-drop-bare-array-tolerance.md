# rebase-fix-team-api-envelope-tighten-drop-bare-array-tolerance

Re-implement the original brief
`streams/fix-team-api-envelope-tighten-drop-bare-array-tolerance.md` against
current `origin/main` HEAD.

## Original PR

PR #2354 (closed for re-author) — `team: require canonical envelope response`,
created 2026-05-26 against an older `main`. Conflicted on:

- `web/lib/api/audit-detail.ts` — main has evolved with a new
  `GatewayDetailEnvelope`, `AuditDetailWithDelegation`, and a delegation-chain
  reshape (`isDemoLive`, new types). The PR's added `AUDIT_ENDPOINT_PATHS`
  constant block needs to land alongside that shape, not against the older
  shape the PR was authored against.
- `web/lib/api/team.test.ts` + `web/lib/api/team.ts` — main keeps
  back-compat tolerance for a bare-array `/api/v1/workers` response with a
  `malformed_envelope` ApiError on a missing `workers` field. The PR wants
  to swap that to an `InvalidTeamEnvelope` class + drop the bare-array path.

## Authored against: origin/main `a5df90c41` on 2026-05-29

## Validate before claim: bash scripts/check-brief-freshness.sh rebase-fix-team-api-envelope-tighten-drop-bare-array-tolerance

## Bug is still load-bearing

Confirmed on `a5df90c41`:

- `web/lib/api/team.ts` lines 79-101 still call
  `apiGet<{ workers: Worker[]; total_count?: number } | Worker[]>` and have
  the `Array.isArray(response)` branch returning the bare-array verbatim.
  This means a legacy gateway / a buggy redeploy can still ship a bare-array
  through the SPA silently. Tightening to envelope-only is still the
  correct contract.
- `web/lib/api/audit-detail.ts` has no `AUDIT_ENDPOINT_PATHS` export and no
  inline JSDoc table of canonical audit paths. The deprecated
  `/api/v1/audit/actions` + `/api/v1/audit/events` paths are not pinned by
  a surface-contract test.

## Acceptance (same as original brief)

- `fetchTeam()` accepts only the canonical
  `{ workers: Worker[], total_count: number }` envelope. Bare-array
  responses are rejected.
- A new `InvalidTeamEnvelope extends ApiError` (with `code:
  "invalid_team_envelope"`) replaces the `malformed_envelope` `ApiError`
  path; tests pin the rejection.
- `web/lib/api/audit-detail.ts` exports `AUDIT_ENDPOINT_PATHS` and uses it
  in the call site. The JSDoc table documents the canonical 3-row
  contract + the two deprecated paths.
- A new `web/tests/surface_contract_audit_paths.test.ts` pins the
  canonical paths against `AUDIT_ENDPOINT_PATHS` and asserts the
  deprecated entries never reappear in the API client.
- A new `docs/skycot/audit-api.md` records the canonical 3-row contract
  for future readers.

## Shape of main on touched files (current state to merge into)

- `web/lib/api/audit-detail.ts` (current) has `import { isDemoLive, isDemoMode }
  from "@/lib/skycot/demo-mode"`, a `GatewayDetailEnvelope` interface,
  `AuditDetailWithDelegation` + `DelegationStep` types, and a
  `withDelegation()` helper. The new `AUDIT_ENDPOINT_PATHS` constant goes
  near the imports; the call-site swap (line 78 in the old PR) is
  effectively at the `apiGet` call inside `fetchAuditDetail`.
- `web/lib/api/team.ts` (current) at lines 79-101 has the
  `apiGet<{ workers: Worker[]; total_count?: number } | Worker[]>` shape and
  the `malformed_envelope` ApiError throw. The rewrite drops the
  `Array.isArray(response)` branch entirely and throws
  `InvalidTeamEnvelope` instead of `ApiError(..., "malformed_envelope", ...)`.

## Notes for the worker

- Do NOT regress the `findWorkerByName()` defence-in-depth lookup (lines
  68-83 in `team.ts`) — it calls `fetchTeam()` and swallows errors. The
  swallow stays.
- The strict-production fallback paths in both files must stay intact
  (`isStrictProduction()` is the operative seam).
- Keep `docs/skycot/audit-api.md` as a new file (matches the original
  PR's intent).
