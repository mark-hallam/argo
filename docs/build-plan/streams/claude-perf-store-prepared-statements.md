# claude-perf-store-prepared-statements — Audit + enforce sqlx prepared-statement caching

**Owner**: claude. **Depends on**: none.

> sqlx caches prepared statements **per connection** when you use the
> macros (`query!`, `query_as!`) or the `prepare()` API. Ad-hoc
> `sqlx::query(format!("..."))` strings bypass the cache and re-parse
> on every call. This brief audits hot-path query sites + wires a
> small CI guard so the regression doesn't sneak back.

## Why

Statement preparation is cheap once, expensive every time. On the
hottest query paths (message list, audit search, session list) the
parse cost shows up as p99 latency. We need:
1. one-time cleanup of any `format!("SELECT ...")`-style strings
   that should be parameterised
2. a CI script that flags new offences before they merge

## Scope

- **Audit** hot-path query files in `argo-store` and the gateway
  handlers under `argo-cli/src/handlers/`. Convert any
  `sqlx::query(&format!(...))` to either:
  - `sqlx::query!(...)` (compile-time check), or
  - `sqlx::query_as::<_, T>("SELECT ... WHERE x = ?")`
    .bind(value) — i.e. parameterised, prepared, cached
- **NEW** `scripts/check-prepared-statements.sh` — grep-based CI
  guard that fails if a hot-path file contains `format!("SELECT`
  or `format!("INSERT` or `format!("UPDATE` or `format!("DELETE`
- **Modify** CI workflow (`.github/workflows/*.yml` — or whatever
  the project uses) to invoke the new script

## Approach

1. Inventory hot-path files: `crates/argo-store/src/*query*.rs`,
   `crates/argo-cli/src/handlers/audit_*.rs`,
   `crates/argo-cli/src/handlers/session_*.rs`,
   `crates/argo-cli/src/handlers/message_*.rs`.
2. For each file, look for query strings built with `format!`,
   string concatenation, or runtime SQL composition.
3. Convert each to a parameterised `query` / `query_as` call.
   **Exception**: if the SQL legitimately needs identifier
   substitution (table name, column name from user code, not data),
   add a `// audit-allow: identifier substitution` comment. The CI
   script honours that marker.
4. Write the check script in pure bash + grep, no fancy parsers.

## Files you may touch

- `scripts/check-prepared-statements.sh` (NEW, ≤ 80 LOC)
- `.github/workflows/ci.yml` (or whatever the existing CI file is)
  — narrowly: one new step invoking the script
- Hot-path query files identified during the audit. Keep each
  file's diff narrow — convert only the offending lines.

## Files you must NOT touch

- Schema / migrations
- Test files (only product code is in scope)
- Files outside the hot-path inventory above (avoid drive-by edits)

## Acceptance criteria

- [ ] Inventory of hot-path files with offending lines reported in
      PR body
- [ ] All offending lines converted (or marked
      `// audit-allow: ...`)
- [ ] `scripts/check-prepared-statements.sh` passes locally
- [ ] CI step added; the step fails when a fresh
      `format!("SELECT ...")` is introduced
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` clean
- [ ] No file grew > 500 LOC

## Branch + PR

- Branch: `claude-perf-store-prepared-statements`
- PR title: `store: enforce prepared-statement caching, add CI guard (claude-perf-store-prepared-statements)`

## Notes for the implementer

- Don't aim for 100% coverage of the workspace — narrow to the
  hot-path inventory above. Drive-by conversions in cold paths
  bloat the PR without measurable gain.
- The CI script should exit 0 with no offences and exit 1 with a
  pretty list when offences are found (filename, line, snippet).
- If the existing CI lives in `.github/workflows/`, hook the script
  there; if not yet, add a `scripts/dev-safe-check.sh` invocation
  near other lint commands.
