# claude-migration-comment-aware-splitter

**Owner**: claude.
**Depends on**: none.
**Authored against**: `origin/main` `d2be56888` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-migration-comment-aware-splitter`.

🔴 **P0 SHIP-BLOCKER — fresh-install regression.** Source: evaluator bootstrap 2026-05-22T02:25 (`docs/skycot/evaluator/scenarios/skill-cross-tenant-leak-blocked-by-migration-bug.md`) + auditor T03-39 (`docs/skycot/audit/scheduled/LATEST.md` §3.1 / §4 P0 #1). PR #1988 introduced `;` characters inside `-- ` SQL comments across migration files; `split_terminator(';')` hands raw English to SQLite and the gateway refuses to start at migration v43. **Every operator who installs SKYCOT today sees the gateway crash.**

## Scope

The migration runners use `split_terminator(';')` on `&'static str` schema constants to feed SQLite statements one-at-a-time. This is naive about `;` characters inside `-- comment` lines, which were introduced when migration authors started writing English explanations like `-- e.g. SELECT *; will be rejected`. Result: SQLite receives `-- e.g. SELECT *` (a comment) then `will be rejected` (a syntax error), aborts the migration, and the boot path errors out at v43.

Confirmed callsites of `split_terminator(';')` on schema constants (this is the bug surface — fix every one):

```
crates/argo-store/src/migrations/v43.rs:23
crates/argo-store/src/migrations/v47.rs:28
crates/argo-store/src/migrations/v51.rs:24
crates/argo-store/src/worker_archive.rs:62
crates/argo-store/src/worker_archive.rs:421
crates/argo-store/src/worker_okr.rs:37
crates/argo-store/src/worker_deps.rs:144
crates/argo-store/src/worker_kpi.rs:89
crates/argo-store/src/learned_plan_template.rs:174
```

(`*_tests.rs` callsites in worker_archive_tests / worker_presence_tests / worker_archive_delete_tests can stay as-is — they feed hand-curated test fixtures, not migrations.)

Fix: introduce a single helper `argo_store::migrations::split_sql_statements(sql: &str) -> impl Iterator<Item = &str>` that:
1. Splits `sql` into lines.
2. For each line, strips `-- ...` from the first occurrence of `--` to end-of-line.
3. Joins the stripped lines back with `\n`.
4. Splits the result on `;` and trims whitespace.
5. Filters out empty statements.

Then replace every `*.split_terminator(';')` callsite above with `split_sql_statements(*)`. Single helper, single unit-test module, all callsites identical pattern.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-store/src/migrations/mod.rs` — add `pub(crate) fn split_sql_statements` (or a new `splitter.rs` sibling module if cleaner).
- `crates/argo-store/src/migrations/v43.rs` — swap `.split_terminator(';')` for `split_sql_statements(...)`.
- `crates/argo-store/src/migrations/v47.rs` — same swap.
- `crates/argo-store/src/migrations/v51.rs` — same swap.
- `crates/argo-store/src/worker_archive.rs` — same swap (2 callsites).
- `crates/argo-store/src/worker_okr.rs` — same swap.
- `crates/argo-store/src/worker_deps.rs` — same swap.
- `crates/argo-store/src/worker_kpi.rs` — same swap.
- `crates/argo-store/src/learned_plan_template.rs` — same swap.
- `NEW: crates/argo-store/src/migrations/splitter_tests.rs` (or co-located `#[cfg(test)] mod tests` inside `splitter.rs`) — unit tests.

## Files you must NOT touch

- Any other crate's source.
- Any migration's `_UP_SQL` constant (this is a fix to the runner, not the SQL).
- Any `*_tests.rs` outside the splitter test module.
- `BUILD_STATUS.md` outside the standard row flip.

## Acceptance criteria

- [ ] `split_sql_statements` helper exists under `argo_store::migrations` and is `pub(crate)`.
- [ ] All 9 callsites listed above use the helper; no `*.split_terminator(';')` remains in production schema-runner code (test fixtures may keep theirs).
- [ ] Unit test: `-- semicolon ; in comment\nCREATE TABLE t (id INTEGER);` yields exactly one statement `CREATE TABLE t (id INTEGER)`.
- [ ] Unit test: trailing semicolon, leading semicolon, empty input, comment-only input — all handled correctly.
- [ ] Unit test: multi-line comment block `-- line one\n-- line two with ;\nSELECT 1;` yields exactly one statement `SELECT 1`.
- [ ] Boot-from-zero smoke (`scripts/smoke-skycot.sh` or equivalent) succeeds against a fresh DB — no v43 crash.
- [ ] `cargo clippy -p argo-store --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-store` passes (incl. new splitter tests).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (final sanity).
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-migration-comment-aware-splitter`
- PR title: `store(migrations): comment-aware SQL splitter (claude-migration-comment-aware-splitter)`

## Notes for the implementer

- This is a P0 fresh-install fix. **Worker priority**: claim ahead of anything else if the queue is mixed.
- Don't try to write a full SQL parser. The helper is line-based, strips `--` comments, then splits on `;`. That's the entire job.
- Watch out for `--` inside string literals (e.g. `'foo--bar'`). The proper SQL spec rule is that `--` only starts a comment when it's NOT inside a quoted string. A simple pragmatic compromise: refuse to strip `--` if the line already contains a `'` before the `--`. Document the limitation in a code comment; the workspace's `_UP_SQL` constants don't currently rely on `--` inside string literals.
- ~200 LOC total expected. Helper ~30 LOC, tests ~80 LOC, callsite swaps ~10 LOC, rest is review-of-existing.
- After implementation, eyeball the v43/v47/v51 `_UP_SQL` constants for the specific `;`-in-comment instances introduced by PR #1988 — add at least one regression test that uses the exact offending comment text. That's the proof the bug is fixed.
