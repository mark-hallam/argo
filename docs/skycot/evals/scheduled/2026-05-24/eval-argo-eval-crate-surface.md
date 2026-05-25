# argo-eval crate surface audit

Date: 2026-05-24
Brief: `eval-argo-eval-crate-surface`
Verdict: PASS

## What is now pinned

- `crates/argo-eval/src/lib.rs` public modules and re-export inventory are asserted from source so additions/removals require a deliberate test update.
- `EvalMetric` scoring is pinned for exact match, substring-contains all-of behavior, Jaccard overlap, and negative cases.
- `run_eval` is pinned for successful subject output, subject-error zero-score records, summary counts, mean score, pass rate, and error count.
- Regression promotion behavior is pinned for stable candidate allowance, real pass-rate regression blocking, and explicit override allowance.
- `HonestyResult` tagged-serde shape is pinned for a suspicious verdict with per-sample breakdown.
- Streaming output formats are pinned for simple fixture lines, verbose NDJSON fixture lines, simple summary lines, verbose summary lines, and off-mode summary behavior.
- Self-healing fixture updates are pinned for environmental-failure proposal, rejected operator decision no-op, and approved operator decision receipt.

## Gaps

No new product gap was found in this audit pass. Existing private module tests already cover deeper matrices for JSONL loading, comparison grouping, honesty sampling behavior, regression edge cases, and self-healing rejection cases; the new integration test pins the downstream public API contract.

## Verification

- `CARGO_TARGET_DIR=/tmp/eval-argo-eval-crate-surface-target cargo test -p argo-eval --tests`
