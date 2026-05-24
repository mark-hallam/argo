# ops-codex-conductor-handoff-no-cargo-gate-lint

**Owner**: codex
**Priority**: P2 - conductor prompt hygiene
**Authored against**: origin/main `e8aabba9d` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-codex-conductor-handoff-no-cargo-gate-lint`.

## Source

The operator clarified that Codex conductors should not spend tick time running
workspace clippy, cargo compiles, or pre-merge cargo gates. The Codex handoff
prompt was corrected manually; this guard prevents that instruction from drifting
back into the Codex conductor path.

## Goal

Add a lightweight docs/prompt lint that ensures the Codex conductor handoff prompt
does not require cargo/clippy/pre-merge cargo gates as a default conductor action.

## May Touch

- `docs/skycot/operator/codex-handoff-prompt.md`
- `scripts/` (new or existing prompt lint)
- `scripts/test/` (new smoke fixture if the repo pattern uses one)
- `docs/build-plan/BUILD_STATUS.md`

## Must Not Touch

- Do not edit Claude conductor instructions.
- Do not change feature code or worker prompts.
- Do not run or add cargo/clippy checks.

## Acceptance

- [ ] Lint fails if the Codex conductor handoff prompt tells conductors to run `cargo`, workspace clippy, or `scripts/conductor-pre-merge-gate.sh` by default.
- [ ] Lint permits worker-facing verification language outside the Codex conductor handoff prompt.
- [ ] The Codex handoff prompt keeps the no-cargo/no-workspace-clippy conductor boundary explicit.
- [ ] Smoke test passes and `git diff --check` passes.

## Notes

This is intentionally Codex-only. Do not modify Claude conductor instructions.
