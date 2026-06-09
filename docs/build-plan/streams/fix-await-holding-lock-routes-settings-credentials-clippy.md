# fix-await-holding-lock-routes-settings-credentials-clippy

**Owner**: claude.
**Authored against**: origin/main `feb5b49d8` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-await-holding-lock-routes-settings-credentials-clippy`.

> Surfaced by DEC-042 **B-3** (#2756 automated rule-capture) as the **#1
> recurring nit** — `routes_settings_credentials` was boundary-noted **41×** in
> `BUILD_STATUS.md`. This is the loop working: B-3 detected it, this brief fixes
> it so it stops being re-noted. Also in operator auto-memory
> (`project_argo_cli_clippy_await_holding_lock_blocker`) as the documented
> `cargo clippy -p argo-cli -D warnings` blocker that "needs a one-line cleanup".

## Why (VERIFIED 2026-06-09, read-only)
`cargo clippy -p argo-cli -D warnings` fails on
`crates/argo-cli/src/handlers/gateway/routes_settings_credentials_tests.rs`
because a `std::sync::Mutex` guard is held across an `.await`
(`clippy::await_holding_lock`): the test grabs the process-wide env serialization
guard `let _g = ENV_LOCK.lock()...` (lines **52** and **84**) and then awaits
(`store().await` line 54/86, `dispatch_get(...).await` line 60/95) while still
holding it. `cargo test` passes — so the blocker is invisible to the test gate
and hides; only `-D warnings` clippy catches it (it has been boundary-noted on
~41 PRs instead of fixed). The guard is HELD ACROSS AWAIT **on purpose** — it
serializes env-var mutation across the async test body, so restructuring to drop
it early would defeat the test. The correct fix is the codebase's already-
established annotation, not a rewrite.

## Grep-before-write
- `rg -n 'ENV_LOCK.lock|\.await|#\[allow\(clippy::await_holding_lock\)\]|#!\[allow' crates/argo-cli/src/handlers/gateway/routes_settings_credentials_tests.rs` — the guard sites (52/84) + their awaits; confirm NO allow is present yet.
- `rg -n 'allow\(clippy::await_holding_lock\)' crates/argo-cli/src/handlers/gateway/` — the ESTABLISHED PATTERN to mirror: `routes_auth_logout_tests.rs` (`#[allow(...)]` per-fn, lines 74/94/112/129), `middleware_session_verify_tests.rs` (87/98/118), `audit_roundtrip_tests.rs` (`#![allow(...)]` module-top, line 22). Use the SAME approach (these are sibling test files with the identical ENV_LOCK-across-await pattern) — do not invent a new one.
- Confirm there is no parallel/competing brief: `rg -n 'await_holding_lock|routes_settings_credentials.*clippy' docs/build-plan/BUILD_STATUS.md docs/build-plan/streams/` (the 41 hits are boundary-notes, not a fix brief).

## Fix (mechanical, established pattern — no logic change)
1. Add the standard annotation to silence the lint for these intentional test
   guards. Prefer the **module-top `#![allow(clippy::await_holding_lock)]`**
   (mirroring `audit_roundtrip_tests.rs:22`) if every guard in the file is the
   same deliberate ENV_LOCK pattern; otherwise per-`#[allow(...)]` on each
   affected `#[tokio::test]` fn (mirroring `routes_auth_logout_tests.rs`). Pick
   whichever matches the file's existing style with the smallest, clearest diff.
2. Add a one-line comment at the allow site explaining WHY (env-serialization
   guard is intentionally held across `.await` to keep env-var mutation
   single-threaded across the async test body) — so a future reader doesn't
   "optimize" the allow away and re-introduce the 41× nit.
3. Do NOT change any test logic, the lock, or the awaits. Annotation + comment only.

## Acceptance (truth-bounded — checkable verifier)
1. `cargo clippy -p argo-cli --all-targets -- -D warnings` (or the repo's
   `scripts/workspace-clippy-fast-fail.sh` scoped to argo-cli) **exits 0** with no
   `await_holding_lock` warning from `routes_settings_credentials_tests.rs`.
   Paste the clean tail.
2. `cargo test -p argo-cli --bin argo routes_settings_credentials` still green
   (the annotation changes no behavior).
3. The diff is annotation + comment only (no logic/await/lock edits) — confirm
   with `git diff --stat` (a handful of lines, one file).
4. File stays < 500 LOC.

## On ship (same commit)
This is a test-only clippy hygiene fix (no shipped-capability change), so no
`FEATURE_MATRIX.md` row is required — BUT update the operator auto-memory pointer
expectation in the PR body: note it CLOSES the
`project_argo_cli_clippy_await_holding_lock_blocker` memory item and the 41×
`routes_settings_credentials` recurring nit (DEC-042 B-3 finding #1). After this
lands, the conductor can drop the standing "boundary-note the argo-cli clippy
blocker" caveat from future PRs.

## Effort
S (one test file, add the established `#[allow(clippy::await_holding_lock)]`
annotation + an explanatory comment; mirrors 3 sibling files; no logic change).
