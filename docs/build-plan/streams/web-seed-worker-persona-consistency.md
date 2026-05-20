# web-seed-worker-persona-consistency — unify worker personas across team page + /audit + /workers

**Owner**: claude.
**Authored against**: origin/main `50e65e9f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-seed-worker-persona-consistency`.
**Priority**: P2 — wave-4 data-integrity: 3 workers (Sam, Mariah, Nori) have split personas — /team shows one role, /audit shows another.
**Sizing**: ~150-300 LOC (seed-data sweep + canonical role definitions + tests).

## Why this brief exists

From the wave-4 data-integrity audit (`docs/skycot/audit/data-integrity/2026-05-20-wave-4-data-integrity.md`):

> Three workers have split personas across views: Sam = ops on team
> page but AP Specialist in /audit; Mariah = marketplace listings
> but Vendor Manager in /audit; Nori = marketing but RevOps in
> /audit.

The demo runbook stays on Mariah (per the go/no-go verdict) but other
personas may be referenced. The seed needs single-source-of-truth.

## Goals

1. Single canonical persona definition per worker: name + role + vertical + headline + traits.
2. Every seed file that references the worker reads from this canonical source.
3. Verify /team and /audit and /workers/[id]/overview all show the same role for each worker.

## Steps

1. Build `web/lib/sample-personas.ts` with canonical persona objects keyed by worker ID.
2. Per worker: derive their role + vertical + headline + traits from this single source.
3. Refactor sample-team.ts, sample-audit.ts, sample-worker-overview.ts, sample-cohorts.ts to read from sample-personas.ts.
4. Test: assert each persona's role is identical across pages.

## may-touch

- New: `web/lib/sample-personas.ts` (canonical)
- `web/lib/sample-team.ts` (refactor to consume canonical)
- `web/lib/sample-audit.ts`
- `web/lib/sample-worker-overview.ts`
- `web/lib/sample-cohorts.ts`
- `web/lib/sample-worker-activity.ts`
- Tests

## must-NOT-touch

- Worker IDs themselves (they're stable identifiers)
- Audit-row content (the actions that workers took — only fix role/vertical references)

## Acceptance criteria

- [ ] Single canonical persona file exists.
- [ ] Sam = same role on /team, /audit, /workers/wkr_01J0SAM/overview.
- [ ] Mariah = same role.
- [ ] Nori = same role.
- [ ] `pnpm typecheck && pnpm lint && pnpm test` clean.

## References

- Audit: `docs/skycot/audit/data-integrity/2026-05-20-wave-4-data-integrity.md`
