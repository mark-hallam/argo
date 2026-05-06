# Claude — Wire H-4c skill-learning into ComputerUseExecutor

**Owner**: claude. **Depends on**: B-computer-use-executor (#787),
H-4c (#805) merged.

## Scope

Close the skill-learning loop that H-4c stubbed. When the
`ComputerUseExecutor` completes a successful trajectory with a
`success_metric` above the threshold, emit a `SkillCandidate`
through the existing `SkillStore::propose_for_review` flow so a
human can review + approve before the skill goes live.

Today H-4c ships the data-shape primitives but the executor
doesn't actually call them. This brief wires the seam.

## Files you may touch
- `crates/argo-executor-computer-use/src/executor.rs` (extend
  the post-success path)
- `crates/argo-executor-computer-use/src/learn.rs` (NEW —
  the bridge module that holds the skill-learning hook so
  executor.rs stays under 500 LOC)
- `crates/argo-executor-computer-use/Cargo.toml` — add
  `argo-skill` path dep
- `crates/argo-executor-computer-use/src/lib.rs` — narrowly:
  add `mod learn;` declaration

## Files you must NOT touch
- `crates/argo-skill/**` — H-4c primitives are fixed; just
  call them
- The `Executor` trait
- Any other crate

## Acceptance criteria
- [ ] After a successful CU action, compute a
      `success_metric` (use `1.0` for explicit completion or
      `0.0` if the action raised an error — basic signal for
      now; nuanced metrics are a follow-up)
- [ ] Call `argo_skill::learn::should_propose(metric, threshold)`
      with `threshold = DEFAULT_SUCCESS_THRESHOLD` (0.8)
- [ ] On `true`, build a `SkillCandidate::from_cu_trajectory`
      from the executor's step log and call
      `SkillStore::propose_for_review` (the propose_for_review
      method exists from prior work — search for it)
- [ ] If the proposal fails (skill name collision, store
      error), log a warning and continue — do NOT fail the
      action
- [ ] The skill lands with provenance source
      `learned:<session_short>` so H-4a's `from_source` grades
      it as `LearnedUnreviewed`
- [ ] Unit test: mock a successful executor outcome, assert
      a SkillCandidate is created with the right shape
- [ ] Integration-style test: full executor run end-to-end
      against an in-memory store, assert the proposed skill
      is queryable
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] Every authored file < 500 LOC

## Branch + PR
- Branch: `claude-cu-skill-learn-wire`
- PR title: `executor: wire skill-learning loop on successful CU completion (claude-cu-skill-learn-wire)`

## Notes
- The `success_metric` heuristic above is a v1 starting point;
  a richer metric (operator-rated, trajectory-length, model-
  confidence) is a future brief.
- The trajectory format used by `SkillCandidate::from_cu_trajectory`
  is `Vec<String>` — render each CU step as one line. Use
  whatever the executor already logs (screenshots stripped —
  text only).
- "Session short" for the provenance label: take the first 8
  chars of the session id.
