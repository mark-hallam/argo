# ops-ledger-unrowed-stream-lint

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `52dc49988` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-ledger-unrowed-stream-lint`.

🟡 **P2 ledger hygiene.** Source: this tick found dozens of `docs/build-plan/streams/*.md` briefs with no `BUILD_STATUS.md` row. We already have follow-up work for "row points to missing stream"; the inverse drift still slips through and makes replenishment harder than it should be.

## Scope

Add a lint/report helper that compares stream brief ids on disk against live ledger rows and fails when a non-archival stream file has no row.

Treat legacy/historical design docs with uppercase phase ids as allowlistable; the goal is to catch operative brief stubs that were authored but never queued.

## Files you may touch

- `scripts/check-unrowed-stream-briefs.sh` (NEW, ≤240 LOC)
- `scripts/tests/check-unrowed-stream-briefs.bats` (NEW, ≤220 LOC)
- `docs/build-plan/CONTRIBUTING_AGENT.md` — narrowly: mention that authored stream stubs need a ledger row

## Files you must NOT touch

- Existing stream brief contents
- `docs/build-plan/BUILD_STATUS.md` except the standard row flip
- Feature crates under `crates/`

## Acceptance criteria

- [ ] Script reports stream ids present on disk but absent from the ledger
- [ ] Script supports a small inline allowlist for historical/non-queue design docs
- [ ] Tests cover clean, missing-row, and allowlisted cases
- [ ] CONTRIBUTING docs mention the new lint expectation
- [ ] Every authored file <500 LOC

## Branch + PR

- Branch: `codex-ops-ledger-unrowed-stream-lint`
- PR title: `ops: lint stream briefs missing ledger rows (ops-ledger-unrowed-stream-lint)`
