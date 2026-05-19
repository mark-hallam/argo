# Stream P0-1 — surface per-worker learned skill overlays in agent prompt

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `80ffa124` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-pm-skill-overlay-load`.
**Source**: skycot-pm-worker REPORT.md §3.6 + §11 P0 #1 (highest-leverage finding).

## Scope

Close the learning-reuse loop. Today `per_worker_learning_cron` writes overlay skills into `worker_skill (worker_id, skill_id, proficiency)` (migration 101) when a worker has earned ≥3 positive ratings averaging ≥0.75 — but `load_prompt_skills_with_library` (the function the agent runner calls every turn to populate the system prompt) **does not consult `worker_skill`**. Result: the cron's promotion does nothing visible to the worker on its next session.

This brief threads a `worker_id` parameter through `load_prompt_skills_with_library` and adds a `SkillStore` accessor that returns the union of `(workspace, library, worker_skill-for-this-worker)` rows. When a `worker_id` is known at session-bind time, the overlay skills materialise in the prompt automatically. When no `worker_id` is known (operator REPL session), behaviour is unchanged.

This is the **single highest-leverage fix** identified in the investigation. Without it the "learning teammate" framing is dishonest.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-agent-runner/src/turn_helpers.rs` — extend `load_prompt_skills_with_library` signature to take `Option<WorkerId>`; when `Some`, join `worker_skill` overlays into the layered lookup.
- `crates/argo-skill/src/store.rs` — narrowly scoped: add `pub async fn list_for_worker(&self, worker_id: WorkerId) -> ArgoResult<Vec<Skill>>` that joins `worker_skill` × `skill` and filters by `lifecycle_state = 'active'`.
- `crates/argo-skill/src/store_tests.rs` (NEW or existing sibling) — coverage for `list_for_worker`.
- `crates/argo-agent-runner/src/turn.rs` — narrowly scoped: pass `session.worker_id` (already on `Session`) into the helper. No other edits.
- `crates/argo-agent-runner/src/turn_helpers_tests.rs` (NEW sibling) — integration test: seed a worker, attach an overlay via `worker_skill`, run `load_prompt_skills_with_library` with the worker_id, assert the overlay body appears.

## Files you must NOT touch

- `crates/argo-skill/src/per_worker_learning_cron.rs` — the writer is already correct.
- `crates/argo-store/migrations/` — `worker_skill` table already exists (migration 101).
- Any `verticals/` content — this is a runtime wiring fix, not a vertical change.
- Anything in `web/` — frontend is unaffected.

## Acceptance criteria

- [ ] `SkillStore::list_for_worker(worker_id)` returns active skills joined via `worker_skill`, deduplicated against workspace + library.
- [ ] When `load_prompt_skills_with_library` is called with `Some(worker_id)`, the returned `Vec<Skill>` includes every active overlay row from `worker_skill` for that worker.
- [ ] When called with `None`, behaviour is byte-for-byte identical to the current implementation (regression test).
- [ ] Overlay skills are merged into the same `SkillRelevanceSelector` pipeline as library skills (no precedence bug).
- [ ] Integration test: seed worker `W1`, `attach_worker_overlay(W1, skill_X)`, start a session for `W1`, capture the system prompt → contains skill_X body. Test in `crates/argo-cli/tests/per_worker_overlay_surfaces.rs` (NEW, follows the existing `cross_tenant_skill_discovery_negative.rs` shape).
- [ ] `cargo clippy -p argo-agent-runner -p argo-skill --all-targets -- -D warnings` clean (crate-scoped — claude tabs exempt from workspace gate per `SWARM_PROTOCOL.md` Verification).
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean — claim explicitly in PR body.
- [ ] `cargo test -p argo-agent-runner -p argo-skill -p argo-cli` passes.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-pm-skill-overlay-load`
- PR title: `agent-runner: load per-worker skill overlays into prompt (mission-pm-skill-overlay-load)`

## Notes for the implementer

- The `worker_skill` schema is `(worker_id TEXT, skill_id TEXT, proficiency INTEGER, created_at TEXT)` per `crates/argo-store/migrations/101__worker_skill.sql`. Join against `skill` on `skill_id`.
- The `Skill` struct has `lifecycle_state` — only `Active` rows surface in the prompt. Pending/Archived/Rejected stay hidden.
- The relevance selector (`SkillRelevanceSelector`) should treat overlays as **library-tier** for ranking purposes (i.e. they compete with library skills, not workspace skills). Don't introduce a third tier — that's scope creep.
- Do NOT change `attach_worker_overlay` or any cron-side code. The cron is correct; only the read path is broken.
- The migration is **already applied** — no migration changes needed in this brief.
- Performance: in worst case a worker has hundreds of overlays. The join should be indexed (the existing `worker_skill_skill_idx` covers it). Verify the query plan in the test.

## Out of scope (follow-up briefs)

- A CLI surface for listing per-worker overlays (`argo worker skills list W1`).
- An "explain why I'm showing this skill" audit row.
- Cross-vertical overlay bleed prevention (current isolation by `vertical_id` is sufficient).
