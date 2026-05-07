# V1.4 — Approval history search CLI

**Owner**: codex. **Depends on**: H-auto-approval merged,
storage layer for approvals stable.

## Why

Operators need to answer questions like "what did we approve
last Friday?", "show every deny on worker `payments-bot`",
"how many approval requests came in this month?". Today the
only path is raw SQL against the local store. Ship a typed
CLI that maps to a single SELECT with WHERE clauses driven by
flags.

## Scope

Add `argo approval history` with these flags:
- `--since <date>` / `--until <date>` (RFC3339 or `7d`/`24h`
  shorthand)
- `--decision <approve|deny|pending|timeout>` (repeatable for
  multi-select)
- `--worker <name>` filter by emitting worker
- `--action-kind <kind>` filter by kind
- `--limit <N>` (default 50, max 1000) — already a project
  pattern, mirror existing list endpoints
- `--format <table|json|markdown>` — markdown family already
  supported across read-side surfaces

Output is a stable column set: `id | requested_at | decided_at |
decision | worker | action_kind | decided_by | latency_secs`.

NOT in scope: full-text search across payload bodies (separate
work). NOT modifying the storage schema.

## Approach

1. NEW `crates/argo-cli/src/handlers/approval_history.rs`
   (~280 LOC):
   - `pub async fn run(args: ApprovalHistoryArgs) -> ArgoResult<()>`
   - `fn build_query(args: &ApprovalHistoryArgs) -> (String, Vec<Param>)`
     — assemble parameterised SQL (NEVER string-concat user input)
   - `fn render(rows: &[Row], format: OutputFormat) -> String`
2. Date parsing: reuse existing `parse_relative_or_rfc3339`
   helper if present (search `crates/argo-cli/src/handlers/`
   first); otherwise inline a small parser supporting `7d`,
   `24h`, `30m`, plain RFC3339.
3. CLI surface in `cli.rs` adds `History(ApprovalHistoryArgs)`
   variant under `ApprovalCmd` (BLANKET CLI ALLOWLIST).

## Files you may touch

- `crates/argo-cli/src/handlers/approval_history.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/approval_history_tests.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod line
- `crates/argo-cli/src/cli.rs` / `main.rs` — narrowly: add
  the subcommand variant + dispatch (BLANKET CLI ALLOWLIST). cli.rs is 49 LOC post-#968 split.
- `crates/argo-cli/src/cli_command_enum.rs` (post-#968: top-level `Command` variants live here — add `Approval` top-level variant here)
- `crates/argo-cli/tests/approval_history_smoke.rs` (NEW) —
  seed 5 approvals, query with mixed filters, verify count

## Files you must NOT touch

- Storage schema or migrations — read-only feature
- `session_approval.rs` decision logic
- Other approval handlers
- `crates/argo-types/` unless adding a tiny output struct

## Acceptance criteria

- [ ] `argo approval history --since 7d` returns approvals
      from the last week
- [ ] `--decision deny --decision timeout` returns BOTH
      categories (OR semantics)
- [ ] `--worker foo` is a strict equality filter
- [ ] `--limit 1001` is clamped to 1000 with a warning
- [ ] `--format json` emits one JSON object per row, NDJSON
- [ ] `--format markdown` emits a GH-flavoured table
- [ ] SQL injection test: `--worker "'; DROP TABLE"` does not
      execute as SQL (parameterised query verified by unit test)
- [ ] Help-smoke test pins every flag
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `codex-approval-history-search-cli`
- PR title: `approval: history search CLI (claude-approval-history-search-cli)`

## Notes for the implementer

- Mirror the column ordering of existing `argo approval list`
  if such a command exists — operators muscle-memory matters.
- Markdown output: use `tabled` with markdown style; do NOT
  hand-roll pipe-separated lines.
- For json/ndjson, emit one record per line (no surrounding
  array) to match other read-side endpoints.
- This brief feeds directly into `codex-approval-audit-export`
  — keep the row struct shareable (same module or a shared
  one in `handlers/approval_row.rs` if natural).
