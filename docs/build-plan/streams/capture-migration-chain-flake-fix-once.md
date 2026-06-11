# capture-migration-chain-flake-fix-once

**Owner**: claude or codex — Rust test hygiene; conductor review before merge.
**Depends on**: none.
**Authored against**: origin/main `e8b569be4` on `2026-06-12`.
**Validate before claim**: bash scripts/check-brief-freshness.sh capture-migration-chain-flake-fix-once

## Problem (DEC-042 B-3 top actionable finding — the loop's own output)

`scripts/capture-recurring-nits.py` (post-#2835) reports **`migration_chain`
boundary-noted 18×** — the #1 unresolved recurring nit. Workers repeatedly
hit `crates/argo-store/tests/migration_chain.rs` failures, note them as
pre-existing, and move on; the cost recurs on every affected brief. The
flake is NOT in `docs/build-plan/KNOWN_FLAKES.md` (verified 2026-06-12 —
zero grep hits for `migration_chain` / `19__worker_fs_root`), so each new
worker re-discovers and re-triages it. Historical context: the
`19__worker_fs_root.sql` split-on-`;` failure class appears in old ledger
notes as a known migration-test issue.

## Fix (capture = fix once OR make it a documented known flake)

1. Reproduce: run `cargo test -p argo-store --test migration_chain`
   (isolated worktree + dedicated CARGO_TARGET_DIR per swarm hygiene) and
   characterize the failure: deterministic bug vs environment-dependent
   flake (parallelism, tmpdir, keychain, page-size class)?
2. If a real bug with a bounded fix → fix it (additive-migration rules
   apply; see docs/skycot/operator/migrations.md).
3. If genuinely environmental → add the KNOWN_FLAKES.md entry (symptom,
   trigger, why-safe, retry guidance) so boundary-noting stops, AND file
   the root-cause as a follow-up brief if the fix is unbounded.
4. Either way: the next `capture-recurring-nits.py` run after the ledger
   row merges should drop `migration_chain` from actionable findings
   (the #2835 resolved-detection keys off this brief's `capture-` /
   `fix-` row reaching MERGED).

## May touch

- `crates/argo-store/tests/migration_chain.rs` (+ helpers it uses)
- `crates/argo-store/src/migrations/` ONLY if the fix is a real bounded bug
- `docs/build-plan/KNOWN_FLAKES.md`

## Acceptance / verifier

1. Reproduction evidence in the PR body (exact failure + 2-run consistency
   check).
2. Path A: failing case fixed, `cargo test -p argo-store --test
   migration_chain` green 2×. Path B: KNOWN_FLAKES.md entry merged with
   the documented trigger.
3. clippy clean on touched crates.

~30–120 LOC depending on path. **🟡 P3 — kills the swarm's #1 recurring
triage tax; B-3 loop dogfood.**
