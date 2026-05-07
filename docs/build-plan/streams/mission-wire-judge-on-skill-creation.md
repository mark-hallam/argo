# Mission load-bearing wire #5 — wire Judge into propose_for_review gate

**Owner**: claude. **Depends on**: H-auto-approval H-4c (#805), claude-skill-quality-judge (#886), argo-judge crate present on main.

> SOURCE: deep-research findings tick 198, Section 1, item #6.
> Mission claim 4 ("workers grow new skills … judge vetoes
> uncertain ones") is delivered structurally — `argo-judge` is a
> real crate with `adjudicate()` at
> [judge.rs:30](crates/argo-judge/src/judge.rs:30) — but no skill
> creation path actually calls it. This brief is the wire.

## Scope

[`crates/argo-skill/src/store_vertical.rs:123`](crates/argo-skill/src/store_vertical.rs)
defines `propose_for_review()` — the entry point for a CU success
or operator proposal to land a `SkillCandidate` in the
`PendingReview` state. Today the function performs lifecycle
state-machine + lock checks but does NOT consult the judge.

This brief inserts a judge gate before the candidate transitions
into `PendingReview`. If `Judge::adjudicate(candidate)` returns
a verdict below the configured floor (default 0.6 confidence),
the candidate is recorded with state `Rejected` and a stamped
reason; otherwise it proceeds to `PendingReview` as today.

The judge call is async + bounded by a 30s timeout; on timeout
or transport error the candidate FAILS OPEN (proceeds to
PendingReview with a `judge_unavailable` audit row), because we
do not want a degraded judge to block all skill learning.

## Files you may touch

- `crates/argo-skill/src/store_vertical.rs` — narrowly: add
  optional `judge: Option<Arc<dyn Judge>>` to `propose_for_review`
  signature; call it before the lifecycle transition; reject with
  reason on low confidence
- `crates/argo-skill/src/lib.rs` — narrowly: re-export the new
  signature; keep the legacy zero-judge form for back-compat (call
  it `propose_for_review_no_judge` and mark `#[deprecated]`)
- `crates/argo-skill/Cargo.toml` — narrowly: add
  `argo-judge = { path = "../argo-judge" }` if not already present
- `crates/argo-skill/src/tests.rs` — narrowly: extend the existing
  4 propose_for_review tests with a 5th using a mock judge that
  returns rejection; assert state == Rejected
- `crates/argo-skill/src/store_vertical_judge_tests.rs` (NEW) — 5
  unit tests (judge approves → PendingReview; judge rejects →
  Rejected with reason; judge times out → PendingReview +
  judge_unavailable audit; floor configurable; back-compat path
  still works)
- `crates/argo-executor-computer-use/src/learn.rs` — narrowly:
  thread the judge through `maybe_propose` → `propose_skill_from_run`
  call (one new optional parameter)

## Files you must NOT touch

- `crates/argo-judge/**` (the crate is locked from prior work; reuse
  its public API only)
- `crates/argo-skill/src/store.rs` (related but separate; this
  brief targets `store_vertical.rs` only)
- `crates/argo-cli/src/handlers/forge_optimize.rs` — separate brief
  (`mission-wire-judge-on-forge-promote`)

## Acceptance criteria

- [ ] `propose_for_review` accepts an optional Judge handle
- [ ] Judge verdict < floor → candidate state `Rejected` with
      stamped reason `judge_low_confidence: <verdict>`
- [ ] Judge transport-error / timeout → candidate proceeds to
      `PendingReview` AND a `judge_unavailable` audit row is written
- [ ] Floor configurable via env `ARGO_JUDGE_SKILL_FLOOR` (default 0.6)
- [ ] 5 new unit tests covering all four outcomes + back-compat
- [ ] CU executor's `maybe_propose` threads the judge through
- [ ] Back-compat: existing call sites without a judge still work
      (legacy form preserved with deprecation marker)
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-skill -p argo-executor-computer-use` clean
- [ ] Every modified file < 500 LOC (`store_vertical.rs` is currently
      well under; this adds maybe 40 LOC)

## Branch + PR

- Branch: `claude-mission-wire-judge-on-skill-creation`
- PR title: `skill: judge gate on propose_for_review (mission-wire-judge-on-skill-creation)`

## Notes for the implementer

- The `argo-judge::Judge` trait shape is fixed; do not extend it in
  this brief.
- Fail-open on judge degradation is deliberate. A broken judge
  should not block all skill learning across the platform; it
  should be visible (audit row + doctor check in a follow-up brief)
  but non-blocking.
- This brief's twin is `mission-wire-judge-on-forge-promote` — both
  call the same Judge trait; if either brief lands first the second
  can reuse the integration shape.
- After this lands, mission claim 4 ("judge vetoes uncertain ones")
  stops being aspirational.
