# Claude — Streaming eval output

**Owner**: claude. **Depends on**: nothing (independent).

## Scope

`argo eval run` runs synchronously today. Add streaming:

- Per-fixture result printed as soon as it completes
- Format: `[3/12] verify_price: PASS (172ms, $0.0021)`
- `--stream off|simple|verbose` flag (default: simple)
  - `off` → batch (existing behaviour)
  - `simple` → one line per fixture
  - `verbose` → NDJSON for piping

## Files you may touch
- `crates/argo-eval/src/streaming.rs` (NEW)
- `crates/argo-eval/src/lib.rs` — narrowly: re-export +
  add streaming variant of run()
- `crates/argo-cli/src/handlers/eval_run.rs` — narrowly:
  switch to streaming variant
- BLANKET CLI ALLOWLIST applies

## Acceptance criteria
- [ ] Streaming output shows per-fixture progress
- [ ] `--stream off` falls back to batch
- [ ] `--stream verbose` emits NDJSON
- [ ] No flaky timing tests (deterministic mock fixtures)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-eval-streaming`
- PR title: `eval: streaming progress for argo eval run (claude-eval-streaming)`
