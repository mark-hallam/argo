# Stream H — `argo eval run` CLI

**Owner**: codex. **Depends on**: G-genesis (#790),
demo-marketplace-seller-vertical (#818) merged.

## Scope

`argo eval run --vertical <id> [--score] [--limit N]` runs
the vertical's eval seed (eval/seed.jsonl) against the
worker pipeline.

Subcommands:
- `argo eval run --vertical <id>` → execute every fixture +
  pass/fail summary
- `argo eval run --vertical <id> --score` → also compute
  per-fixture scores via argo-judge
- `argo eval list --vertical <id>` → table of fixtures

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/cli_eval_cmds.rs` (NEW)
- `crates/argo-cli/src/handlers/eval_run.rs` (NEW)
- `crates/argo-cli/tests/eval_run.rs` (NEW)

## Acceptance criteria
- [ ] All 3 subcommands wired
- [ ] `--vertical` required + `--json` everywhere
- [ ] Test against demo-marketplace-seller fixtures
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `codex-H-eval-runner-cli`
- PR title: `cli: argo eval run/list (H-eval-runner-cli)`
