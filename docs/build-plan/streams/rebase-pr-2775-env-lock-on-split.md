# rebase-pr-2775-env-lock-on-split

Authored against: origin/main ca46b97f1e42d9d093d275763b55a04add3d65c3 on 2026-06-10
**Validate before claim**: bash scripts/check-brief-freshness.sh rebase-pr-2775-env-lock-on-split

## Goal

Rebase PR #2775 (`fix-gateway-tests-shared-env-lock-migration`, 65 files
migrating per-module `ENV_LOCK` statics to the canonical `shared_env_lock`)
onto current main, re-applying the migration across `routes_duties_tests.rs`'s
new split layout. Intent re-validated 2026-06-10 and STILL load-bearing
(the ~19-test parallel-flake class persists).

## Why now

#2775 touched `routes_duties_tests.rs`; #2774 then split it into
`routes_duties_tests.rs` + `routes_duties_tests2.rs` + `routes_duties_test_helpers.rs`.
The env-lock changes to the old monolithic file must be re-applied across the
three new files. Structural conflict — needs the migration's context.

## How

1. Fetch + rebase `claude-fix-gateway-tests-shared-env-lock-migration` on
   origin/main.
2. For `routes_duties_tests.rs`: the `static ENV_LOCK` → `shared_env_lock`
   change now applies to wherever the affected tests landed across the 3 split
   files. Apply `super::shared_env_lock().await` /
   `shared_env_mutex().blocking_lock()` to each, drop the now-dead
   `#[allow(clippy::await_holding_lock)]` in migrated files.
3. Other 64 files should rebase cleanly (not touched by the split).
4. Force-push with lease.

## Acceptance / verifier

1. `rg -l 'static ENV_LOCK' crates/argo-cli/src/handlers/gateway/` returns 0
   (all migrated).
2. `cargo test -p argo-cli` at DEFAULT parallelism green 2× consecutively.
3. Full gate `--pr 2775` PASS.

## Size / owner

Rebase + resolve ~3 split files. Owner: claude. 🟡 P2 test reliability.
