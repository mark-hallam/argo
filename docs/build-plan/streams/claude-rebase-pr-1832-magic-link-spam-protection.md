# claude-rebase-pr-1832-magic-link-spam-protection

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `5582bcba9` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1832-magic-link-spam-protection`.

🟢 **P2 Auditor 5th-consec escalation.** Triage AWAITING-REBASE #1832.

## Scope

30-min read-only triage: rebase + admin-merge if mechanical, OR close-as-superseded with 1-line reason. Per auditor 6th-consecutive escalation.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- the PR's branch
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

- Branch: `claude-rebase-pr-1832-magic-link-spam-protection`
- PR title: `claude-rebase-pr-1832-magic-link-spam-protection`

## Notes for the implementer

- ~varies LOC expected.
- Cite the gap manifest `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` verbatim in the PR body.
- This is a queue-replenishment brief authored as a STUB — feel free to expand acceptance criteria during pre-flight.
