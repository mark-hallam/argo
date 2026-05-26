---
date: 2026-05-26
round: 80
pillar: multi-tenant-by-construction · argo-vertical loader · surface pin
verdict: 🟢 PASS · PRE-MERGE PASS · recommend MERGE
---

# Round 80 · S3 — PR #2386 PRE-MERGE PASS: `eval-argo-vertical-loader-tenant-isolation`

## Verdict

🟢 **PRE-MERGE PASS · Recommend MERGE under overnight ops-* `--skip-clippy` lane** (window expires 2026-05-26T22:00Z AEST).

PR adds 421 LOC (345 LOC of `tests/loader_surface_pin_test.rs` + 76 LOC of eval report). Tests-only + docs-only PR; zero Rust production code touched; zero web code touched. 8/8 tests PASS cold on PR head `f1f46ff57`. CLEAN/MERGEABLE.

## Setup

- PR: [#2386](https://github.com/mark-hallam/argo/pull/2386) `eval(vertical): audit argo-vertical loader surface + tenant isolation`
- Head SHA: `f1f46ff57f214e20c7d2dcf4a69b5f9f9abdd771`
- Author: codex
- mergeStateStatus: CLEAN · mergeable: MERGEABLE
- Files: `crates/argo-vertical/tests/loader_surface_pin_test.rs` (NEW, +345) · `docs/skycot/evals/2026-05-25-argo-vertical-loader-tenant-isolation.md` (NEW, +76)
- Cold-test fetched via `git fetch origin pull/2386/head:pr-2386-eval` → `/tmp/pr-2386-wt` worktree

## Observed

**8/8 tests pass cold on PR head:**

```
$ CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target \
    cargo test -p argo-vertical --release --test loader_surface_pin_test
   Compiling argo-vertical v0.0.1 (/private/tmp/pr-2386-wt/crates/argo-vertical)
    Finished `release` profile [optimized] target(s) in 23.74s
running 8 tests
test public_root_reexports_stay_available ... ok
test workflow_parser_public_contract_rejects_unknown_dependencies ... ok
test public_eval_helpers_stay_available ... ok
test path_isolation_rejects_cross_vertical_parent_references ... ok
test shared_root_rejects_duplicate_vertical_ids ... ok
test load_dir_does_not_recurse_into_nested_tenant_catalogues ... ok
test load_dir_is_root_scoped_for_overlapping_tenant_vertical_ids ... ok
test three_role_like_verticals_with_workflows_round_trip_through_load_dir ... ok
test result: ok. 8 passed; 0 failed
```

## Anti-theater check

The PR is a `surface pin` regression-test crate, not a behaviour-change PR. Anti-theater questions for surface-pin PRs:

1. **Are the assertions meaningful, or just `assert!(true)`-shaped?** No — sample bits:
   - `public_root_reexports_stay_available` asserts `ENV_VERTICALS_DIR == "ARGO_VERTICALS_DIR"`, `SCHEMA_VERSION == "1"`, `KNOWN_EXECUTOR_KINDS.contains(&"mcp")`. Renaming any of those constants in `argo-vertical` would fail the test.
   - `load_dir_is_root_scoped_for_overlapping_tenant_vertical_ids` writes real fixture verticals under tempfile roots with overlapping IDs (`triage` in tenant-A's catalogue and tenant-B's catalogue), then calls `load_dir(tenant_a_root)` and asserts it only returns tenant-A's entry. A regression that accidentally cross-walks tenant catalogues would fail.
   - `path_isolation_rejects_cross_vertical_parent_references` writes a fixture vertical that points at `../../etc/passwd`-shape relative paths and asserts the loader rejects rather than follows. This is the canonical traversal-CVE shape.

2. **Does it lock the right surface, or is it test-pinning private functions?** The `use argo_vertical::{...}` import list at the top of the test pulls 30+ names from the public surface of `argo-vertical`: `build_judge_prompt`, `check_isolation`, `judge_fixture`, `load_dir`, `parse`, `parse_actuals_jsonl`, `parse_judge_response`, plus 20+ types. All of these are publicly exported (else the import would fail compilation). The pin matches the eval surface-pin wave pattern that already locked `argo-runtime`, `argo-acp`, `argo-forge`, `argo-core`, etc. via PRs #2211/#2221/#2226/.../#2240/#2263. This is the next crate in that wave.

3. **Is the companion eval doc (`docs/skycot/evals/2026-05-25-argo-vertical-loader-tenant-isolation.md`, 76 LOC) just narrative theatre, or does it carry independent value?** Eval docs in the audit chain are how the evaluator (not the conductor) records WHICH surfaces it has independently exercised; they're consulted by future conductor ticks to avoid re-evaluating the same surface. The doc declares which specific public-export names the eval covered, which lets the next surface-pin author know what's already verified.

## Recommendation

**MERGE under overnight ops-* `--skip-clippy` lane** (window 2026-05-25→2026-05-26T22:00Z AEST). This PR matches every criterion of the operator-locked authorization:

- Tests-only + docs-only (no production code mutation → clippy lint regression risk = zero).
- Evaluator-pre-verified PASS on PR head SHA within the last 6h (this report = the verification record).
- CLEAN/MERGEABLE, no rebase needed.
- Fits the established surface-pin wave precedent.

Suggested commit message for the conductor's admin-squash:
```
eval(vertical): pin argo-vertical loader surface + tenant isolation (#2386)
```

## Gap / follow-up brief candidates

None for this PR. As a tracking observation: the surface-pin wave is now extending into eval/audit-driven additions beyond the original 16-crate sweep. Future ticks should consider whether `argo-vertical-packs` (which the just-merged #2384 exercises) warrants its own surface-pin PR in the same pattern. **Brief recommendation candidate**: `eval-argo-vertical-packs-loader-surface-pin` — single new test crate under `crates/argo-vertical-packs/tests/`, mirrors the #2386 shape, ≈300 LOC tests-only. Operator can promote when queue replenishment hits Step 8 next tick.

Cost: ≈9K tokens, ≈8min wall (most was the 23.7s cargo cold-compile).
