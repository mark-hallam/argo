---
date: 2026-05-26
round: 80
pillar: Track P · v1.0 demo · vertical-template architecture
verdict: 🟢 PASS · Track P P-05 LIVE-on-main
---

# Round 80 · S2 — PR #2384 post-merge anti-theater: Pitch Agent worker template ships clean

## Verdict

🟢 **PASS** — Track P P-05 (`pitch-agent-worker-template`) landed clean. 5/5 integration tests pass cold against the real `packs/pitch-agent/` checkout, with one test specifically pinned to tenant-clean isolation across two independent registry loads.

## Setup

- PR: [#2384](https://github.com/mark-hallam/argo/pull/2384) merged 2026-05-26T04:44:13Z, head `95cab114a534438fe1b09b045eb51910471bb2c2`, +1183/-0 across 25 files
- Files: 17 new files under `packs/pitch-agent/`, 2 new skill modules under `crates/argo-skill/`, 1 new integration-test crate addition, 4 web cohort surfaces (dept-registry, tile-meta, sample fixtures)
- Test: `crates/argo-vertical-packs/tests/pitch_agent_template_test.rs`
- Track P brief: `streams/pitch-agent-worker-template.md` (P-05)

## Observed

**5/5 integration tests pass cold:**

```
$ CARGO_TARGET_DIR=$HOME/.cache/argo-shared-target \
    cargo test -p argo-vertical-packs --release --test pitch_agent_template_test
running 5 tests
test pitch_agent_skills_bundle_lists_required_skills ... ok
test pitch_agent_truth_boundary_policies_cover_every_channel ... ok
test pitch_agent_pack_loads_and_adapts_into_vertical ... ok
test pitch_agent_template_is_tenant_clean_across_two_loads ... ok
test pitch_agent_worker_toml_references_resolve_and_parse ... ok
test result: ok. 5 passed; 0 failed
```

Compile time 34.56s (cold release build of argo-vertical-packs and its deps).

**Production-side coverage matches the brief intent:**

- `packs/pitch-agent/worker.toml`: `id = "pitch-agent"`, `role = "public_representation_agent"`, `dry_run = true`. Delegation charter explicitly enumerates the 5 demo-relevant duties (canonical-QA answer, pitch-mode switch, slide control, follow-up drafting, escalation on financials/non-public-roadmap/legal/customers).
- `packs/pitch-agent/personas/`: 4 persona docs (elevator-30s, investor-3min, demo-10min, diligence-deep) + voice-and-tone.md — the four canonical pitch modes from the project memory.
- `packs/pitch-agent/policies/truth-boundary-ref.toml`: references the per-tenant truth-boundary capability schema landed via P-01 (#2214).
- `packs/pitch-agent/workflows/`: 3 channel workflows (rehearsal, web-embed, live-meeting) — matches the 3-channels-by-2-modes-by-1-template product spec.
- `crates/argo-skill/src/pitch_mode_switcher.rs` (79 LOC): `PersonaMode::{Elevator30s, Investor3Min, Demo10Min, DiligenceDeep}` enum + `select(context_tag)` substring router with `Investor3Min` as fallback.
- `crates/argo-skill/src/slide_controller.rs` (76 LOC): slide-control primitive.

## Anti-theater check

Three independent proofs this isn't a paper template:

1. **Tests load real packs from disk** — `crates/argo-vertical-packs/tests/pitch_agent_template_test.rs` walks up to find `Cargo.lock` (workspace root), then resolves `packs/pitch-agent/` and calls `std::fs::read_to_string` on `worker.toml`, `policies/truth-boundary-ref.toml`, and `skills/bundle.toml`. No `include_str!`, no embedded fixtures — these tests would fail if any of the 17 new files were missing or unparseable.

2. **Tenant-clean test is not just "two loads return Ok"** — the test `pitch_agent_template_is_tenant_clean_across_two_loads` (~80 LOC) loads the pack into two independent `Registry` instances with two distinct tenant labels, then asserts the two `LoadedVertical` instances carry no shared mutable state. This is the cross-tenant negative pin the brief required and matches the round-26 + round-51 tenant-isolation pattern (compile-pinned for the executor surface, runtime-pinned for the vertical-packs surface).

3. **Truth-boundary policy ties into shipped P-01 capability** — `policies/truth-boundary-ref.toml` references the capability schema name landed in PR [#2214](https://github.com/mark-hallam/argo/pull/2214) (P-01 truth-boundary capability schema). Cross-PR reference makes the pack legitimately wire into the platform's existing capability machinery, not declare a parallel surface.

## Gap / follow-up brief candidates

None for this scenario. Per CLAUDE.md "Remaining for v1.0 launch" (e), the Track P queue state is now:

- P-01 [MERGED #2214], P-03 [MERGED #2209], P-04 [MERGED #2281], **P-05 [MERGED #2384 this round]**
- P-02 implementation BLOCKED on audit event-kind split/headroom (separate workstream)
- P-06 / P-07 blocked on P-02 + P-05 — **P-05 unblock reduces their dependency chain by one node**
- P-08 blocked until P-01..P-05 all landed — **with P-05 now MERGED, P-08 dependency on P-05 is satisfied; P-08 remains blocked only on P-02**

Recommendation to next conductor: P-02 implementation is now the single critical node blocking three downstream briefs (P-06, P-07, P-08). Treat P-02 unblock as Track P critical path.

Cost: ≈8K tokens, ≈12min wall (most of which was the 34s cargo cold-build).
