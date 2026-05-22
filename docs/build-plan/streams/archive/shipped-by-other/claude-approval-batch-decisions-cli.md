# V1.4 — Approval batch decisions CLI

**Owner**: claude.

**Authored against**: origin/main 2026-05-09 (rewrite v2 by conductor — original v1 referenced a non-existent `apply_decision` API and assumed an `argo approval` top-level command. The actual surface is `argo worker actions approve-batch` (in `cli_worker_cmds.rs::WorkerActionsCmd::ApproveBatch`); this v2 narrows to extending it with `--deny` / `--confirm` / `--dry-run` flags). **Depends on**: H-auto-approval merged,
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

- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly: extend the existing `WorkerActionsCmd::ApproveBatch` variant with `--deny`, `--confirm`, `--dry-run` flags. Do NOT introduce a new top-level `argo approval` command — the existing surface is `argo worker actions approve-batch`.
- `crates/argo-cli/src/handlers/worker_actions.rs` — narrowly: dispatch arm for the new flags.
- `crates/argo-cli/src/handlers/worker_actions_dispatch.rs` — narrowly: extend `approve_batch` to honour the new flags. Do NOT reference a non-existent `apply_decision` API; the dispatch already knows how to approve/deny.
- `crates/argo-cli/src/handlers/worker_actions_tests.rs` (or sibling) — narrowly: tests for the new flag behaviour.

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
