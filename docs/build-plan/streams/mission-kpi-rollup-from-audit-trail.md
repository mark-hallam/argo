# mission-kpi-rollup-from-audit-trail

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-kpi-rollup-from-audit-trail`.

🟡 **P1 CLAIM-2.** Mariah-card KPIs aren't auto-derived.

## Scope

Per gap manifest: KPIs shown on Mariah-card mockup must be populated from worker's audit-row history. Cron job that walks worker's audit_record rows, aggregates by action_type + outcome, writes to worker_kpi table.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- crates/argo-store/src/worker_kpi.rs, crates/argo-cron/src/, argo-cli handler
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Production source outside the may-touch list.
- Other crates not part of this audit.

## Acceptance criteria

- [ ] Per-brief acceptance (see Scope above).
- [ ] If real cross-tenant leak / broken-wire found: ship as `#[ignore]`'d FINDINGS-ONLY test + name a fix-brief.
- [ ] `cargo clippy -p <affected-crate> --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `mission-kpi-rollup-from-audit-trail`
- PR title: `mission-kpi-rollup-from-audit-trail`

## Notes for the implementer

- ~300-500 LOC expected.
- Cite the gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` verbatim in the PR body.
- This is a queue-replenishment brief authored as a STUB — feel free to expand acceptance criteria during pre-flight.
