# mission-per-vertical-demo-script — gtm_engineer_learning_loop.sh-shape demo per vertical

**Owner**: claude. **Depends on**: M16-M20 MERGED (vertical packs ship the worker manifests these demos exercise).
**Authored against**: origin/main `7598d5d1` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-per-vertical-demo-script`.

## Scope

MISSION_BACKLOG item 141. The existing `gtm_engineer_learning_loop.sh` (sales-vertical-demo) is the template — a single shell script that drives the worker through hire → first task → learn → refine → AB-promote, with `argo` CLI calls + assertions at each step. This brief generates the per-vertical equivalent for the 5 v1 priority verticals.

LOC: ~800 (5 shell scripts + 1 README).

## Files you may touch

- NEW `verticals/recruiter/demo/learning_loop.sh`.
- NEW `verticals/ap-specialist/demo/learning_loop.sh`.
- NEW `verticals/vendor-manager/demo/learning_loop.sh`.
- NEW `verticals/support-tier1/demo/learning_loop.sh`.
- NEW `verticals/deal-desk/demo/learning_loop.sh`.
- NEW `verticals/README-demo-scripts.md` — index + invocation pattern.

## Files you must NOT touch

- The existing `gtm_engineer_learning_loop.sh` (reference template; copy-pattern, don't refactor); vertical pack manifests (consume, don't change).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] Each shell script runs end-to-end on mock-mode against its vertical (--mock-mode or equivalent).
- [ ] Each exits 0 on green path; exits non-zero with clear assertion message on failure.
- [ ] README-demo-scripts.md indexes all 5 with one-line description.
- [ ] Smoke test: CI runs all 5 against mock-mode on every PR (separate workflow brief if needed; otherwise just doc).
- [ ] No file > 500 LOC.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust-side).
- [ ] `pnpm typecheck` clean (web-side).
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-per-vertical-demo-script`
- PR title: `gtm_engineer_learning_loop.sh-shape demo per vertical (mission-per-vertical-demo-script)`

## Notes for the implementer

- Mission-claim alignment: claim 1 (Teammate, not tool — demonstrable persona behavior per role).
- Authored from MISSION_BACKLOG.md [NEW] item — no prior brief existed; check `rg 'pub (struct|enum|fn) <Name>'` for any new names introduced before declaring (drift prevention rule).
