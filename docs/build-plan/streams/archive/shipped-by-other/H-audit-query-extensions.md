# Stream H (follow-up) — Extend `argo audit query` with H-stack filters

**Owner**: codex. **Depends on**: H-1 (#796), H-3 (#799),
H-orchestrator-persist (#806) merged.

## Scope

The `argo audit query` command exists today but doesn't have
filters for the H-stack-specific event kinds. Operators
debugging "why did this action auto-approve?" or "show me
every handoff for session X" need:

- `--decision auto_approve|ask|block` → filter rows where
  `kind == "auto_approval_decision"` and the decision
  matches
- `--worker <id>` → filter rows where `worker_id == <id>`
- `--session <id>` → filter rows where `session_id == <id>`
- `--escalation` → shorthand for kind=escalation_exhausted
  OR kind=handoff_emitted OR kind=handoff_settled

Compose with existing `--since` / `--until` / `--limit`.

## Files you may touch
- `crates/argo-cli/src/handlers/audit_query.rs` — extend
  with new filters; do NOT replace existing logic
- `crates/argo-cli/src/cli_audit_cmds.rs` — narrowly: add
  the new flag definitions
- `crates/argo-audit/src/query.rs` (or wherever the query
  builder lives) — extend with the WHERE clause additions

## Files you must NOT touch
- The audit row format / chain hash — append-only, fixed
- `crates/argo-audit/src/sink.rs` (write path) — read-only
  side here
- Any other crate

## Acceptance criteria
- [ ] All 4 new filter flags work + compose
- [ ] Invalid `--decision` values rejected with a clear
      message (not silently ignored)
- [ ] `--json` output includes the new filter values in the
      query envelope (so the operator can see what filters
      were applied)
- [ ] Integration test seeding 5+ audit rows of mixed kinds
      + asserting each filter narrows correctly
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every modified file stays < 500 LOC

## Branch + PR
- Branch: `codex-H-audit-query-extensions`
- PR title: `audit: H-stack filters on argo audit query (--decision/--worker/--session/--escalation) (H-audit-query-extensions)`

## Notes
- Mirror the existing `--since` flag's UX: ISO-8601
  timestamps, clear error on parse failure.
- The `--decision` filter assumes audit rows for
  auto-approval decisions are tagged `kind="auto_approval_decision"`
  with a `decision` field in the JSON payload — if not, codex
  documents the gap in the PR body and falls back to a
  text-search on the row body. (Don't change the audit
  schema in this PR.)
