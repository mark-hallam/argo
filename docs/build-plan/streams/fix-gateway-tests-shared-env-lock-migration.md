# fix-gateway-tests-shared-env-lock-migration

**Owner**: claude.
**Authored against**: origin/main `7c81986c2` on `2026-06-10`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-gateway-tests-shared-env-lock-migration`.

> Mined from the PR #2752 boundary note (HARD RULE 2): under default
> **parallel** `cargo test -p argo-cli`, ~19 tests across modules the PR did
> not touch (`routes_approvals_json::*`, `resolve_tenant::*`,
> `routes_worker_overview/detail::*`, `routes_genesis_stream::*`, plus the
> env-mutating `routes_channels_voice::*` / `routes_channels_webinbox::*`)
> intermittently FAIL; the same suite is 100% green single-threaded
> (`--test-threads=1`, 2908/0). The 2026-06-02 session triaged these as
> "~20 parallelism flakes, not bugs" and left them. This brief is the root
> fix, so the flake class stops burning gate runs and boundary-notes
> (exactly the recurring-nit pattern DEC-042 B-3 exists to capture).

## Why (VERIFIED 2026-06-10, read-only)
The gateway test modules mutate PROCESS-GLOBAL state (`std::env::set_var`,
`std::env::set_current_dir`) under PER-MODULE locks, so cross-module
parallelism clobbers the shared env/cwd:

- A canonical shared lock ALREADY EXISTS:
  `shared_env_mutex()` / `shared_env_lock()` (`#[cfg(test)]`,
  `crates/argo-cli/src/handlers/gateway/mod.rs` ~lines 482-492 — an
  `Arc<tokio::sync::Mutex<()>>` in a `OnceLock`, returning an OWNED guard
  precisely so it can be held across `.await` without tripping
  `clippy::await_holding_lock`).
- But only **12** files use it (`rg -l 'shared_env_lock'
  crates/argo-cli/src/handlers/gateway/ | wc -l` = 12), while **63** files
  still declare a LOCAL lock (`rg -l 'static ENV_LOCK|fn env_lock'
  crates/argo-cli/src/handlers/gateway/ | wc -l` = 63) and **72** files
  mutate env/cwd. Two modules each holding their OWN lock serialize
  nothing against each other — that is the race.
- The class is NOT tracked in `docs/build-plan/KNOWN_FLAKES.md` (zero hits
  for `set_current_dir|set_var|env_lock`), so every gate run that trips it
  re-litigates "is this my failure?" from scratch.

## Grep-before-write
- `rg -n 'shared_env_lock|shared_env_mutex' crates/argo-cli/src/handlers/gateway/mod.rs` — the canonical lock to ADOPT (do NOT author a second shared lock; do NOT move it).
- `rg -l 'shared_env_lock' crates/argo-cli/src/handlers/gateway/` — the 12 existing adopters; copy the exact call pattern (`super::shared_env_lock().await` from a file-level module; `super::super::...` from an inner `mod tests`) from one of them rather than inventing a path.
- `rg -l 'static ENV_LOCK|fn env_lock' crates/argo-cli/src/handlers/gateway/` — the 63 migration targets.
- `rg -n 'allow\(clippy::await_holding_lock\)' crates/argo-cli/src/handlers/gateway/` — annotations whose sole reason is a LOCAL `std::sync::Mutex` guard held across `.await` (e.g. `routes_settings_credentials_tests.rs` module-top allow from #2757, `routes_auth_logout_tests.rs`, `middleware_session_verify_tests.rs`, `audit_roundtrip_tests.rs`); after migrating to the tokio owned guard these allows become dead and should be dropped IN THE SAME FILE'S diff.

## Fix (mechanical, single pattern, no test-logic changes)
1. In each of the 63 local-lock modules: delete the local
   `static ENV_LOCK: ...` (or local `fn env_lock()`), and acquire the
   shared lock instead:
   - async tests (`#[tokio::test]`): `let _g = super::shared_env_lock().await;`
   - sync tests (`#[test]`, no runtime): `let _g = super::shared_env_mutex().blocking_lock();`
     (tokio's `blocking_lock` is callable outside a runtime; do NOT call it
     inside one).
2. Where the local std Mutex was the ONLY reason for an
   `#[allow(clippy::await_holding_lock)]` / module-top `#![allow(...)]`,
   remove the now-dead allow (the tokio `OwnedMutexGuard` across `.await`
   is clippy-clean).
3. If a module's local lock guards something OTHER than process-global
   env/cwd (rare): keep it, add a one-line justification comment, and list
   the file in the PR body. Everything else migrates.
4. Add a `KNOWN_FLAKES.md` entry ONLY if any racer remains after migration
   (the goal is the entry is unnecessary); otherwise add a short
   "RESOLVED 2026-06: gateway env/cwd races" note so future triage finds
   the fix.
5. Zero changes to assertions, fixtures, or test bodies beyond the lock
   acquisition lines. This brief is lock-plumbing only.

## Acceptance (truth-bounded — checkable verifier)
1. `rg -l 'static ENV_LOCK|fn env_lock' crates/argo-cli/src/handlers/gateway/`
   returns ZERO files (or only the justified keepers from Fix-3, each named
   in the PR body with its one-line reason).
2. `cargo test -p argo-cli` at DEFAULT parallelism passes **2× consecutively**
   (this is the load-bearing proof — the racing modules named above
   included); paste both tails. If a non-env-related pre-existing flake
   fires (check `KNOWN_FLAKES.md`), note it per HARD RULE 2 and rerun.
3. Clippy on touched files: no NEW warnings; specifically NO
   `await_holding_lock` reappears anywhere in the gateway test files (the
   shared tokio guard is the established clean pattern).
4. Diff review: lock-acquisition lines + dead-allow removals only (verify
   with `git diff --stat` — wide but shallow; ~63 files, a few lines each).
5. All files < 500 LOC (these are existing files; the diff only removes a
   static and edits a let — no file grows materially).

## On ship (same commit)
Test-hygiene fix (no shipped-capability change) — no FEATURE_MATRIX row.
PR body: note it roots-out the PR #2752 boundary-noted ~19-test parallel
flake class and retires the per-module `#[allow(clippy::await_holding_lock)]`
pattern in migrated files; flag to the conductor that future
`--test-threads=1` workarounds for THIS class should no longer be needed.

## Effort
M (mechanical, wide-but-shallow: ~63 files × 2-5 lines; one cargo-test
verification cycle dominates wall time; no logic design).
