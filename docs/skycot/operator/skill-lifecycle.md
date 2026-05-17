# Skill Lifecycle — From CU Trajectory to Cross-Worker Propagation

Operator runbook for the **skills-learned loop** that closes
SKYCOT mission claim 4: "a skill learned by one worker propagates
to other workers without redeploy." The integration test
`crates/argo-cli/tests/skill_loop_closure_smoke.rs` asserts every
waypoint listed here; running it is the fastest health check.

## The closed loop

```text
  workerA (tenant A) completes a successful CU action
      │  argo_skill::learn::should_propose(metric, threshold)
      ▼
  SkillCandidate (name + trajectory + tokens + metric)
      │  SkillStore::propose_for_review(SKILL.md, session, vertical)
      ▼
  row in `skill` table, lifecycle_state = pending_review
      │  argo skill review approve <id> → SkillApproved audit
      ▼
  same row, lifecycle_state = active
      │  argo skill publish <name> --tag 0.1.0
      ▼
  bundle under mock://registry/oci/<name>/<tag>/manifest.json
      │  OCI sync → ingest into the shared library SessionStore
      ▼
  library row with provenance source: published:verified:vendor
      │  workerB session boot:
      │  LayeredLookup::with_library + SkillRelevanceSelector
      ▼
  workerB's prompt context includes the published skill
```

## Run the smoke locally

```bash
cargo test -p argo-cli --test skill_loop_closure_smoke -- --nocapture
```

Completes in <3 seconds on a typical dev laptop. The panic message
on failure names the broken waypoint.

## Debugging when a waypoint fails

| Waypoint | What to inspect when it fails |
| --- | --- |
| Threshold gate | `argo_skill::learn::should_propose`; trajectory's `success_metric` |
| Candidate persists | `SELECT * FROM skill WHERE lifecycle_state = 'pending_review'` |
| Approve flips state | `argo audit recent --kind skill.approved` |
| Publish writes bundle | `find <registry-root>/oci -type f` — manifest+bundle+blob |
| Library has the row | `SELECT name, source FROM skill` in `$ARGO_SKILL_LIBRARY_DB` |
| Selector returns the skill | Verify `skill.vertical_id == worker.vertical_id` and trust grade |

## Cross-tenant boundary

Tenant A's workspace-private skill (source `user`, never published)
**must not** appear in tenant B's prompt. The test enforces this
explicitly. A flipped negative assertion = security incident; check
the gateway's library-store binding
(`crates/argo-agent-runner/src/turn_helpers.rs:112`,
`ARGO_SKILL_LIBRARY_DB` env var) — a misconfigured library path can
point one tenant's view at another tenant's database.

## Audit chain check

After the loop, the per-session chain must verify clean. Operators:

```bash
argo audit verify-chain --json
```

The chain MUST include `tool_call → tool_result →
skill.candidate.promoted → skill.approved` for the originating
session.

## Known gap (open follow-up)

`SkillCandidate::to_skill_md` emits `capabilities:\n  - cu\n` — a
plain string list — but `argo_types::Capability` is a
`#[serde(tag = "kind")]` tagged enum, so the raw output fails to
parse via `SkillStore::propose_for_review`. The closure-trace test
strips the malformed block as a compensating workaround. Follow-up
brief should either drop the `capabilities` block from `to_skill_md`
(currently dead-letter — nothing references "cu" as a capability) or
serialize a valid `Capability::ToolDispatch { tool_name_glob: "cu*" }`
arm.

## Launch-blocker semantics

Mission claim 4 graduates from PARTIAL to PROVEN when this test is
green on main. A future PR that breaks it makes the launch demo's
"workerA teaches workerB" segment non-reproducible — treat a red
`skill_loop_closure_smoke` test as a mission-claim-4 launch blocker.
