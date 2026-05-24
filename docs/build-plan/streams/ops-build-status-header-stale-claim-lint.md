# ops-build-status-header-stale-claim-lint

**Owner**: codex.
**Depends on**: none.
**Authored against**: origin/main `52dc49988` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-build-status-header-stale-claim-lint`.

🟡 **P2 header-truth guard.** Source: the top of `BUILD_STATUS.md` still contains at least one stale status claim that contradicts the live ledger (for example, a "pending ledger insertion" note for briefs that are already MERGED). The header is the operator's source of truth; stale prose there is more dangerous than stale comments in old streams.

## Scope

Add a narrow lint helper for explicit brief-id references inside the header block of `BUILD_STATUS.md`:

1. Extract brief ids mentioned before the first table row.
2. Compare them to live row status.
3. Fail when the header text contains obviously stale phrases like `pending ledger insertion` for a brief that already has a row, or `TODO` for a MERGED row.

Keep the first version intentionally narrow and phrase-based; it only needs to catch the concrete stale-claim class already observed.

## Files you may touch

- `scripts/check-build-status-header-claims.sh` (NEW, ≤260 LOC)
- `scripts/tests/check-build-status-header-claims.bats` (NEW, ≤220 LOC)
- `docs/build-plan/BUILD_STATUS.md` — only if the implementer fixes a real stale claim while landing the lint

## Files you must NOT touch

- Stream brief files
- Feature crates under `crates/`
- Other protocol docs except a short note if needed

## Acceptance criteria

- [ ] Script scans the header block only
- [ ] Script catches at least the "pending ledger insertion but row exists" stale-claim case
- [ ] Script catches at least one stale status keyword mismatch against a referenced row
- [ ] Tests cover clean and failing fixtures
- [ ] Every authored file <500 LOC

## Branch + PR

- Branch: `codex-ops-build-status-header-stale-claim-lint`
- PR title: `ops: lint stale header claims in build status (ops-build-status-header-stale-claim-lint)`
