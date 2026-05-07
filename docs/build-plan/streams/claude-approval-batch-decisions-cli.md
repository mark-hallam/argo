# V1.4 — Approval batch decisions CLI

**Owner**: claude. **Depends on**: H-auto-approval merged,
session_approval.rs handlers stable.

## Why

Operators with a backlog of pending approval requests today must
decide them one-at-a-time via `argo approval approve <id>` /
`argo approval deny <id>`. After a long run that produced 30+
queued prompts (common during a multi-worker SKYCOT swarm) this
is tedious. Mission claim 6 calls multi-channel approvals "v0";
batch decisions are table-stakes for v1 ergonomics.

## Scope

Add `argo approval batch <decision> <id1> <id2> ...` that
applies the same decision (approve | deny) to every listed
pending approval id atomically (per id; no transactional
all-or-nothing — partial success is fine and reported). Print
a summary table at the end: id | result | reason-on-failure.

Default behaviour confirms once before writing
("Apply 'approve' to 14 pending approvals? [y/N]") unless
`--yes` / `-y` is passed. `--dry-run` lists the ids that would
be touched and exits without writing.

NOT in scope: filter expressions like `--all-pending` or
`--worker <name>` (separate brief
`claude-approval-history-search-cli` covers query). NOT
modifying decision storage schema.

## Approach

1. NEW `crates/argo-cli/src/handlers/approval_batch.rs`
   (~200 LOC, ≤ 320):
   - `pub async fn run(args: ApprovalBatchArgs) -> ArgoResult<()>`
   - Iterates ids, calls existing `session_approval::apply_decision`
     (or whatever the canonical single-id path is — confirm by
     reading `session_approval.rs` first) for each.
   - Collects outcomes into a `Vec<BatchOutcomeRow>` and prints
     via `tabled` (already a workspace dep).
2. Wire CLI surface in `cli.rs` under the existing
   `Approval` subcommand (BLANKET CLI ALLOWLIST — cli.rs is
   permitted).
3. Unit tests in `approval_batch_tests.rs` covering: all approve,
   mixed (some ids missing), dry-run, --yes skips prompt.

## Files you may touch

- `crates/argo-cli/src/handlers/approval_batch.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/approval_batch_tests.rs` (NEW, ≤ 250 LOC)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 1 pub mod line
- `crates/argo-cli/src/cli.rs` — narrowly: add `Batch` variant to
  `ApprovalCmd` enum + dispatch arm (BLANKET CLI ALLOWLIST)
- `crates/argo-cli/src/main.rs` — narrowly: dispatch handler
  (BLANKET CLI ALLOWLIST)
- `crates/argo-cli/tests/approval_batch_smoke.rs` (NEW) — one
  end-to-end test: seed 5 pending, batch approve, verify all 5
  marked Approved + exit 0

## Files you must NOT touch

- `crates/argo-types/src/handoff.rs` — no schema change
- `session_approval.rs` — call its existing public API; do not
  modify its internals
- Storage layer (`crates/argo-store/`) — no migration

## Acceptance criteria

- [ ] `argo approval batch approve a b c` approves all three
- [ ] Missing / already-decided ids are reported as failure rows
      but don't abort the run
- [ ] Confirmation prompt shows the count + decision; `--yes`
      bypasses it
- [ ] `--dry-run` writes nothing
- [ ] Help-smoke test pins the subcommand variants
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-approval-batch-decisions-cli`
- PR title: `approval: batch decisions CLI (claude-approval-batch-decisions-cli)`

## Notes for the implementer

- Read `session_approval.rs` first to find the single-id apply
  function — reuse it verbatim. Do not duplicate the
  worker-state transition logic.
- The confirmation prompt should use `dialoguer` if already a
  workspace dep, else a hand-rolled `print!` + `read_line`.
- Output format: stable column order (id, decision, status,
  note) so downstream `argo approval export` (separate brief)
  can re-use the same row shape.
