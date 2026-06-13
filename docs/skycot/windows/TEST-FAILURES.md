# Windows test-suite — failure inventory

**Brief**: `win-test-suite-run-and-failure-inventory` (P2 — discovery).
**Machine**: `Latitude5440` — Windows 11 Pro, `x86_64-pc-windows-msvc`,
cargo/rustc 1.96.0. Deliverable is the categorized inventory, not green.

## How it was run

```
cargo test --workspace --exclude argo-cli --exclude argo-executor-computer-use-windows --no-fail-fast
```

**Two crates are excluded** because their compile fixes are in review, not yet
on `main` (so their test targets can't build on this branch):

- `argo-cli` — blocked by the kryptering/PKCS#11 Windows compile error
  (`win-kryptering-cfg-gate-hsm-signing-backend`, PR #2905, in review).
- `argo-executor-computer-use-windows` — fixed in `win-computer-use-executor-first-build`
  (PR #2906, in review).

> `cargo test` does **not** accept `--keep-going` (build-only flag), and plain
> `--no-fail-fast` aborts the whole run if *any* test target fails to compile.
> Re-run **both excluded crates' test suites** once #2905 + #2906 merge to close
> the gap. Everything else compiled — there were **zero test-target compile
> failures** across the rest of the workspace.

## Result

| | Count |
|---|---|
| Test-result groups | **401** (unit + integration + doctest, ~90 crates) |
| Groups passing | **396** |
| Groups failing | **5** |
| Individual failing tests | **17** (16 tests + 1 doctest) |

The workspace is overwhelmingly Windows-test-clean. All 17 failures cluster in
4 crates and fall into 3 root causes.

---

## Failures by crate (categorized)

Categories: **(a)** real cross-platform bug · **(b)** test assumes unix
(path-sep / line-ending / /tmp / perms / shell) · **(c)** windows-env flake.

### argo-tool — 11 failures

| Test | Cat | Detail | Owning brief |
|---|---|---|---|
| `path_guard::tests::resolve_relative_joins_onto_root` | (b) | `"\tmp\w\a.txt"` vs `"/tmp/w/a.txt"` — `resolve_against_root` returns OS-native seps (correct); test literal is unix | `win-line-ending-and-path-sep-test-audit` |
| `path_guard::tests::resolve_absolute_passes_through_unchanged` | (b) | `"\etc\passwd"` vs `"/etc/passwd"` — **also** `"/etc/passwd"` isn't absolute on Windows, so it doesn't pass through; test needs a platform-abs path or `#[cfg(unix)]` | `win-line-ending-and-path-sep-test-audit` |
| `path_guard::tests::resolve_parent_climb_lands_outside_root` | (b) | `"\tmp\escape"` vs `"/tmp/escape"` | `win-line-ending-and-path-sep-test-audit` |
| `path_guard::tests::resolve_curdir_and_nested_climb_normalized` | (b) | `"\tmp\w\a.txt"` vs `"/tmp/w/a.txt"` | `win-line-ending-and-path-sep-test-audit` |
| `tools::file_read::tests::in_ctx_absolute_path_unchanged` | (b) | `"\etc\passwd"` vs `"/etc/passwd"` | `win-line-ending-and-path-sep-test-audit` |
| `tools::file_read::tests::in_ctx_resolves_relative_against_root` | (b) | `"\tmp\w\a.txt"` vs `"/tmp/w/a.txt"` | `win-line-ending-and-path-sep-test-audit` |
| `tools::file_write::tests::in_ctx_absolute_path_unchanged` | (b) | `"\etc\evil"` vs `"/etc/evil"` | `win-line-ending-and-path-sep-test-audit` |
| `tools::file_write::tests::in_ctx_resolves_relative_against_root` | (b) | `"\tmp\w\out.txt"` vs `"/tmp/w/out.txt"` | `win-line-ending-and-path-sep-test-audit` |
| `tools::file_patch::tests::in_ctx_resolves_relative_against_root` | (b) | `"\tmp\w\a.txt"` vs `"/tmp/w/a.txt"` | `win-line-ending-and-path-sep-test-audit` |
| `process_registry::tests::tail_returns_recent_stdout` | (b) | spawns bash syntax `"for i in 1 2 3; do echo line-$i; done; sleep 0.2"` — Windows shell can't run it; stdout lacks `line-1` | **NEW** `win-process-registry-spawn-portability` |
| `process_registry::tests::reap_finished_drops_exited_entries` | (b) | spawns `"true"` (unix command, absent on Windows) → never reaped | **NEW** `win-process-registry-spawn-portability` |

> **process_registry caveat:** verify whether `ProcessRegistry::spawn` hardcodes
> a unix shell (`sh -c`) in *production* — if so the failures are partly **(a)**
> (the spawn primitive itself isn't portable), not just unix-y test commands. The
> NEW brief should check the spawn shell selection before fixing the tests.

### argo-identity — 4 failures (fs_root path cluster + a migration count)

| Test | Cat | Detail | Owning brief |
|---|---|---|---|
| `store_hire_fs_root::tests::captured_access_folder_persists_to_fs_root_column` | (a)/(b) | actual `...\SkycotDemo/Inbox` vs expected `...\SkycotDemo\Inbox` — **mixed separators**: fs_root persistence joins with a literal `/` somewhere; likely a real path-join portability issue, not just a test literal | **NEW** `win-identity-fs-root-path-portability` |
| `store_hire_fs_root::tests::explicit_under_home_draft_fs_root_persists` | (a)/(b) | actual `...\SkycotTeammates\ava` vs expected `...\Projects\acme` — wholly different path; the resolver picks a different base on Windows (home-relative vs configured). Needs investigation | **NEW** `win-identity-fs-root-path-portability` |
| `store_hire_fs_root.rs` doctest `resolved_fs_root_from_env` (line 259) | (a)/(b) | `resolved_fs_root_from_env` returns `None` where the doctest expects `Some("C:\\Users\\operator\\Projects\\acme")` — env-var → fs_root resolution differs on Windows | **NEW** `win-identity-fs-root-path-portability` |
| `store_trust_grade::tests::migration_v46_applies_to_v45_db` | (a)/(b) | column/row count `94` vs `92` (2 extra). **Suspect CRLF**: embedded migration SQL checked out with `\r\n` on Windows can change parsing/counting. Could also be a real migration bug | investigate: `win-line-ending-and-path-sep-test-audit` if CRLF-caused, else **NEW** migration brief |

### argo-store — 1 failure

| Test | Cat | Detail | Owning brief |
|---|---|---|---|
| `archive_cascade_deletes_only_matching_tenant_child_rows` (`tests/worker_archive_cascade_tenant_pin.rs:19`) | (a)/(c) | after cascade delete, `1` row remains vs expected `0`. SQLite cascade shouldn't differ by OS → either a test-isolation/ordering flake **(c)** or a real cascade/`PRAGMA foreign_keys` setup difference **(a)**. Re-run in isolation to disambiguate | **NEW** `win-store-archive-cascade-investigate` |

### argo-capability — 1 failure

| Test | Cat | Detail | Owning brief |
|---|---|---|---|
| `tests::workspace_defaults_scope_user_filesystem_caps` (`lib_tests.rs:90`) | (b) | `["/tmp/ws\\**", …]` vs `["/tmp/ws/**", …]` — default fs capability glob built with OS-native sep | `win-line-ending-and-path-sep-test-audit` |

> **capability-glob caveat (potential (a)):** if the default fs-cap glob is built
> with **backslashes** on Windows, glob matching (globset uses `/`) may silently
> fail to match real paths at runtime — i.e. a Windows user could be denied (or
> wrongly granted) filesystem access. The fix may need to **normalize cap globs
> to `/`** in production, not just adjust the test. Flag for the owning brief to
> decide test-only vs production-fix.

---

## Summary → recommended follow-up briefs

| Owning brief | Failures | Status |
|---|---|---|
| `win-line-ending-and-path-sep-test-audit` (exists, row in ledger) | 10 (argo-tool path-sep ×9, argo-capability glob ×1) + possibly migration_v46 | TODO — head-start notes already in ledger |
| **NEW** `win-process-registry-spawn-portability` | 2 (argo-tool process_registry) | recommend authoring |
| **NEW** `win-identity-fs-root-path-portability` | 4 (argo-identity fs_root cluster) | recommend authoring — likely real path-join bug |
| **NEW** `win-store-archive-cascade-investigate` | 1 (argo-store cascade) | recommend authoring — flake vs real |

Per the brief, this is **inventory only — no mass-fix**. The path-sep cluster is
mechanical (test literals); the argo-identity fs_root + argo-capability glob +
argo-store cascade items each need a look at *production* code, not just tests.

## Honest gaps (NO THEATER)

1. **2 crates not covered** — `argo-cli` (kryptering #2905) and
   `argo-executor-computer-use-windows` (#2906) were excluded because they don't
   compile on this branch yet. Their test suites are **uninventoried**; re-run
   after both PRs merge.
2. **Single run, no flake confirmation** — the (c)-suspect failures
   (argo-store cascade) were seen once; their owning briefs should re-run in
   isolation before deciding flake vs real.
3. **Category (a) vs (b) is provisional** for the argo-identity / argo-capability
   / migration items — they're flagged where production code (not just a test
   literal) may be the culprit; the owning brief confirms.
