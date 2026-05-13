# claude-error-types-flip-ci — flip `check-error-variant-usage.sh` CI guard from warning to fail

**Owner**: claude. **Depends on**: `claude-error-types-rewrite-io` MERGED **AND** `claude-error-types-rewrite-schema` MERGED **AND** `claude-error-types-invariant-strategy` MERGED.
**Authored against**: origin/main `adeb3faf` on `2026-05-13`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-error-types-flip-ci`.

> SPLIT 4 of 4 from `claude-error-types-rewrite-callsites`. Final guard flip — only claim after all 3 sibling rewrite sub-briefs have MERGED.

## Scope

Flip the `scripts/check-error-variant-usage.sh` CI step in `.github/workflows/ci.yml` from warning-only (`continue-on-error: true`) to fail (`continue-on-error: false`). Until the 3 sibling sub-briefs land, the audit script still flags mismatches; the CI guard would fail on every PR. After all 3 land, mismatches should be 0 and the guard becomes useful.

## Files you may touch

- `.github/workflows/ci.yml` — narrowly: ONE `continue-on-error` flip on the error-variant-audit step. ≤3 modified lines.

## Files you must NOT touch

- Anything else. This is the final flip.

## Acceptance criteria

- [ ] CI step `error-variant-audit` (or equivalent name) no longer has `continue-on-error: true`.
- [ ] Pre-claim check: `bash scripts/check-error-variant-usage.sh` reports 0 mismatches on current `main` (proves the 3 sibling briefs have indeed landed cleanly).
- [ ] CI passes on the PR itself.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-error-types-flip-ci`
- PR title: `ci: flip error-variant-audit to fail (claude-error-types-flip-ci)`

## Notes for the implementer

- This brief is intentionally tiny. Its only purpose is to land **after** the rewrites so the CI gate becomes useful.
- If the pre-claim audit script reports any mismatches, BLOCK — that means a sibling sub-brief didn't fully complete.
