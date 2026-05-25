# Stream fix-agent-runner-legacy-worker-treated-as-lowest-grade-test — fix or register the pre-existing `turn_skills_tests::legacy_worker_treated_as_lowest_grade` red

**Owner**: codex (small Rust test or production fix; ~30-80 LOC).
**Depends on**: none.
**Authored against**: origin/main `be9206ef2` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-agent-runner-legacy-worker-treated-as-lowest-grade-test`.

🟢 **P3.** Source: round-50 evaluator finding (PR #2256 anti-theater verify), carry-forward through r51-r53 (3rd-consecutive round). The test `turn::turn_skills_tests::legacy_worker_treated_as_lowest_grade` at `crates/argo-agent-runner/src/turn_skills_tests.rs:98` is red on origin/main HEAD, pre-dating PR #2256, NOT tracked in `docs/build-plan/KNOWN_FLAKES.md`. Not demo-blocking, but it's been hiding behind "pre-existing" disclosure footnotes in every claim-2/3/6 PR for ~4 days.

## Scope

1. **Reproduce on current origin/main**: `cd /tmp && git clone <repo> agent-runner-legacy-fix && cd agent-runner-legacy-fix && cargo test -p argo-agent-runner --lib turn_skills_tests::legacy_worker_treated_as_lowest_grade --no-run` then run.
2. **Triage**: is this (a) a real bug in `turn_skills.rs` that's been latent for weeks, (b) a stale test asserting old behavior that was deliberately changed, or (c) something flaky (TTL, ordering, race)?
3. **Ship the right fix**:
   - (a) → fix the production code, keep the test as-is.
   - (b) → rewrite the test to assert the new desired behavior, with a PR-body comment about the behavioral shift.
   - (c) → register in `docs/build-plan/KNOWN_FLAKES.md` with reproducer, then either fix-via-retry or `#[ignore = "flaky — see KNOWN_FLAKES"]` until properly fixed.

Default to (a) — most "pre-existing red" cases turn out to be real bugs that nobody triaged. (c) is the escape hatch only if you can prove flakiness with 5+ alternating green/red runs.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: [`crates/argo-agent-runner/src/`]

## Files you may touch

- `crates/argo-agent-runner/src/turn_skills_tests.rs:98` — the failing test.
- `crates/argo-agent-runner/src/turn_skills.rs` — production code under test, if root cause is here.
- `crates/argo-agent-runner/src/turn.rs` — adjacent if the test exercises the broader turn flow.
- `docs/build-plan/KNOWN_FLAKES.md` — registration entry if option (c).

## Tests

- `cargo test -p argo-agent-runner --lib turn_skills_tests::legacy_worker_treated_as_lowest_grade` green (or removed/ignored with documented reason).
- `cargo test -p argo-agent-runner --lib` green overall — no regressions.

## Definition of done

- Test passes OR is registered in KNOWN_FLAKES.md with reproducer + workaround.
- PR body documents: (a) the root cause finding, (b) which option (a/b/c) was taken, (c) verification command output.
- Future PRs no longer carry the "1 pre-existing test failure in turn_skills_tests" footnote.

Notes: don't bundle with unrelated cleanup. Small focused PR.
