# eval-claim-3-live-genesis-to-workflow-run-test

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-claim-3-live-genesis-to-workflow-run-test`.

🟡 **P1 CLAIM-3 LIVE.** Claim 3 LIVE regression test.

## Scope

Per MISSION.md: 'genesis plan → workflow run produces real output'. Test: drive genesis on a plain-English role, save resulting workflow, execute via WorkerSessionRunner (#2089) against mock MCP, assert audit_record + workflow_run + step outcome.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- crates/argo-workflow/tests/claim_3_live_genesis_to_run.rs
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

- Branch: `eval-claim-3-live-genesis-to-workflow-run-test`
- PR title: `eval-claim-3-live-genesis-to-workflow-run-test`

## Notes for the implementer

- ~200-350 LOC expected.
- Cite the gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` verbatim in the PR body.
- This is a queue-replenishment brief authored as a STUB — feel free to expand acceptance criteria during pre-flight.
