# Mission load-bearing wire #7 — wire operator feedback into CU maybe_propose

**Owner**: codex. **Depends on**: claude-cu-skill-learn-wire (#807), claude-operator-feedback-channel (#889).

> SOURCE: deep-research findings tick 198, Section 1, item #8.
> Mechanical swap: CU executor's `maybe_propose` calls the
> non-feedback `should_propose`; should call
> `should_propose_with_feedback` so operator votes actually flow
> into the gate.

## Scope

[`crates/argo-executor-computer-use/src/learn.rs:158`](crates/argo-executor-computer-use/src/learn.rs)
defines:

```rust
pub async fn maybe_propose(
    store: Option<&SkillStore>,
    request: &RunRequest,
    outcome: &Outcome,
    session: SessionId,
    threshold: f64,
) -> Option<SkillCandidate> {
    let metric = success_metric_for_outcome(outcome);
    if !should_propose(metric, threshold) {
        return None;
    }
    ...
}
```

`should_propose_with_feedback` was added by
claude-operator-feedback-channel (#889) and is exported from
`argo-skill::feedback` ([lib.rs:41](crates/argo-skill/src/lib.rs)).
It honors operator votes (Up forces propose, Down blacklists, None
falls back to the threshold check). Today the CU dispatch path
ignores it.

This brief swaps the call. ~30 LOC including tests.

## Files you may touch

- `crates/argo-executor-computer-use/src/learn.rs` — narrowly:
  swap `should_propose` → `should_propose_with_feedback`; the
  function signature gains an optional feedback-store handle
- `crates/argo-executor-computer-use/src/learn_tests.rs` —
  narrowly: 3 new tests (Up vote forces propose under threshold;
  Down vote blacklists over threshold; None falls back to threshold)
- `crates/argo-executor-computer-use/Cargo.toml` — narrowly: verify
  `argo-skill` already a dep (it is, via H-4c); no new deps expected

## Files you must NOT touch

- `crates/argo-skill/**` (the feedback API is locked from #889)
- The CU dispatch loop (`anthropic.rs` etc.) — this brief is just
  the propose-gate swap

## Acceptance criteria

- [ ] `maybe_propose` calls `should_propose_with_feedback` instead
      of `should_propose`
- [ ] Operator Up vote forces propose even when metric is sub-threshold
- [ ] Operator Down vote blocks propose even when metric is suprathreshold
- [ ] No vote → existing threshold semantics preserved
- [ ] 3 new tests + existing tests still pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-executor-computer-use` clean
- [ ] Every modified file < 500 LOC (currently 267 LOC; trivially safe)

## Branch + PR

- Branch: `codex-mission-wire-feedback-in-cu-executor`
- PR title: `cu: maybe_propose honors operator feedback votes (mission-wire-feedback-in-cu-executor)`

## Notes for the implementer

- This is the smallest brief in the mission-wires set (~30 LOC).
  It is gated behind `claude-cu-skill-learn-wire` only because
  that brief established the maybe_propose call site.
- Don't expand scope. The feedback semantics are entirely owned
  by `argo-skill::feedback`; this brief is the call-site swap, no
  more.
