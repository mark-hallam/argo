# win-store-canonicalize-workspace-root

**Owner**: windows-fleet — **P1 WINDOWS MULTI-TENANT DATA-INTEGRITY — deferred by PR #2924 (path deep-audit).**
**Depends on**: none. (Builds on the audit doc shipped in #2924; that PR explicitly named THIS brief-id.)
**Authored against**: origin/main `648930d73d6d29fda0ee208d63d560e6f74405af` on `2026-06-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh win-store-canonicalize-workspace-root`
**Fleet**: WINDOWS (BUILD_STATUS_WINDOWS.md). Build + VERIFY ON REAL WINDOWS (cargo build/test/clippy for `argo-store`). The FIX touches the multi-tenant core, so the macOS conductor ALSO re-verifies the unix tenant-resolution path is byte-unchanged before merge. Push branch `win-store-canonicalize-workspace-root`, flip the row to IN_REVIEW, **do NOT self-merge**.
Source: docs/skycot/windows/PATH-AUDIT.md (DEFERRED section, shipped in PR #2924) — "the single most important finding is NOT fixed here, deliberately ... recommend a dedicated brief `win-store-canonicalize-workspace-root`."

## Problem (with file:line evidence)
`argo-store` resolves a tenant+workspace by a **raw, case-sensitive SQL string
equality** on `root_path`, fed un-canonicalized strings by ~30 callers.

- `crates/argo-store/src/workspace.rs:135` — `lookup_by_root` runs
  `SELECT id, tenant_id FROM workspace WHERE root_path = ? LIMIT 1` and
  `.bind(root_path)` **verbatim** (line 136). No normalization.
- `crates/argo-store/src/workspace.rs:104-128` — on a `lookup_by_root` MISS,
  `ensure_workspace` **mints a NEW `TenantId::new()` + `WorkspaceId::new()`**
  (lines 112-113) and inserts a fresh `workspace` row. The rustdoc at line 102
  promises idempotency "re-running against the same `root_path` reuses the
  existing row" — but "same directory" is NOT "same string" on Windows.
- The `root_path` argument is `cwd.to_string_lossy()` at ~30 call sites, e.g.
  `crates/argo-cli/src/home.rs:47` (`cwd_workspace`),
  `crates/argo-cli/src/handlers/tenant.rs:113`,
  `crates/argo-cli/src/handlers/worker_actions_auto_approval_gates.rs:75`.

**On Windows the SAME directory yields different cwd strings** — `C:\Users\x\proj`
vs `C:/Users/x/proj`, drive-letter casing (`c:` vs `C:`), a trailing separator,
or a `\\?\` verbatim prefix. Each variant misses `lookup_by_root` and **mints a
new tenant_id + workspace_id** — breaking idempotency and **splitting one
tenant's data across phantom tenants** — a multi-tenant data-integrity bug
(Windows-only; unix `to_string_lossy()` is already stable for a given cwd).

## May touch (explicit allow-list — keep `argo-store` < 500 LOC/file, fns < 80 LOC)
- `crates/argo-store/src/workspace.rs` — add a private `fn canonicalize_root_key(&str) -> String` (or sibling `path_key.rs` module if it would push `workspace.rs` over 500 LOC) and call it INSIDE `ensure_workspace` (before `lookup_by_root`) AND inside `lookup_by_root` itself (so both the read key and the stored insert key go through the same normalizer). Normalize what is BOUND to SQL — do not store divergent forms.
- A NEW Windows regression test: `crates/argo-store/src/workspace_root_key_tests.rs` (module-gated, `#[cfg(test)]`) OR a `crates/argo-store/tests/windows_workspace_root_key.rs` integration test.
- `docs/build-plan/FEATURE_MATRIX.md` — update the argo-store / workspace-bootstrap row in-band (status + this brief's commit SHA + crate).
- `docs/skycot/windows/PATH-AUDIT.md` — flip the DEFERRED finding to RESOLVED with the PR/branch (optional but tidy).

## Parallel-surface declaration
`rg 'pub (struct|enum|type|fn) .*(canonical|normalize.*root|root_key)' crates/argo-store/` returns NO existing root-path normalizer in `argo-store` (only `gdpr::normalize_subject` for GDPR subjects + lowercased-hex helpers — unrelated). This brief introduces ONE new **private** helper; it does not add a public type. `argo-tool/src/path_guard.rs` is a different concern (capability sandbox containment, already cfg-handled in #2908) — do NOT reuse or modify it.

## Fix shape (NO THEATER — unix must stay byte-identical)
Per the PATH-AUDIT deferral guidance: make the normalization **`#[cfg(windows)]`-only**.
- On **unix**: `canonicalize_root_key(s) == s` (byte-for-byte passthrough) so EVERY existing unix row + the live demo/cwd workspace keep resolving exactly as today. No migration, no re-mint on unix.
- On **windows** (`#[cfg(windows)]` branch): normalize the comparison key — at minimum (a) replace `\` with `/` (or vice-versa, pick one canonical separator and document it), (b) strip a `\\?\` / `\\\\?\\` verbatim prefix, (c) lowercase the drive letter, (d) trim a single trailing separator. Do NOT call `std::fs::canonicalize` (it hits the filesystem, can fail, and resolves symlinks — out of scope and risky). Pure-string canonicalization only.
- Windows has **no production workspace rows yet** (first Windows build was #2900, days ago), so there is no migration risk on Windows; do not author a data migration.
- If a Windows primitive genuinely can't be normalized safely, the `#[cfg(windows)]` branch says so honestly in a comment — never fake idempotency.

## Acceptance criteria — CHECKABLE Windows verifier
1. On REAL Windows: `cargo test -p argo-store windows_workspace_root_key` (or the in-module test) PASSES, asserting that `ensure_workspace(name, "C:\\Users\\x\\proj")` and `ensure_workspace(name, "C:/Users/x/proj")` (and a `c:` lowercase-drive variant, and a `\\?\C:\Users\x\proj` verbatim variant, and a trailing-`\` variant) all resolve to the **SAME `(TenantId, WorkspaceId)`** — i.e. the second call hits `lookup_by_root` instead of minting a phantom tenant.
2. On REAL Windows: `cargo build -p argo-store` and `cargo clippy -p argo-store -- -D warnings` are CLEAN.
3. The conductor (unix side) re-verifies: `cargo test -p argo-store` is green on macOS/Linux AND the unix `canonicalize_root_key` is a verified no-op (a unix test asserts `canonicalize_root_key("/tmp/x") == "/tmp/x"`), so no existing unix tenant re-mints.
4. `docs/build-plan/FEATURE_MATRIX.md` updated in the SAME commit (status badge + commit SHA + `argo-store`).

## Verify
Branch `win-store-canonicalize-workspace-root`, paste the Windows `cargo test`
tail (showing the same-tenant assertion green) + the unix-noop assertion into
the PR body. Flip the ledger row to IN_REVIEW. Conductor gates + re-verifies
unix unchanged + merges.