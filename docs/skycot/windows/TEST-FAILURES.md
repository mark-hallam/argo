# Windows test-suite — failure inventory

**Brief**: `win-test-suite-run-and-failure-inventory` (P2 — discovery).
**Machine**: `Latitude5440` — Windows 11 Pro, `x86_64-pc-windows-msvc`,
cargo/rustc 1.96.0. Deliverable is the categorized inventory, not green.

## How it was run

Run against **current `main`** (after the Windows P1 wave merged: kryptering
#2905, CU-executor first-build #2906, line-ending/path-sep audit #2908):

```
# Full workspace first — argo-cli now COMPILES (kryptering gated off by #2905),
# but its TEST target has 1 compile error (see below), which aborts the run:
cargo test --workspace --no-fail-fast
#  → error: could not compile `argo-cli` (bin "argo" test)

# So the runtime inventory is the workspace minus argo-cli's tests:
cargo test --workspace --exclude argo-cli --no-fail-fast
```

`cargo test` does **not** accept `--keep-going`, and plain `--no-fail-fast`
aborts the whole run if any *test target* fails to compile — hence the two-step
approach. `argo-executor-computer-use-windows` is now **included** (compiles +
its tests pass) since #2906 merged.

## Result (definitive, current main, excl. argo-cli tests)

| | Count |
|---|---|
| Test-result groups (unit + integration + doctest) | **403** |
| Passing | **397** |
| Failing groups | **6** |
| Individual failing tests | **9** (8 tests + 1 doctest) |
| Test-target compile failures | **1** (argo-cli, below) |

#2908 already fixed the 9 argo-tool path-separator test assertions from the
earlier draft of this inventory — they now pass. The remaining failures are a
tighter, higher-signal set: most need a **production** look, not a test tweak.

---

## Failures (categorized)

Categories: **(a)** real cross-platform bug · **(b)** test assumes unix/non-Windows
· **(c)** windows-env flake.

### argo-capability — `tests::workspace_defaults_scope_user_filesystem_caps` (lib_tests.rs:90)

- **Cat (a) — production, security-adjacent.** `left: ["/tmp/ws\**", …]` vs
  `right: ["/tmp/ws/**", …]`. The default filesystem-capability glob is built
  with the **OS-native separator** (`\` on Windows). **#2908 (test-audit) did NOT
  fix this** — because it isn't a test literal, it's the production glob.
- **Why it matters:** capability globs are matched with `globset`, which matches
  on `/`. A glob containing `\` will **silently fail to match real paths** on
  Windows → a teammate could be wrongly **denied or granted** filesystem access.
- **Fix:** normalize capability path-globs to `/` in production (the glob is a
  matching pattern, not an OS path). **NEW brief: `win-capability-glob-normalize`.**

### argo-identity — fs_root path cluster (3 tests + 1 doctest)

| Test | Cat | Detail |
|---|---|---|
| `store_hire_fs_root::tests::captured_access_folder_persists_to_fs_root_column` (tests:291) | (a) | actual `…\SkycotDemo/Inbox` vs expected `…\SkycotDemo\Inbox` — **mixed separators**: fs_root persistence joins with a literal `/` |
| `store_hire_fs_root::tests::explicit_under_home_draft_fs_root_persists` (tests:310) | (a) | actual `…\SkycotTeammates\ava` vs expected `…\Projects\acme` — resolver picks a different base on Windows |
| `store_hire_fs_root.rs` doctest `resolved_fs_root_from_env` (line 259 → panic at :25) | (a) | `resolved_fs_root_from_env` returns `None` where the doctest expects `Some("C:\Users\operator\Projects\acme")` — env→fs_root resolution differs on Windows |

- **Cat (a) — production path logic.** These are not test literals; the fs_root
  join/normalization + env resolution behaves differently on Windows.
- **Fix:** normalize fs_root path construction (join with `PathBuf`, not literal
  `/`; resolve env consistently). **NEW brief: `win-identity-fs-root-path-portability`.**

### argo-memory — `shared_team::tests::author_stamp_survives_round_trip` (shared_team.rs:209)

- **Cat (a)/(c) — timestamp precision exposed by the Windows clock.**
  `left (round-tripped): …784542Z` vs `right (original): …784542300Z`. The
  original timestamp has **nanosecond** precision (`…300` ns); the round-trip
  serialization truncates to **microseconds**, so the round-trip isn't lossless.
- **Why Windows surfaces it:** the Windows high-resolution clock produced a
  sub-microsecond value (`…300` ns); on macOS/Linux the value is often
  microsecond-aligned (`…000`), hiding the truncation.
- **Fix:** either serialize with nanosecond precision, or normalize/truncate the
  timestamp to the serialized precision *before* the equality check. **NEW brief:
  `win-shared-team-timestamp-precision`** (also a latent unix bug, not Windows-only).

### argo-identity — `store_trust_grade::tests::migration_v46_applies_to_v45_db` (store_trust_grade.rs:198)

- **Cat (a)/(b) — count `94` vs `92` (2 extra).** A schema/migration count.
  **Suspect CRLF**: embedded migration SQL checked out with `\r\n` on Windows can
  change parsing/counting. (Note #2908 was a *test* path-sep audit and would not
  have touched embedded-SQL line endings.) Could also be a real migration bug.
- **Fix:** check whether migration SQL files have a `.gitattributes` LF pin; if
  CRLF is the cause, pin them. **Owning: investigate under a NEW migration/line-
  ending brief.**

### argo-store — `archive_cascade_deletes_only_matching_tenant_child_rows` (tests/worker_archive_cascade_tenant_pin.rs:19)

- **Cat (a)/(c) — `1` row remains vs `0` expected** after a cascade delete.
  SQLite cascade shouldn't differ by OS → either a test-isolation/ordering flake
  **(c)** or a real `PRAGMA foreign_keys`/cascade setup difference **(a)**.
- **Fix:** re-run in isolation to disambiguate. **NEW brief:
  `win-store-archive-cascade-investigate`.**

### argo-tool — `process_registry::tests::{tail_returns_recent_stdout, reap_finished_drops_exited_entries}`

- **Cat (b)/(a) — tests spawn unix shell.** `"for i in 1 2 3; do echo line-$i; done; sleep 0.2"` and `"true"` — Windows' shell can't run them.
- **Caveat:** confirm whether `ProcessRegistry::spawn` hardcodes `sh -c` in
  *production* — if so it's **(a)** (the spawn primitive isn't portable). **NEW
  brief: `win-process-registry-spawn-portability`** (check production first).

### argo-cli — TEST-TARGET COMPILE ERROR (separate from the runtime sweep)

- `worker_actions_executor_cu_macos_tests.rs:71` calls `.dispatch()` on
  `ComputerUseMacOSExecutor`, but that `WorkerActionExecutor::dispatch` impl is
  macOS-gated, so the test doesn't compile on Windows (`E0599`). The argo-cli
  **bin** itself compiles now (kryptering gated off by #2905) — only this one
  macOS-only test target breaks, which blocks `cargo test -p argo-cli`.
- **Cat (b).** **Fix:** `#[cfg(target_os = "macos")]`-gate the macOS-executor test
  module. **Owning: `win-portability-cli-handler-unix-perms`** (argo-cli Windows
  test cleanup, currently claimed) or a small dedicated test-cfg brief.

---

## Summary → recommended follow-up briefs

| Owning brief | Failures | Notes |
|---|---|---|
| **NEW** `win-capability-glob-normalize` | 1 | **(a) production** — globset matches `/`; real RBAC-match risk on Windows |
| **NEW** `win-identity-fs-root-path-portability` | 4 | **(a) production** — fs_root join/env resolution |
| **NEW** `win-shared-team-timestamp-precision` | 1 | **(a)** — lossy timestamp round-trip (latent unix bug too) |
| **NEW** `win-store-archive-cascade-investigate` | 1 | (a)/(c) — flake vs real cascade |
| **NEW** `win-process-registry-spawn-portability` | 2 | (b)/(a) — check production spawn shell |
| migration_v46 (94 vs 92) | 1 | investigate CRLF in embedded SQL (`.gitattributes` LF pin) |
| `win-portability-cli-handler-unix-perms` (claimed) | argo-cli test compile | cfg-gate the macOS-executor test |

Per the brief: **inventory only — no mass-fix.** Note the shift from the first
draft: the path-sep test cluster is **resolved by #2908**; what remains is mostly
**(a) production** issues (capability glob, fs_root paths, timestamp precision)
that a test-literal audit would not catch.

## Honest gaps (NO THEATER)

1. **argo-cli runtime tests uninventoried** — its bin compiles, but the macOS-test
   compile error blocks `cargo test -p argo-cli`. Once that test is cfg-gated,
   re-run `cargo test -p argo-cli` for the real argo-cli runtime inventory.
2. **Single run** — the (c)-suspects (argo-store cascade) seen once; owning briefs
   should re-run in isolation before deciding flake vs real.
3. **(a) vs (c)** for the timestamp + cascade items is provisional; the values
   point at real precision/behavior issues but the owning brief confirms.
