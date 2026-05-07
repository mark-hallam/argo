# V1.4 — Eval comparison CLI

**Owner**: codex. **Depends on**: claude-eval-streaming (#842)
merged.

## Scope

Today `argo eval run` produces results and `--stream` shows
progress. Missing: easy A/B comparison across 2 eval runs to
measure score deltas after a change (skill update, prompt
tweak, vertical revision).

Add:
- `argo eval compare <run-a> <run-b> [--by skill|fixture|metric]
  [--show-regressions] [--show-improvements] [--json]` —
  side-by-side comparison; default groups by fixture, shows
  pass/fail flips + score deltas
- `argo eval list [--vertical <id>] [--limit N] [--json]` — list
  recent eval runs by id + vertical + timestamp
- `argo eval show <run> [--json]` — detail for a single run

For comparison output (markdown):
```
## Comparison: run-a vs run-b
- Total fixtures: 47 (same)
- Pass rate: A=78% → B=85% (+7%)
- Regressions: 2 fixtures (skill: invoice_review, billing_dispute)
- Improvements: 5 fixtures (skill: customer_replies)
- Score delta median: +0.04
```

## Files you may touch
- BLANKET CLI ALLOWLIST applies
- `crates/argo-cli/src/handlers/eval_compare.rs` (NEW)
- `crates/argo-cli/src/handlers/eval_list.rs` (NEW)
- `crates/argo-cli/src/handlers/eval_show.rs` (NEW)
- `crates/argo-cli/src/handlers/mod.rs` — narrowly: 3 pub mods
- `crates/argo-cli/src/cli_eval_cmds.rs` — narrowly: add
  `Compare`, `List`, `Show` variants
- `crates/argo-eval/src/compare.rs` (NEW) — pure diff logic
  (run_a, run_b → ComparisonReport)
- `crates/argo-eval/src/lib.rs` — narrowly: re-export
- `crates/argo-cli/tests/eval_compare_smoke.rs` (NEW)

## Acceptance criteria
- [ ] `argo eval list` returns list newest-first
- [ ] `argo eval show <run>` returns single run detail
- [ ] `argo eval compare a b` outputs markdown comparison
- [ ] `--by skill` groups deltas by skill instead of fixture
- [ ] `--show-regressions` filters output to only worse-than-A
- [ ] `--json` envelope universal across all 3 subcommands
- [ ] 6 unit tests + 1 CLI smoke
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean

## Branch + PR
- Branch: `codex-eval-compare-cli`
- PR title: `eval: argo eval list/show/compare — A/B run comparison (codex-eval-compare-cli)`
