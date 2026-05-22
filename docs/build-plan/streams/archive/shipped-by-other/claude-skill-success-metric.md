# Claude — Richer skill-learning success metric

**Owner**: claude. **Depends on**: claude-cu-skill-learn-wire
merged.

## Scope

The first wire-up of skill-learning (`claude-cu-skill-learn-wire`)
uses a binary `success_metric` (1.0 on completion, 0.0 on
error) — adequate to test the seam but loses signal that
distinguishes a one-step quick win from a 30-step grind that
ALSO succeeded.

This brief replaces the binary with a structured score that
the orchestrator can use to rank candidates and the operator
can use to triage:

```rust
pub struct SuccessMetric {
    /// 0.0..=1.0 overall confidence the trajectory succeeded.
    pub confidence: f64,
    /// Number of executor steps. Lower = simpler path.
    pub trajectory_length: u32,
    /// Walltime in seconds.
    pub duration_seconds: f64,
    /// Optional final-state score (e.g. URL match) when the
    /// vertical defines one.
    pub final_state_score: Option<f64>,
}
```

`should_propose` evolves to take this struct + per-vertical
thresholds (or use sensible defaults).

## Files you may touch
- `crates/argo-skill/src/learn.rs` — extend `should_propose`
  signature to accept `SuccessMetric` (keep the old `f64`
  variant as a deprecated thin wrapper for one release)
- `crates/argo-skill/src/success_metric.rs` (NEW)
- `crates/argo-skill/src/lib.rs` — re-export
- `crates/argo-executor-computer-use/src/learn.rs` —
  compute the structured metric instead of binary
- `crates/argo-executor-computer-use/src/success_metric.rs`
  (NEW — heuristic computation: trajectory_length =
  step_count, duration from start/end timestamps,
  final_state_score from the vertical's optional declared
  fixture)

## Files you must NOT touch
- The skill_candidate / pending_review schemas (additive
  metric, not a schema change)
- argo-cli (the gate hookup is already wired; the new
  metric lands transparently)

## Acceptance criteria
- [ ] `SuccessMetric` struct with the 4 fields above
- [ ] `should_propose(metric: SuccessMetric, threshold:
      ProposalThreshold)` — `ProposalThreshold` is a small
      struct of per-dimension floors (default: confidence
      ≥ 0.8, trajectory_length ≤ 25, duration ≤ 600s)
- [ ] Old `should_propose(f64, f64)` kept as deprecated
      shim that builds a SuccessMetric with synthetic
      values + delegates
- [ ] CU executor computes the metric from the actual
      trajectory + timing
- [ ] Unit tests for each dimension's floor + the
      combination logic
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-skill-success-metric`
- PR title: `skill: structured SuccessMetric for proposal gating (claude-skill-success-metric)`

## Notes
- Don't over-engineer the heuristic — even a simple linear
  combination of the 4 dimensions is plenty for v1. The
  point is to keep operators from drowning in low-value
  proposals.
- Per-vertical thresholds via the manifest field
  `[skill_learning] proposal_threshold` are a v1.x
  follow-up; for now ship the defaults.
