# ops-ledger-brief-file-existence-check

**Owner**: claude
**Priority**: P3 - ledger hygiene
**Authored against**: origin/main `50a3ea9f0` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-ledger-brief-file-existence-check`.

## Source

The scheduled auditor report for `2026-05-24T03-40Z` found that `fix-skill-worker-overlay-blob-text-join-mismatch` had a live ledger row for days but no corresponding `docs/build-plan/streams/*.md` file on disk. That left workers with a visible TODO but no claimable brief.

## Goal

Add a repo-local lint that detects active ledger rows whose `Brief: streams/<brief-id>.md` reference points at a missing file, and wire it into the existing brief lint smoke path.

## May Touch

- `scripts/lint-brief.sh`
- `scripts/lint-brief-tests.sh`
- `scripts/check-brief-files-exist.sh` (NEW, if cleaner than growing `lint-brief.sh`)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not rewrite historical ledger rows.
- Do not change worker claim scripts.
- Do not edit feature code.

## Acceptance

- [ ] The check scans `docs/build-plan/BUILD_STATUS.md` and reports every active row that cites `Brief: streams/<id>.md` when the target file is missing.
- [ ] `MERGED`, `DROPPED`, `SUPERSEDED`, and archived historical rows do not fail the check.
- [ ] Rows with multiple status-note paragraphs are handled without depending on fixed column widths.
- [ ] The failure message names the brief id, status, and missing path.
- [ ] `scripts/lint-brief-tests.sh` or an adjacent smoke test covers at least one missing-file failure and one valid active row.
- [ ] The script remains bash-only and fast enough for conductor ticks.

## Notes

Prefer a structured-enough parse of the markdown table over fragile line-number assumptions. The active table is the only required surface for this brief.
