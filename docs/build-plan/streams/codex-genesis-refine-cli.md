# V1.x — Genesis refine CLI

**Owner**: codex. **Depends on**: G-genesis (#790),
claude-genesis-llm-refinement (#812) merged.

## Scope

`argo genesis refine <vertical-id>` improves an existing
vertical via LLM iteration. Genesis (G-genesis #790)
generates verticals offline; the LLM refinement pass
(#812) was opt-in for new verticals. This brief lets
operators **re-refine existing** verticals — useful when
the operator wants to improve a vertical they've already
deployed but find sub-optimal.

- `argo genesis refine <vertical-id> [--judge|--no-judge]
  [--max-iterations <n>] [--dry-run]`
- Reads the vertical's current charter + manifest
- Calls Anthropic with the same refinement prompt as #812
- Re-runs safety judge unless `--no-judge`
- `--dry-run` prints the diff without writing
- Otherwise persists refined vertical (versioned — old
  vertical kept as `<id>-v<n>`)

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/genesis_refine.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: pub mod
- `crates/argo-cli/src/cli_genesis_cmds.rs` — narrowly: add
  `Refine` subcommand variant
- `crates/argo-cli/tests/genesis_refine_smoke.rs` (NEW)
- `crates/argo-vertical/src/version.rs` (NEW or extend) —
  versioning helpers if not already present

## Acceptance criteria
- [ ] `--dry-run` prints diff cleanly
- [ ] Without `--dry-run`, persists refined vertical
      versioned
- [ ] `--no-judge` skips safety re-run
- [ ] `--max-iterations` honored
- [ ] `--json` envelope
- [ ] 4 unit tests + 1 CLI smoke test
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-genesis-refine-cli`
- PR title: `cli: argo genesis refine — re-refine existing verticals (codex-genesis-refine-cli)`
