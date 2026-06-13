# Windows path-handling deep audit

**Brief**: `win-path-canonicalization-deep-audit` (P2). Path handling was the #1
Windows bug class this session — this is the DEEP sweep beyond the already-fixed
cases (capability glob #2913, fs_root #2914, path_guard verbatim #2908,
home/tmp #2904).

**Method**: 5 parallel auditors (one per bug pattern) over `argo-store`,
`argo-tool`, `argo-identity`, `argo-acp`, and `argo-cli/src/handlers`:
string-path-compare · literal-separator-join · glob-os-native-sep ·
verbatim-and-absolute · tmp-home-and-case.

**Machine**: `Latitude5440`, `x86_64-pc-windows-msvc`, cargo 1.96.0.

---

## Fixed in this PR (9 real bugs, all unix-safe)

| # | File | Bug | Fix |
|---|---|---|---|
| 1 | `argo-capability/src/worker_profile.rs::with_fs_root` | **Worker denied its own sandbox on Windows.** fs_root stored via `display()` (backslash) becomes the worker's FsRead/FsWrite GRANT glob `C:\…\maya/**`; globset treats `\` as an escape, so the pattern never matches → the worker can't access its own files. Unfixed sibling of #2913. Reachable: Genesis hire → `store_hire_fs_root` → `turn_helpers_run` → `with_fs_root`. | normalize `\`→`/` (no-op on unix), mirroring `lib.rs:320`. **Regression test added** (`fs_root_with_windows_backslash_separators_still_covers_rooted_path`). |
| 2 | `argo-cli/.../gateway/routes_packs.rs::default_cache_root` | **Gateway packs route hard-errors on native Windows** ("HOME is not set"). | `argo_credentials::dirs_home_dir()` (HOME‖USERPROFILE, #2904). |
| 3 | `argo-cli/.../migrate_encrypt.rs::plan_backup_path` | **DB-encrypt migration hard-errors on Windows** ("$HOME unset"). | `dirs_home_dir()`. |
| 4 | `argo-cli/.../mcp_image_gen.rs` | HOME-only audit path → silently degrades to a CWD-relative path on Windows. | `dirs_home_dir()` before the CWD fallback. |
| 5 | `argo-cli/.../self_update_install.rs::restart_daemon_best_effort` | HOME-only PID-dir → post-update daemon restart silently skipped on Windows. | ARGO_HOME → `dirs_home_dir()/.argo`. |
| 6 | `argo-cli/.../worker_actions_executor_stubs_filesystem.rs::resolve_fs_root` | **Production fs_root fallback `/tmp/argo-fs/<id>`** is a bogus off-drive-root path on Windows. | `std::env::temp_dir().join("argo-fs")`. |
| 7 | `argo-cli/.../gateway/routes_worker_files.rs::resolve_fs_root` | Same `/tmp/argo-fs` fallback (the gateway side — must agree with #6). | `std::env::temp_dir().join("argo-fs")`. |
| 8 | `argo-cli/.../vertical_install.rs` | `.tar` extension compared **case-sensitively** → `pack.TAR` rejected on Windows. | `eq_ignore_ascii_case("tar")` (mirrors `genesis.rs` pdf check). |
| 9 | `argo-cli/.../security_scan_secrets.rs::glob_match` | Allowlist glob splits the path side on `/`, but the path is `Path::display()` (backslash on Windows) → multi-segment allowlist globs never match → secret-scan false positives on Windows CI. | split the path on both `/` and `\`. |

**Verification (real Windows):** `cargo test -p argo-capability` 64+20 pass
(incl. the new regression test), `cargo clippy -p argo-capability --all-targets
-- -D warnings` clean, `cargo build -p argo-cli` Finished (exit 0 — all 8 handler
fixes compile on Windows). argo-cli **test-run** verification awaits #2921 (the
macos-test compile fix) merging; the production fixes are build- + clippy-clean.

---

## DEFERRED — recommend a dedicated brief: `win-store-canonicalize-workspace-root`

**The single most important finding is NOT fixed here, deliberately**, because it
touches the multi-tenant core and has cross-platform + migration nuance that
warrants focused review rather than bundling into a path-cleanup PR.

- **`argo-store/src/workspace.rs` `ensure_workspace` / `lookup_by_root`** resolves
  tenant+workspace by a **raw, case-sensitive SQL string equality** on
  `root_path` (`WHERE root_path = ?`), fed un-canonicalized `cwd.to_string_lossy()`
  by ~30 callers. On Windows the SAME directory yields different cwd strings
  (drive-letter / path casing, `\` vs `/`, or a `\\?\` verbatim prefix), so each
  variant **mints a NEW tenant_id + workspace_id** — breaking documented
  idempotency and **splitting one tenant's data across phantom tenants**
  (a multi-tenant data-integrity bug, Windows-only).
- **Why deferred:** the fix (canonicalize the key centrally in `ensure_workspace`)
  changes the stored `root_path` form on BOTH platforms (`\\?\` verbatim on
  Windows; symlink-resolution on unix), which (a) changes unix behavior and
  (b) makes existing rows mismatch on first run (one-time re-mint). A safe shape
  is a `#[cfg(windows)]`-only normalization (lowercase drive + separator) so unix
  is byte-unchanged and Windows (which has no production workspace rows yet) is
  fixed without migration risk. That deserves its own brief + the conductor's
  unix re-verification of the tenant-resolution path.

---

## Benign-noted (inspected, sound — no change)

- `argo-tool/src/path_guard.rs` verbatim-`..` handling — **already fixed** (#2908):
  chains `requested.components()` (never verbatim) ahead of `joined.components()`.
- `is_absolute` unix-semantics in `store_hire_fs_root.rs` / `genesis_export.rs` —
  **fail-safe**: a leading-`/` path is joined under home / rejected by the
  `Component::Normal`-only allowlist; `..` is rejected lexically. No escape.
- `store_hire_fs_root.rs:253` absolute-fs_root-vs-HOME compare without canonicalize
  — fail-closed (only over-rejects); low priority.
- `strip_prefix` sites (`code_execute.rs`, `backup_create.rs`, `genesis_export.rs`)
  — both sides are read_dir-derived from the same non-canonicalized root; no
  verbatim mismatch.
- Case-sensitive extension/suffix dir-scans over **platform-authored** files
  (`approval_policy.rs` `.toml`, `vertical_attach.rs` `.skill.md`, model/provider/
  cron/health/doctor/connector/eval list handlers) — only bite for hand-placed
  mixed-case files; low priority.
- `argo-acp` has **no filesystem path handling**; `argo-store` `strip_prefix` is on
  scheme strings (`env:`/`file:`/`aws-kms:`), not paths. HTTP route/URL `/`-splits
  and tar entry `/`-joins are correct (wire formats, not FS paths).

## Honest gaps

- argo-cli **test-run** on Windows is blocked until #2921 (macos-test compile fix)
  merges; this PR verifies the argo-cli fixes at build + clippy level only.
- The deferred argo-store canonicalization is the highest-impact residual — track
  it before relying on Windows multi-tenant workspace idempotency.
