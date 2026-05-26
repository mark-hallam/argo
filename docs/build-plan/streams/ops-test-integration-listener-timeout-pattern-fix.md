# ops-test-integration-listener-timeout-pattern-fix

**Owner**: claude or codex.
**Authored against**: origin/main `c35ad17d9` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-test-integration-listener-timeout-pattern-fix`.
**Source**: evaluator round 84 batch-pre-merge-drain (`docs/skycot/evals/scheduled/2026-05-26/round-84-batch-pre-merge-drain.md`) brief-shape recommendation. Closes the PR #2376 flake class (PASS-WITH-CAVEAT verdict because the integration test failed 3× on hardcoded 30s listener-await without `--skip-startup-check`).

🟢 **P3 — operational test-hardening.** Several `crates/argo-cli/tests/*.rs` integration tests use a hardcoded `Duration::from_secs(30)` deadline to await the gateway listener coming up. Under shared-target compile load (multiple cold-cargo runs hitting the same target dir), the 30s window is sometimes not enough — the test fails intermittently with no logic regression. The fix-pattern PR #2402 introduced uses `await_listener_signal` from `common::listener_signal` which is more robust under contention.

## Acceptance criteria

- [ ] `rg -l "Duration::from_secs\(30\)" crates/argo-cli/tests/` returns the candidate file list (estimate ~5-10 files).
- [ ] For each candidate, identify whether it's an integration test that waits for the gateway listener. If so:
  - Replace the hardcoded `tokio::time::sleep(Duration::from_secs(30))` or the equivalent retry loop with `await_listener_signal` from `crates/argo-cli/tests/common/listener_signal.rs` (or whatever the path is — PR #2402 introduces it).
  - Verify the test still passes (cold cargo + 3 consecutive runs to confirm no flake).
- [ ] PR body documents the sweep findings + lists every file modified.
- [ ] No production code changes — purely test infrastructure.

## May-touch

- `crates/argo-cli/tests/*.rs` (sweep candidates)
- `crates/argo-cli/tests/common/listener_signal.rs` (the pattern source — only if extending the helper)

## Out of scope

- Adding new tests — only refactor existing ones.
- Other flake classes (compile-time, async-runtime, fs-race) — separate brief if found.
- Touching production code paths — this is pure test ergonomics.

## Worker note

PR #2376 (`fix-onboarding-provision-idempotent-flag-and-toast`) was merged as PASS-WITH-CAVEAT in R84 because its single new integration test exhibits this flake. Production code in that PR is a 1-line additive (adding `"created": outcome.created` to the JSON payload) and is correct. This brief closes the broader pattern that caused the flake. Estimate ~40 LOC across 5-10 files. Falls under operator-locked ops-* `--skip-clippy` lane after evaluator pre-verification.
