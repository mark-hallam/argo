# win-fix-pre-existing-test-failures

**Owner**: windows-fleet — **P2 — fix 3 pre-existing argo-cli Windows test failures flagged OUT OF SCOPE by #2921.**
**Depends on**: none (the argo-cli test target now compiles on Windows post #2921).
**Authored against**: origin/main `648930d73d6d29fda0ee208d63d560e6f74405af` on `2026-06-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-fix-pre-existing-test-failures`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build + VERIFY ON REAL WINDOWS (cargo test for the touched crate), push branch `win-fix-pre-existing-test-failures`, flip the row to IN_REVIEW. Do NOT self-merge — the macOS conductor gates + reviews (these are tests in cross-platform files, so the unix path MUST be re-verified by the reviewer).

## Problem (NO THEATER — from #2921's own body, which flagged these 3 as OUT OF SCOPE)

#2921 (MERGED, `win-cu-executor-wire-into-resolve-executor`) made the **argo-cli
test target compile on Windows** for the first time. Once it compiled, 3 unrelated
PRE-EXISTING failures became visible. #2921 explicitly did NOT fix them (out of
scope) and flagged them for a follow-up brief. They are still on `origin/main`
unchanged (the two test files were last touched by the feature PR #2825, not by
#2921):

1. `cu_screenshots::tests::sweep_expired_deletes_only_old_rows` — panics in std
   `time.rs:746` (SystemTime arithmetic).
   File: `crates/argo-cli/src/handlers/cu_screenshots_tests.rs:90`.
2. `cu_screenshots::tests::sweep_treats_tenants_independently_of_each_other` — same
   SystemTime panic.
   File: `crates/argo-cli/src/handlers/cu_screenshots_tests.rs:114`.
3. `worker_actions_executor_promotion::tests::approved_path_honours_cu_promotion_with_flag_off`
   — failed at a promotion assertion on #2921's host.
   File: `crates/argo-cli/src/handlers/worker_actions_executor_promotion_tests.rs:294`
   (assertion at `:308`).

### Root-cause analysis (verify on the real host before fixing — NO THEATER)

**Failures 1 + 2 — a SystemTime portability bug in the TEST FIXTURE, not production.**
Both tests build a "far future" sentinel:

```rust
let far_future = SystemTime::UNIX_EPOCH + Duration::from_secs(i64::MAX as u64 / 4);
```

(`cu_screenshots_tests.rs:90` and `:114`). On macOS/Linux `SystemTime` is backed by
a 64-bit-seconds `timespec`, so adding ~2.3e18 s is tolerated. On Windows
`SystemTime` is backed by `FILETIME` (100-nanosecond ticks since 1601-01-01);
`UNIX_EPOCH + Duration::from_secs(i64::MAX/4)` **overflows the FILETIME
representation and panics inside std `time.rs`** — exactly the reported panic site.
Production code is fine: `CuScreenshotStore::sweep_expired`
(`cu_screenshots.rs:235`) takes the `now: SystemTime` from its caller and only
calls `now.duration_since(UNIX_EPOCH)`; the panic is the test constructing an
unrepresentable `SystemTime` on Windows. The two tests only need a "now" far
enough past the row's `captured_at` (seconds, written by `unix_seconds_now()`) that
a 0-second retention boundary deletes the row — they do NOT need i64::MAX/4.

**Failure 3 — diagnose on the real host first.** Two candidate root causes; confirm
which one fires on Windows before changing anything (do not assume):
- `python3()` (`worker_actions_executor_promotion_tests.rs:24`) shells out to
  `which python3`. `which` is a unix command; on Windows the equivalent is `where`.
  If `which` is absent the helper returns `None` and the test early-returns (silent
  pass). If a Git-Bash/MSYS `which` is on PATH, python3 resolves and the body runs —
  then the real failure is the assertion.
- The assertion at `:308` requires `exec.kind() == ExecutorKind::ComputerUseMacOS`
  for a `computer_use_native_macos` grant (`cu_grant`, `:281`). The approved-resolve
  production path `promoted_cu_macos_executor`
  (`worker_actions_executor_promotion.rs:195`) constructs `ComputerUseMacOSExecutor`
  by KIND regardless of host OS, so this assertion SHOULD hold on Windows. If it
  does not, capture the actual `exec.kind()` value in the branch output — that is the
  real finding and the fix follows the truth (do not force the assert green).

## Fix

- **Failures 1 + 2** (`cu_screenshots_tests.rs`): replace the overflowing
  `SystemTime::UNIX_EPOCH + Duration::from_secs(i64::MAX as u64 / 4)` sentinel (both
  call sites, `:90` and `:114`) with a **portable** far-future value that is
  representable on Windows FILETIME — e.g. `SystemTime::now() + Duration::from_secs(86_400)`
  (one day ahead is already well past the row's `captured_at`, and the test uses a
  0-second retention so the boundary deletes the row), matching the pattern already
  used at `cu_screenshots_tests.rs:143`. Keep the assertions (`deleted == 1` / `== 2`,
  list empty) UNCHANGED. The fix is in test fixtures only — no production change.
- **Failure 3** (`worker_actions_executor_promotion_tests.rs`): apply the fix that
  matches what you actually observed on the host. Most likely the `python3()` helper:
  make the python3 lookup portable so the test runs (or skips) identically on Windows
  — replace the hardcoded `which` with a cross-platform lookup (try `which` then
  `where`, or probe `python3`/`python` via `--version`, or gate the `which` call with
  `#[cfg(unix)]` and use `where` under `#[cfg(windows)]`). Keep the early-return-skip
  semantics when python is genuinely absent (NO THEATER — a skipped fixture is not a
  pass claim). If, with python resolved, the `ExecutorKind` assertion genuinely fails
  on Windows, fix the TRUE divergence and write the real value into the verifier doc —
  do not paper over it.

Keep every touched file < 500 LOC and every fn < 80 LOC (these are localized test-only
edits; both files are already under budget — keep them there).

## May touch (explicit)

- `crates/argo-cli/src/handlers/cu_screenshots_tests.rs` (the two far-future sentinels)
- `crates/argo-cli/src/handlers/worker_actions_executor_promotion_tests.rs` (the
  `python3()` helper and, only if the real-host evidence demands it, the assertion)
- `docs/skycot/windows/` (a short verifier note, e.g. append to or create
  `docs/skycot/windows/PRE-EXISTING-TEST-FIXES.md` with the before/after cargo-test tails)
- `docs/build-plan/FEATURE_MATRIX.md` (in-band ledger row — see below)

Do NOT touch production code in `cu_screenshots.rs` or
`worker_actions_executor_promotion.rs` unless the real-host evidence proves a genuine
production portability bug (then add it to "May touch" in the PR body and flag it for
the reviewer — production cross-platform changes need unix re-verification).

## Parallel-surface declaration

No new `pub struct`/`enum`/`type` is introduced. `rg 'pub (struct|enum|type)'` over
the touched test files returns nothing (test fixtures only). No parallel abstraction —
this fixes existing tests in place. The portable-python-lookup helper, if added, stays a
private `fn` inside the existing test module (do not create a new shared surface).

## Acceptance criteria (CHECKABLE Windows verifier)

On real Windows (`x86_64-pc-windows-msvc`), all three named tests pass — capture the
cargo tails:

```
cargo test -p argo-cli cu_screenshots::tests::sweep_expired_deletes_only_old_rows
cargo test -p argo-cli cu_screenshots::tests::sweep_treats_tenants_independently_of_each_other
cargo test -p argo-cli worker_actions_executor_promotion::tests::approved_path_honours_cu_promotion_with_flag_off
```

- Failures 1 + 2: each shows `... ok` (no `time.rs` panic) with the `deleted == 1`/`== 2`
  + empty-list assertions intact.
- Failure 3: shows `... ok` — and the verifier doc records whether the test RAN (python3
  resolved) or SKIPPED (python3 genuinely absent on the host); a skip is reported as a
  skip, never claimed as a pass of the assertion (NO THEATER).
- Regression guard: `cargo test -p argo-cli` no longer reports these 3 as failures (paste
  the failure-count delta). `cargo clippy -p argo-cli --all-targets -D warnings` clean on
  Windows.
- The fixes are test-fixture-only (or, if a real production portability bug was found,
  it is named in the PR body for unix re-verification).

## Ledger (in-band, same commit)

Update `docs/build-plan/FEATURE_MATRIX.md` in the SAME commit: note that the argo-cli
Windows test suite's 3 pre-existing failures (SystemTime FILETIME-overflow in the
cu-screenshot sweep fixtures + python3-lookup portability in the cu-promotion test) are
fixed, with the PR/commit SHA and the `argo-cli` crate. Do NOT commit the generated
`docs/skycot/feature-matrix.html` (DEC-043). Branch + IN_REVIEW; conductor reviews + merges.