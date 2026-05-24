# ops-ledger-mixed-owner-todo-lint

**Owner**: codex
**Priority**: P2 - ledger hygiene
**Authored against**: origin/main `e8aabba9d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-ledger-mixed-owner-todo-lint`.

## Source

The live ledger header says the Codex worker adapter strictly matches
`owner=codex` on TODO row selection. Rows with `owner=claude or codex` look
available to humans but may be invisible to exact-match worker loops and distort
queue-depth checks.

## Goal

Add a small ledger lint that flags active TODO rows whose owner is not exactly
`claude` or `codex`, with an allowlist only for historical/non-active rows.

## May Touch

- `scripts/` (new or existing BUILD_STATUS lint script)
- `scripts/test/` (new smoke fixture if the repo pattern uses one)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not change worker claim logic.
- Do not rewrite historical ledger rows unless the test fixture requires one.
- Do not add cargo/Rust checks.

## Acceptance

- [ ] Lint exits nonzero for a fixture row with `status=TODO` and `owner=claude or codex`.
- [ ] Lint accepts exact `claude` and exact `codex` TODO rows.
- [ ] Historical `MERGED`, `DROPPED`, or `BLOCKED` rows with non-exact owners do not block the lint unless explicitly configured.
- [ ] README or script usage text says conductors should flip mixed TODOs to an exact owner during replenishment.
- [ ] Shell/script smoke test passes and `git diff --check` passes.

## Notes

This is a conductor-process guard only. It should not run cargo or inspect code.
