# Pre-existing argo-cli Windows test fixes (`win-fix-pre-existing-test-failures`)

Three pre-existing argo-cli test failures became visible once #2921 made the
argo-cli test target compile on Windows. #2921 flagged them OUT OF SCOPE; this
brief fixes them. All three are **test-fixture portability bugs — no production
change**. Diagnosed on the real host first (NO THEATER), then fixed.

Host: `Latitude5440`, `x86_64-pc-windows-msvc`, cargo/rustc 1.96.0.

## Failures 1 + 2 — SystemTime FILETIME overflow (cu_screenshots sweep fixtures)

`cu_screenshots_tests.rs:90` and `:114` built a far-future sentinel as
`SystemTime::UNIX_EPOCH + Duration::from_secs(i64::MAX as u64 / 4)`. On macOS/
Linux `SystemTime` is a 64-bit-seconds `timespec` (tolerates it); on Windows it
is backed by `FILETIME` (100-ns ticks since 1601), so the add **overflows and
panics inside std `time.rs`**.

**BEFORE (real host):**
```
thread '...sweep_expired_deletes_only_old_rows' panicked at std/src/time.rs:746:
  overflow when adding duration to instant
thread '...sweep_treats_tenants_independently_of_each_other' panicked at std/src/time.rs:746:
  overflow when adding duration to instant
test result: FAILED. 1 passed; 2 failed
```

**Fix (test fixture only):** use a representable far-future value —
`SystemTime::now() + Duration::from_secs(86_400)` (one day ahead is already past
the row's `captured_at`; with the test's 0-second retention the boundary still
deletes the row) — matching the pattern already at `cu_screenshots_tests.rs:143`.
Assertions (`deleted == 1`/`== 2`, list empty) unchanged. **Production
`CuScreenshotStore::sweep_expired` is untouched** — it takes `now` from its
caller and only calls `now.duration_since(UNIX_EPOCH)`; the panic was the test
constructing an unrepresentable `SystemTime`.

## Failure 3 — TOML parse of a backslash path (cu-promotion fixture)

The brief's leading hypothesis was the `python3()` helper. **The real host
proved otherwise:** `python3()` *resolved* (`/c/.../WindowsApps/python3` via
Git-Bash `which`), so the test RAN — and panicked in `echo_manifest`
(`:68`) because the fixture embeds the script path **with backslashes**
(`C:\Users\…\mcp_echo_server.py`) into a TOML basic string, where the TOML
reader treats `\U` (in `\Users`) as an invalid 8-digit unicode escape.

**BEFORE (real host):**
```
thread '...approved_path_honours_cu_promotion_with_flag_off' panicked at
  worker_actions_executor_promotion_tests.rs:68:
  echo manifest parses: Schema("vertical manifest: TOML parse error at line 9,
  column 116 ... command = \"/c/.../python3\", args = [\"C:\Users\operator\...\
  mcp_echo_server.py\"] ... invalid unicode 8-digit hex code")
test result: FAILED. 0 passed; 1 failed
```

**Fix (test fixture only):** forward-slash the `command` and `script` paths in
`echo_manifest` before formatting them into the TOML
(`python.replace('\\', "/")`, `script.to_string_lossy().replace('\\', "/")`).
Forward slashes are valid in TOML basic strings AND valid Windows paths; on unix
this is a no-op. The CU-promotion test does not actually spawn the echo MCP
server (it promotes a `computer_use_native_macos` grant and resolves it by KIND,
which yields `ComputerUseMacOS` regardless of host OS — so the assertion holds);
the manifest only needed to PARSE.

Additionally made `python3()` portable (`which` then `where`) so the test RUNS
(or cleanly SKIPS) on a bare Windows host without Git-Bash, instead of always
skipping. **Skip semantics preserved** — python genuinely absent → `None` →
early-return skip, never a faked pass.

## AFTER (real host — all three green)

```
$ cargo test -p argo-cli --bin argo cu_screenshots::tests::sweep
test handlers::cu_screenshots::tests::sweep_expired_deletes_only_old_rows ... ok
test handlers::cu_screenshots::tests::sweep_treats_tenants_independently_of_each_other ... ok
test handlers::cu_screenshots::tests::sweep_at_exact_retention_boundary_deletes_the_row ... ok
test result: ok. 3 passed; 0 failed

$ cargo test -p argo-cli --bin argo approved_path_honours_cu_promotion_with_flag_off
test handlers::worker_actions_executor_promotion::tests::approved_path_honours_cu_promotion_with_flag_off ... ok
test result: ok. 1 passed; 0 failed
```

Failure 3 **RAN** (python3 resolved on this host) and passed — not skipped.

## Scope / honesty

- All edits are **test-fixture-only** in `cu_screenshots_tests.rs` +
  `worker_actions_executor_promotion_tests.rs`. No production code changed; no
  assertion weakened (the existing assertions all still hold).
- The `echo_manifest` forward-slash fix is the same TOML-escape class catalogued
  in `TEST-FAILURES.md` (the `~10` "TOML fixtures embed un-escaped Windows paths"
  bucket); fixing the shared `echo_manifest` helper also clears the other
  cu-promotion tests that route through it.
- `cargo clippy -p argo-cli --all-targets -- -D warnings` clean on Windows.

_Verified by the Windows fleet (Latitude5440). Test-only changes in cross-platform
files — conductor re-verifies argo-cli green on macOS (the fixes are no-ops on unix)._
