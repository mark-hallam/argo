# mission-skill-propagation-loop — cross-worker skill discovery + trust gating

**Owner**: claude. **Depends on**: #948 (Dreams skill-distillation; verify status — if not yet MERGED, this brief blocks). **Tier M — Mission MVP critical path.**

**Authored against**: origin/main 7785708b on 2026-05-10.

## Why

Mission claim: "the swarm gets smarter overnight" — a skill distilled from one worker's
trajectory becomes available to peer workers without manual promotion. Dreams
skill-distillation (#948) handles the per-worker side: trajectory → distilled skill
artifact. The cross-worker layer is missing — Worker B never sees Worker A's promoted
skill at boot. This brief closes that loop with a single integration test asserting
propagation gated by trust grade.

## Approach (concrete)

1. **Verify** at claim-time: confirm #948 is MERGED on `origin/main`. If still
   IN_REVIEW or BLOCKED, post a BLOCKED note on this brief naming #948 as the gate
   and pick a different brief.
2. **Modify or extend** `crates/argo-skill/src/registry.rs`:
   - Add `available_for_worker(worker_id, trust_grade) -> Vec<SkillId>` returning all
     promoted skills in the tenant whose minimum required trust_grade is ≤ the
     caller's trust_grade.
   - Skill rows already carry a `min_trust_grade` field (per #948 schema); reuse, do
     not extend schema.
3. **NEW or extend** `crates/argo-skill/src/discovery.rs` (~150 LOC):
   - `SkillDiscovery::load_for_session(worker_id) -> ArgoResult<Vec<SkillRecord>>`
     reads the registry, returns the worker's available skill set on session boot.
4. **Modify** `crates/argo-cli/src/handlers/worker_actions.rs` — narrowly: at the
   boot-time path where the worker's available action vocab is loaded, also call
   `SkillDiscovery::load_for_session` and merge the result into the available skill
   set passed to the runtime.
5. **Capability**: read-only against `SkillRegistry`; no new capability surface.
6. **Rustdoc**: every `pub` item carries doc + at least one example.

## Files you may touch

- `crates/argo-skill/src/registry.rs` — narrowly: add `available_for_worker` method.
- `crates/argo-skill/src/discovery.rs` (NEW or extend, ≤ 200 LOC)
- `crates/argo-skill/src/discovery_tests.rs` (NEW, ≤ 200 LOC) — 3 unit tests:
  trust_grade gating accepts, trust_grade gating rejects, empty-registry returns empty.
- `crates/argo-skill/src/lib.rs` — narrowly: pub mod line for discovery.
- `crates/argo-cli/src/handlers/worker_actions.rs` — narrowly: boot-time skill-set load.
  Note: this file is on the larger side; if at/over 500 LOC, split per HARD RULE 6
  (mark BLOCKED with "split needed" reason).
- `crates/argo-cli/tests/skill_propagation_e2e.rs` (NEW, ≤ 250 LOC) — 1 integration
  test: Worker A trajectory → distilled skill promoted to registry with
  `min_trust_grade=L2` → Worker B (trust_grade=L2) sees skill on session boot →
  Worker C (trust_grade=L1) does NOT see skill.

## Files you must NOT touch

- Skill schema / migrations — reuse existing `min_trust_grade` field from #948.
- Dreams distillation pipeline (#948) — read-only consumer of its output.
- Trust grade primitives — read-only consumer.
- Any UI / gateway code.

## Acceptance criteria

- [ ] `SkillRegistry::available_for_worker` filters by `min_trust_grade ≤ caller_grade`.
- [ ] `SkillDiscovery::load_for_session` is called at worker session boot.
- [ ] 3+ unit tests covering accept / reject / empty paths.
- [ ] 1 integration test demonstrates A → B propagation with C blocked at lower trust grade.
- [ ] Rustdoc on every pub item.
- [ ] `cargo clippy -p argo-skill -p argo-cli --all-targets -- -D warnings` clean.
- [ ] Every modified file < 500 LOC.

## Branch + PR

- Branch: `claude-mission-skill-propagation-loop`
- PR title: `skills: cross-worker propagation gated by trust grade (mission-skill-propagation-loop)`
