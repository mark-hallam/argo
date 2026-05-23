# eval-claim-6-live-cross-tenant-audit-chain-test

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-claim-6-live-cross-tenant-audit-chain-test`.

🟡 **P1 CLAIM-6 LIVE.** Claim 6 LIVE regression test (3 halves).

## Scope

Per round-18 draft: (a) outbound email+Slack approval with signed token + audit; (b) cross-tenant — tenant_b can't see tenant_a approvals (404 not 403); (c) audit-chain — argo audit verify OK on clean DB, FAIL after tampering. Three sub-tests all green = LIVE.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- crates/argo-channels/tests/, crates/argo-audit/tests/
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

- Branch: `eval-claim-6-live-cross-tenant-audit-chain-test`
- PR title: `eval-claim-6-live-cross-tenant-audit-chain-test`

## Notes for the implementer

- ~400-700 LOC expected.
- Cite the gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` verbatim in the PR body.
- This is a queue-replenishment brief authored as a STUB — feel free to expand acceptance criteria during pre-flight.
