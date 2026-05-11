# Mission load-bearing wire — wire SkillStore::available_for_worker into agent session boot (M6 precursor 3/3)

**Owner**: claude. **Depends on**: `mission-skill-store-available-for-worker` MERGED (precursor 2/3 — provides the lookup); `argo-agent-runner::turn` SHIPPED (verified at `crates/argo-agent-runner/src/turn.rs`).
**Authored against**: origin/main `5e1ef545` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-session-boot-wire`.

> SOURCE: precursor authored 2026-05-11 by conductor to unblock `mission-skill-propagation-loop` (Tier M6). Today when a worker session boots, it does NOT consult the skill store at all — the agent prompt is built from worker identity + memory + role, not from "skills this worker can use". This brief adds the missing wire: at session boot, query `SkillStore::available_for_worker(worker_id)` and inject the skill list into the agent's system prompt as a structured "Available skills" block.

## Scope

1. **Locate the session-boot path**. The agent prompt assembly likely lives in `argo-agent-runner::turn` (per surface scan: `turn.rs`, `turn_helpers.rs`, `turn_memory.rs`). **Implementer's first task**: trace where the system prompt is finalized before the first LLM call. Citation in PR body.

2. **At boot time** (before the first turn), call `SkillStore::available_for_worker(worker_id)`. The returned Vec<Skill> is rendered into a markdown block:
   ```
   ## Available skills
   - **skill_name** (v1.2): one-line description
   - ...
   ```
3. **Inject** the block into the system prompt. Position: AFTER the worker's persona/charter/role block, BEFORE the memory composition block.

4. **Empty case**: if no skills available, omit the entire "Available skills" block (don't render "No skills available." — that pollutes context with negative info).

5. **Error case**: if `available_for_worker` errors (e.g. worker not found), log `tracing::warn!` and proceed without the skill block. Don't fail session boot.

This is NOT in scope: dynamic skill discovery mid-turn (only at boot); skill invocation logic (separate concern); rendering skill_body content in the prompt (only name + description); per-action skill filtering (TODO follow-up if needed).

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-agent-runner/src/turn.rs` OR `turn_helpers.rs` (verify which holds prompt assembly in first 5min) — narrowly scoped: ADD one call site to `SkillStore::available_for_worker` + the render-and-inject logic. ≤ 50 LOC of additions.
- `crates/argo-agent-runner/src/turn_skills.rs` (NEW, ≤ 150 LOC) — the skill-block renderer + `inject_skills_block` helper. Sibling module keeps `turn.rs` under the 500-LOC ceiling.
- `crates/argo-agent-runner/src/turn_skills_tests.rs` (NEW, sibling) — 6 unit tests: empty-list-omits-block; populated-list-renders-correctly; injected-position-is-after-persona-before-memory; error-from-lookup-warns-and-skips; legacy-worker-treated-as-lowest-grade; markdown-format-stable.
- `crates/argo-agent-runner/Cargo.toml` — narrowly scoped: add `argo-skill` dep IF not already present (verify first).

## Files you must NOT touch

- `crates/argo-skill/src/store.rs` — store API stable; only consuming `available_for_worker` here
- `crates/argo-runtime/**` — runtime is upstream
- `crates/argo-identity/**` — identity surface unchanged
- Any non-agent-runner crate

## Acceptance criteria

- [ ] Session boot calls `SkillStore::available_for_worker(worker_id)` exactly once
- [ ] Returned skills rendered as markdown "Available skills" block
- [ ] Block injected AFTER persona/charter block, BEFORE memory composition
- [ ] Empty list → block omitted entirely (not "No skills.")
- [ ] Lookup error → `tracing::warn!` + omit block, session boot succeeds
- [ ] No change to per-turn behavior (only boot path)
- [ ] 6 unit tests
- [ ] No new public API outside the listed exports
- [ ] `cargo build -p argo-agent-runner -p argo-skill` clean
- [ ] `cargo clippy -p argo-agent-runner -p argo-skill --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-agent-runner -p argo-skill` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-session-boot-wire`
- PR title: `agent-runner: inject available-for-worker skills into session boot prompt (mission-skill-session-boot-wire)`

## Notes for the implementer

- **First 5min investigation is critical**: trace `turn.rs::start_session` (or equivalent boot entry point). Confirm the prompt-assembly site. Cite line numbers in the PR body.
- The markdown block format MUST be stable across session boots — the test asserts exact format. Tweak ONLY if a real prompt-engineering reason exists, and document.
- Don't try to render skill body content in the prompt — too verbose. Just `name (version): short_description`.
- The "error → warn + skip" semantics are load-bearing: a missing skill_store mid-deployment shouldn't crash agent sessions.
- This is precursor 3/3. The next brief (rewritten `mission-skill-propagation-loop`) builds the end-to-end demo: Worker A creates a skill, Worker B's session boot sees it via this wire, can invoke it autonomously.
