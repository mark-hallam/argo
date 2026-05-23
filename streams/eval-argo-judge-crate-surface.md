# eval-argo-judge-crate-surface

**Authored against:** `origin/main` 2af54930f on 2026-05-23 (conductor tick-110).
**Validate before claim:** `bash scripts/check-brief-freshness.sh eval-argo-judge-crate-surface`
**Owner:** codex
**Priority:** 🟢 P3 — substrate audit, no production change
**Source:** mission-vs-code-gap-manifest.md row argo-judge (🔴 NOT AUDITED).

## What to do

Read `crates/argo-judge/` end to end, then write `crates/argo-judge/tests/surface_contract.rs` to pin:

1. All `pub` items in `crates/argo-judge/src/lib.rs` (re-exports + struct fields + enum variants).
2. `Judge` trait shape — method count, arg/return signatures.
3. `JudgeVerdict` enum variant exhaustiveness (exact `match` test that breaks on add/remove).
4. `LlmJudge::with_default_rubric` constructs a `LlmJudge` whose rubric matches the documented thresholds (0.8 / 0.5).
5. Default rubric serde round-trips JSON → Rust → JSON byte-identical.
6. Any other surface that PR #940 (fail-CLOSED gate) load-bears on — go look at #940 for context.

Also: write a short report `docs/skycot/evals/scheduled/2026-05-23/eval-argo-judge-crate-surface.md` covering:
- Surface inventory (count of pub items per category)
- Versioning discipline assessment (pre-1.0 status + known breaking-change sources)
- Recommended fix-briefs (if any 🔴 cells emerge — e.g. unexpected `unwrap`, missing rustdoc, dead pub items)

Sibling pattern: PR #2132 (eval-argo-skill-sdk-surface) is the template.

## May-touch

- `crates/argo-judge/src/lib.rs` (rustdoc only — no behavior change)
- `crates/argo-judge/tests/surface_contract.rs` (new)
- `docs/skycot/evals/scheduled/2026-05-23/eval-argo-judge-crate-surface.md` (new)

## Acceptance

- All 6 surface-contract test groups pass.
- Per-crate clippy clean.
- Workspace clippy clean.
- Report committed.
- No production source touched beyond rustdoc.

## LOC budget

~200-400 LOC (test file + rustdoc + report).
