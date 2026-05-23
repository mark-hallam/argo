# eval-argo-store-tenant-bind-sweep

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-store-tenant-bind-sweep`.

🟡 **P1 CLAIM-6.** Sweep 30+ argo-store tables for tenant_id binding.

## Scope

Round-7 verified workflow_definition; round-13 spot-checked 13 of 91 tables (found 2 leaks). Sweep the remaining ~70 tables: for each, identify queries, check tenant_id binding, ship cross-tenant negatives. Output: per-table verdict + fix-brief recommendations.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- crates/argo-store/src/**, eval report at docs/skycot/evals/scheduled/
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

- Branch: `eval-argo-store-tenant-bind-sweep`
- PR title: `eval-argo-store-tenant-bind-sweep`

## Notes for the implementer

- ~400-800 (manager-worker eligible) LOC expected.
- Cite the gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` verbatim in the PR body.
- This is a queue-replenishment brief authored as a STUB — feel free to expand acceptance criteria during pre-flight.
