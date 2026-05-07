# V1.4 — Approval audit export

**Owner**: codex. **Depends on**: claude-approval-history-search-cli
(shares the row struct; can land before or after, but row
shape MUST match).

## Why

Compliance and security review need exportable approval logs:
quarterly SOC2-style audits, post-incident timelines, ad-hoc
"give me every approval the bot made on the prod cluster last
week" pulls. Today operators screenshot the terminal, which
neither scales nor verifies.

## Scope

Add `argo approval export` that writes the same row set as
`argo approval history` to a file (or stdout) in CSV or JSON
form. Same filters, same date semantics. Adds `--out <path>`,
`--format <csv|json|jsonl>`, and an integrity field
(`row_count`, `sha256` of rendered body) printed to stderr at
the end so downstream tooling can verify the file wasn't
truncated.

CSV columns (stable order): `id, requested_at, decided_at,
decision, worker, action_kind, decided_by, latency_secs,
payload_excerpt` (excerpt = first 80 chars of payload, escaped).

JSON: array per row; `jsonl` emits one object per line.

NOT in scope: encryption / signing of the output (separate
brief if compliance demands it). NOT remote upload — write to
local path only.

## Approach

1. NEW `crates/argo-cli/src/handlers/approval_export.rs`
   (~280 LOC):
   - Reuse `build_query` from
     `claude-approval-history-search-cli` if landed; otherwise
     inline a private copy with a TODO comment to dedupe later.
   - CSV via `csv` crate (already a workspace dep — verify;
     add to allowlist for argo-cli only if missing per
     `project_argo_dep_allowlists.md`).
   - SHA256 via `sha2` crate (existing workspace dep).
2. CLI surface adds `Export(ApprovalExportArgs)` variant under
   `ApprovalCmd`.
3. Integrity stderr print: `wrote 1234 rows, sha256=<hex>` —
   parseable by simple shell scripts.
4. Refuse to overwrite existing files unless `--force`.

## Files you may touch

- `crates/argo-cli/src/handlers/approval_export.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/approval_export_tests.rs` (NEW, ≤ 280 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod line
- `crates/argo-cli/src/cli.rs` / `main.rs` — narrowly: add
  subcommand variant + dispatch (BLANKET CLI ALLOWLIST). cli.rs is 49 LOC post-#968 split.
- `crates/argo-cli/src/cli_command_enum.rs` (post-#968: top-level `Command` variants live here)
- `crates/argo-cli/tests/approval_export_smoke.rs` (NEW) —
  seed → export → re-read CSV → assert round-trip
- `crates/argo-cli/Cargo.toml` — narrowly: add `csv` if missing
  (and only if allowlist permits)

## Files you must NOT touch

- Storage schema or migrations
- `session_approval.rs` decision logic
- Other approval handlers
- Any non-argo-cli crate's `Cargo.toml`

## Acceptance criteria

- [ ] `argo approval export --since 30d --format csv --out /tmp/q1.csv`
      writes a UTF-8 CSV with header row + one row per approval
- [ ] `--format json` writes a valid JSON array
- [ ] `--format jsonl` writes one object per line
- [ ] sha256 printed to stderr matches `sha256sum` on the file
- [ ] `--out -` writes to stdout (UNIX convention)
- [ ] Refuses to overwrite without `--force`
- [ ] CSV escaping verified for payloads containing commas,
      quotes, and newlines (round-trip test)
- [ ] Help-smoke test pins every flag
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every file < 500 LOC

## Branch + PR

- Branch: `codex-approval-audit-export`
- PR title: `approval: audit export CSV/JSON (codex-approval-audit-export)`

## Notes for the implementer

- Land AFTER `claude-approval-history-search-cli` if possible
  to share the query builder. If forced to land first, use a
  private inlined copy + leave a TODO to dedupe.
- payload_excerpt: truncate at 80 chars and append `...` if
  longer. Replace embedded newlines with `\\n` (literal
  two-char escape) so CSV stays single-line per row.
- The integrity hash is computed over the file's BYTES, not
  the row set — re-running with the same filters MUST produce
  the same hash for a given store snapshot (deterministic
  ORDER BY on `requested_at, id`).
- For jsonl, each line MUST be valid JSON on its own (no
  trailing comma, no array wrapper).
