# codex-fn-loc-gate-script — Add missing scripts/check-fn-loc.sh + split 2 LOC violators (F-09 P1)

**Owner**: codex. **Depends on**: nothing.
**Authored against**: origin/main `ad581eca` on `2026-05-16`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh codex-fn-loc-gate-script`.
**Source**: F-09 P1 in `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.

## Scope

P1 hygiene. Two invariants from `CLAUDE.md:56-57` are silently not enforced:

1. **`scripts/check-fn-loc.sh` does not exist.** `scripts/local-ci-check.sh:145` invokes it only if the file exists; otherwise it prints "skip" and returns success. So functions over 80 LOC accumulate silently.

2. **2 files exceed the 500 LOC ceiling per `CLAUDE.md:56`:**
   - `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs` — 635 LOC
   - `crates/argo-tool/src/tools/memory_tests.rs` — 585 LOC

Scope:

- Create `scripts/check-fn-loc.sh` (NEW). Implementation: walk `crates/**/*.rs`, find functions whose body spans > 80 LOC (exclude `#[cfg(test)]` if operator decision is "tests get a separate budget"; otherwise enforce uniformly). Exit non-zero with list of offending functions. Keep the script in pure bash + grep/awk (no extra dependencies).
- Split `routes_audit_feed_tests.rs` (635 LOC) into 2-3 sibling test files by logical grouping (e.g., `routes_audit_feed_query_tests.rs`, `routes_audit_feed_filter_tests.rs`). Each < 400 LOC.
- Split `memory_tests.rs` (585 LOC) into 2 sibling test files by logical grouping (e.g., `memory_legacy_tests.rs`, `memory_namespace_tests.rs`). Each < 400 LOC.
- Run the new script locally; expect it to surface MANY violations (see codex audit's sample list: `input.rs:27`, `routes_dispatch_post.rs:136`, `worker_actions.rs:35`, etc.). DO NOT fix those violators in this brief — that's a follow-up.
- Decide explicitly in PR body: do tests get a separate LOC budget, or is the uniform 80-LOC rule enforced?

LOC: ~250 (script + 2 file splits).

## Files you may touch

- `scripts/check-fn-loc.sh` (NEW) — function-LOC gate script. ≤80 LOC.
- `scripts/local-ci-check.sh` — narrowly: ensure the file is now invoked (it's already invoked conditionally; verify the path is correct). ≤5 modified lines.
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs` — split into siblings. Plus 1-2 new sibling files. ≤400 modified lines combined.
- `crates/argo-tool/src/tools/memory_tests.rs` — split into siblings. Plus 1 new sibling file. ≤400 modified lines combined.
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (or wherever `mod ... tests` declares the test modules) — narrowly: add `pub mod` declarations for new test files if needed. ≤10 modified lines.
- `crates/argo-tool/src/tools/memory.rs` — same. ≤10 modified lines.

## Files you must NOT touch

- Production code in the test-target files — only test code is being split.
- Other LOC violators surfaced by the new script (out of scope; follow-up brief).
- The `.github/workflows/ci.yml.disabled` workflow's `loc-budget` job — that's `codex-ci-workflows-reactivate`'s scope.

## Acceptance criteria

- [ ] `scripts/check-fn-loc.sh` exists and is executable.
- [ ] Running `bash scripts/check-fn-loc.sh` reports all functions > 80 LOC in `crates/**`.
- [ ] Both originally-violating files split into siblings, all under 500 LOC.
- [ ] `find crates -name "*.rs" -not -path "*/target/*" | xargs wc -l | awk '$1>500 && $2!~/total/'` returns NO output.
- [ ] `cargo test -p argo-cli routes_audit_feed` passes (refactor must preserve test coverage).
- [ ] `cargo test -p argo-tool memory` passes (same).
- [ ] `cargo clippy -p argo-cli -p argo-tool --all-targets -- -D warnings` clean.
- [ ] PR body explicitly states the tests-budget decision (uniform 80 LOC OR separate test budget — pick one).
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `codex-fn-loc-gate-script`
- PR title: `tooling: add check-fn-loc.sh + split 2 LOC-ceiling violators (codex-fn-loc-gate-script) — F-09 P1`

## Notes for the implementer

- This brief is **structural** like F-01 — turn on the gate + fix the obvious violations. Don't try to fix all 80-LOC function violations in this brief; that's a follow-up.
- The 2 splits are mechanical: identify logical test groupings (often there are already `// SECTION:` comments or natural cluster boundaries) and move tests across new sibling files.
- After this brief lands AND the F-01 CI re-enable brief lands, the next CI run will fail-fast on any new file > 500 LOC or fn > 80 LOC. That's the goal.
- Tests-budget decision: operator likely prefers "uniform 80-LOC rule" for honesty (`CLAUDE.md:57` doesn't carve out tests). But if implementing that immediately fails too many existing tests, fallback is "separate test budget" but with explicit, documented limit (e.g., 200 LOC per fn for tests). Pick one + document.
