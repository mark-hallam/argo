# fix-pitch-truth-boundary-runtime-enforcement-brief-refresh

**Owner**: claude.
**Authored against**: origin/main `bec35dd45` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pitch-truth-boundary-runtime-enforcement-brief-refresh`.
**Unblocks**: Track P P-02 cascade (P-05, P-06, P-07, P-08) — operator-locked v1.0 demo gate.

🔴 **P0 v1.0 ship-blocker — META-BRIEF — refreshes BLOCKED P-02 brief against current main**

The existing P-02 brief at `docs/build-plan/streams/pitch-truth-boundary-runtime-enforcement.md` was BLOCKED 2026-05-25T07:22Z by codex-heartbeat pre-claim with three drift points:

1. **`crates/argo-agent-runner/src/agent_deps.rs`** is referenced in the brief's may-touch list but does NOT exist on current `origin/main`. Current files in the crate: `lib.rs / turn.rs / turn_helpers.rs / turn_helpers_*.rs / dispatch.rs / helpers.rs / transport.rs`. The brief assumed `AgentDeps` lives at `agent_deps.rs`; the type has moved.
2. **`crates/argo-audit/src/events.rs`** is referenced; current file is `crates/argo-audit/src/event.rs` (+ `event_kind.rs` for the kind enum + `sink.rs` for emission). Plural→singular rename happened in a recent canonicalization pass.
3. **API-contract mismatch**: brief requires `CapabilityEngine::evaluate_claim` but P-01 (#2214) shipped `argo_capability::truth_boundary::evaluate_claim` as a **free function** at `crates/argo-capability/src/truth_boundary.rs:414`. Brief explicitly forbids changing `truth_boundary.rs`, so the engine type either needs to be a new wrapper outside that file or the brief needs to drop the wrapper.

## Why this brief exists

P-02 is the gate that makes truth-boundary *enforced* rather than *defined*. Without it the demo storyboard's three refusal beats (Scenes 2b/2c/4b) cannot happen — the agent would still hallucinate a valuation because no code path is checking. Track P P-05 / P-06 / P-07 / P-08 all `BLOCKED-BY` P-02 directly or transitively; unblocking P-02 cascades to the entire Track P demo gate.

## Scope (this brief — edit the OTHER brief; do NOT implement P-02 here)

Edit `docs/build-plan/streams/pitch-truth-boundary-runtime-enforcement.md` in place:

1. **Re-locate `AgentDeps`** — `rg 'struct AgentDeps' crates/argo-agent-runner/` to find current home; update may-touch list with the actual path. Likely `lib.rs` or `turn.rs`.
2. **Replace `events.rs` references** with `event.rs` / `event_kind.rs` / `sink.rs` as appropriate. Verify with `rg 'pub enum AuditEventKind|pub struct AuditEvent' crates/argo-audit/`.
3. **Resolve the API mismatch — choose ONE of two recipes** and update the brief accordingly:
   - **Recipe A (preferred — minimal scope)**: call `argo_capability::truth_boundary::evaluate_claim()` directly as a free function from the turn-helpers exit point. Define a thin local `CapabilityEngine` wrapper IN THIS BRIEF (new surface in `argo-capability` or `argo-agent-runner`) that holds the loaded `TruthBoundaryPolicy` + emission handle. Does NOT touch `truth_boundary.rs`.
   - **Recipe B (richer scope)**: add a `CapabilityEngine` struct in P-01 territory. Requires touching `truth_boundary.rs` (which the original brief explicitly forbade) — flag as a P-01 follow-up if chosen.
   Document the choice + 1-sentence rationale in the brief's intro.
4. **Re-check freshness** — update `Authored against:` SHA to current `origin/main` (`bec35dd45` or whatever is current at refresh time). Run `bash scripts/check-brief-freshness.sh pitch-truth-boundary-runtime-enforcement`; confirm exit 0.
5. **Flip ledger row BLOCKED → TODO** for the P-02 row in `docs/build-plan/BUILD_STATUS.md` (the row is currently around line 1327). Commit the BLOCKED→TODO flip in the same PR as the brief refresh.

## Acceptance criteria

- All may-touch paths in `pitch-truth-boundary-runtime-enforcement.md` exist on current `origin/main` (verify with `ls`).
- API-contract section names the actual exported symbol in `argo-capability` (free function vs. method).
- `bash scripts/check-brief-freshness.sh pitch-truth-boundary-runtime-enforcement` exits 0.
- BUILD_STATUS.md row for `pitch-truth-boundary-runtime-enforcement` flipped BLOCKED → TODO.
- This meta-brief flips to IN_REVIEW once the PR opens.

## May-touch

- `docs/build-plan/streams/pitch-truth-boundary-runtime-enforcement.md` (refresh)
- `docs/build-plan/BUILD_STATUS.md` (P-02 BLOCKED → TODO flip)
- `docs/build-plan/streams/fix-pitch-truth-boundary-runtime-enforcement-brief-refresh.md` (status updates)

## Out of scope

- Implementing P-02 itself (separate worker claim after refresh lands)
- Touching `crates/argo-capability/src/truth_boundary.rs`
- Touching any Rust source code

## Effort

~15-25 minutes — pure doc edit (~80-120 LOC of brief delta).
