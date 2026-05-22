# claude-cli-worker-cmds-split — split cli_worker_cmds.rs to free WorkerCmd headroom

**Owner**: claude. **Depends on**: nothing. **Phase A architectural prep — unblocks `claude-worker-presence-tracking` and any future `argo worker <verb>` extension.**

**Authored against**: origin/main 046088ff on 2026-05-09 (conductor-authored after pre-claim ceiling guard refused `claude-worker-presence-tracking` on `cli_worker_cmds.rs` at 496/500 LOC).

## Why

`crates/argo-cli/src/cli_worker_cmds.rs` is at 496 LOC on current
main — 4 LOC of headroom against the 500-LOC HARD RULE 6 ceiling.
The pre-claim ceiling guard (shipped 2026-05-09 in commit 8236cab8)
correctly refuses any brief whose may-touch list includes
`cli_worker_cmds.rs`, including `claude-worker-presence-tracking`
which adds a small `Fleet` enum variant that would otherwise fit.

The right fix is to split `cli_worker_cmds.rs` so the `WorkerCmd` enum
+ its dispatch arms have ample headroom for future `argo worker <verb>`
extensions. Pure refactor; no behaviour change.

## Approach (concrete)

`cli_worker_cmds.rs` likely contains the `WorkerCmd` enum + dispatch
table for many verbs that have accumulated over time (`charter`,
`onboarding`, `archive`, `costs`, `kpis`, `okr`, `handoff`, etc.).
Inspection should show several "verb groups" that can be moved to
sibling files following the existing per-verb sibling-file pattern
(e.g., `cli_worker_archive_cmds.rs`, `cli_worker_budget_cmds.rs`).

Two viable splits:

**Option A (preferred): keep enum + dispatch in cli_worker_cmds.rs,
extract one or two cohesive verb groups to siblings.**
- Identify the 1-2 largest verb groups by `wc -l` of their dispatch
  arms + impl blocks
- Move them to `cli_worker_<verb>_cmds.rs` siblings
- Keep `WorkerCmd` enum variants in cli_worker_cmds.rs (they're
  one-liners and stable)
- Re-export or inline `pub use` from the sibling

**Option B (only if A insufficient): split the enum itself into
sub-commands.**
- Introduce `WorkerCmd::Charter(WorkerCharterCmd)` etc.
- Move each sub-enum to its sibling file
- More invasive; only do this if Option A doesn't free enough room

Target after split: cli_worker_cmds.rs ≤ 350 LOC (≥ 150 LOC headroom
for future `argo worker <verb>` additions).

## Files you may touch

- `crates/argo-cli/src/cli_worker_cmds.rs` — narrowly: extract verb
  groups to sibling(s), re-export
- `crates/argo-cli/src/cli_worker_<verb>_cmds.rs` (NEW, ≤ 250 LOC each)
  — extracted content
- `crates/argo-cli/src/main.rs` — narrowly: any new `mod` declarations
  needed
- `crates/argo-cli/src/cli_reexports.rs` — narrowly: re-exports if
  any callers consume from there

## Files you must NOT touch

- The actual handler files in `crates/argo-cli/src/handlers/worker_*.rs`
  — refactor only the CLI subcommand layer
- Any test file other than to verify no behaviour change
- The `cli_command_enum.rs` `Worker` variant — the top-level enum is
  stable

## Acceptance criteria

- [ ] `cli_worker_cmds.rs` ≤ 350 LOC after the split (≥ 150 LOC
      headroom for future `argo worker <verb>` additions)
- [ ] Each extracted sibling ≤ 250 LOC
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` clean (no behaviour change)
- [ ] No public API changes; existing call sites unchanged
- [ ] `argo worker --help` output identical pre/post (capture stdout
      and diff)
- [ ] Every modified file ≤ 500 LOC

## Branch + PR

- Branch: `claude-cli-worker-cmds-split`
- PR title: `cli: split cli_worker_cmds.rs below 500-LOC ceiling (claude-cli-worker-cmds-split)`

## Notes for the implementer

- Pattern-match on prior splits: `claude-routes-post-split` (#932),
  `claude-routes-get-split` (#956), `claude-channels-push-voice-split`
  (#1087), `claude-doctor-channels-split` (#1088). All four are
  pure-refactor extracts that preserve public API via `pub use`.
- The existing per-verb sibling pattern in argo-cli
  (`cli_worker_archive_cmds.rs`, `cli_worker_budget_cmds.rs`,
  `cli_worker_costs_cmds.rs`) is the model — pull more verbs out of
  `cli_worker_cmds.rs` into the same sibling shape.
- Expected biggest-headroom-recovery extracts: handoff, kpi, okr
  (verbs added recently in the Phase B claim cycle). Confirm via
  `wc -l` after extraction that the source file is ≤ 350.

## Unblocks

- **claude-worker-presence-tracking** — currently refused by pre-claim
  ceiling guard (cli_worker_cmds.rs at 496/500). After this brief
  lands, the worker-presence brief becomes claimable.
- Any future `argo worker <verb>` extension that would otherwise
  push past the ceiling.
