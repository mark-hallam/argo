# Genesis iteration cycle — operator-driven revise of system prompt + actions + eval seeds

**Owner**: claude. **Depends on**: claude-genesis-prompt-engineer.

## Scope

After Genesis emits a v0 vertical, the operator typically says "but I
also want X" or "drop Y" or "make this more conservative". Today they
re-run `genesis run` from scratch, losing locked sections. This brief
adds a `genesis revise` CLI command that takes a free-text feedback
string and surgically revises the system prompt + action vocabulary +
eval seeds while preserving anything the operator marked `[KEEP]`.

Mirrors mission claim 3 (iteration loop).

## Approach (concrete)

1. **NEW** `crates/argo-cli/src/handlers/genesis_revise.rs` (~280 LOC):
   - `pub async fn handle_revise(args: GenesisReviseArgs) -> ArgoResult<()>`
   - Args: `--vertical <name>` (existing vertical to revise), `--feedback <text>`,
     `--target <prompt|actions|eval|all>` (default: all),
     `--dry-run` (print diff, don't write)
   - Loads the existing `verticals/<name>/{vertical.toml, system-prompt.md, eval-seed.toml}`
   - For each target, calls the matching genesis sub-module:
     - prompt → `argo_genesis::prompt_engineer::revise(...)`
     - actions → `argo_genesis::action_suggest::suggest(...)` with
       updated role description = original + feedback delta
     - eval → `argo_genesis::eval_seed::generate(...)` with refreshed actions
   - Writes outputs back; prints unified diff in `--dry-run` mode
2. **Modify** `argo-cli/src/cli.rs`: add `Genesis::Revise { ... }` subcommand
3. **Modify** `handlers/mod.rs`: 1 pub mod line

## Files you may touch

- `crates/argo-cli/src/handlers/genesis_revise.rs` (NEW, ≤ 320 LOC)
- `crates/argo-cli/src/handlers/genesis_revise_tests.rs` (NEW, ≤ 220 LOC) — 5 tests:
  - revise --target prompt updates system-prompt.md, leaves actions alone
  - revise --target all updates all three artefacts
  - --dry-run does not modify any file on disk
  - missing vertical → ArgoError::NotFound
  - locked sections (`[KEEP]…[/KEEP]`) survive prompt revision
- `crates/argo-cli/src/cli.rs` (narrowly: add Revise subcommand)
- `crates/argo-cli/src/handlers/mod.rs` (narrowly: 1 pub mod line)
- `crates/argo-cli/tests/genesis_revise_smoke.rs` (NEW, ≤ 180 LOC) —
  end-to-end: scaffold a fake vertical in a tempdir, run revise --target prompt --dry-run, assert diff is non-empty

## Files you must NOT touch

- `crates/argo-genesis/src/*` — this brief consumes the genesis API,
  doesn't extend it. Extension belongs in the prompt-engineer /
  action-suggestor / eval-seed-generator briefs.
- The 5 demo verticals (#937) — tests use a tempdir scaffold

## Acceptance criteria

- [ ] `argo genesis revise --vertical sales-sdr --feedback "be more terse" --dry-run` prints a non-empty diff
- [ ] `argo genesis revise --vertical sales-sdr --feedback "..." --target actions` only updates `vertical.toml`
- [ ] Locked-section markers survive the revision
- [ ] Smoke test scaffolds a tempdir vertical and runs the full revise path
- [ ] 5 unit tests + 1 smoke test pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-cli` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-iteration-cycle`
- PR title: `genesis: revise CLI command for prompt/actions/eval iteration (claude-genesis-iteration-cycle)`

## Notes for the implementer

- BLANKET CLI ALLOWLIST applies — cli.rs / handlers/mod.rs / Cargo.toml
  edits are pre-approved.
- Diff output: prefer `similar` crate (already in workspace deps from
  audit work) or roll a minimal line-by-line diff.
- For the LLM call inside revise(), reuse `argo-llm` — do not introduce
  a new client.
