# claude-cli-tests-vertical-gtm-engineer-assets-split — split argo-cli/tests/vertical_gtm_engineer_assets.rs below 500-LOC ceiling

**Owner**: claude. **Depends on**: nothing. **Tier 1 split-prep — pure refactor.**

**Authored against**: origin/main 61782374 on 2026-05-10.

## Why

`crates/argo-cli/tests/vertical_gtm_engineer_assets.rs` is at 468/500 LOC, tight against the hard ceiling. Pre-emptive split keeps headroom for future feature briefs.

## Approach (concrete)

1. **Identify a cohesive sibling group** — the file has 9+ `#[test]` fns covering manifest loading, escalation, skills, eval suite, memory schema, role section, leak isolation. Cohesive split: extract the eval/memory/role cluster (`gtm_engineer_eval_suite_has_ten_required_fixtures`, `gtm_engineer_memory_schema_parses_with_eleven_entries`, `gtm_engineer_role_section_points_at_extant_files`, `gtm_engineer_assets_do_not_leak_into_pr_checker`) into a sibling test file. Leave manifest + escalation + skills tests in the original.
2. **Extract to sibling** `crates/argo-cli/tests/vertical_gtm_engineer_assets_eval.rs`. Each integration-test file is its own binary; duplicate small helpers (`repo_verticals_dir`) as needed or factor into `tests/common/verticals.rs`.
3. **Re-export via `pub(crate) use`** (N/A for tests).
4. **Keep public API unchanged.**
5. **Test.** `cargo clippy -p argo-cli --all-targets -- -D warnings` clean. `cargo test -p argo-cli` green.

## Files you may touch

- `crates/argo-cli/tests/vertical_gtm_engineer_assets.rs` (must drop below 400 LOC)
- `crates/argo-cli/tests/vertical_gtm_engineer_assets_eval.rs` (NEW, ≤ 300 LOC)
- `crates/argo-cli/tests/common/mod.rs` or `crates/argo-cli/tests/common/verticals.rs` (NEW, only if helpers are factored)

## Files you must NOT touch

- Any other source file
- Any test file outside `crates/argo-cli/tests/`

## Acceptance criteria

- [ ] `vertical_gtm_engineer_assets.rs` drops below 400 LOC
- [ ] New sibling created, ≤ 300 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` green
- [ ] Public API unchanged

## Branch + PR

- Branch: `claude-cli-tests-vertical-gtm-engineer-assets-split`
- PR title: `argo-cli: split tests/vertical_gtm_engineer_assets.rs below 500-LOC ceiling (claude-cli-tests-vertical-gtm-engineer-assets-split)`
