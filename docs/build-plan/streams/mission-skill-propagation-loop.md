# Mission load-bearing wire — cross-worker skill propagation loop (Tier M6) — REWRITTEN

**Owner**: claude. **Depends on**:
- `mission-skill-min-trust-grade-column` MERGED (precursor 1/3 — `skill.min_trust_grade` column)
- `mission-skill-store-available-for-worker` MERGED (precursor 2/3 — `SkillStore::available_for_worker` lookup)
- `mission-skill-session-boot-wire` MERGED (precursor 3/3 — agent prompt injects skill list at boot)
- `mvp-worker-trust-grade-column` MERGED #1273 (worker trust_grade)

**Authored against**: origin/main `5e1ef545` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-skill-propagation-loop`.

> **REWRITTEN 2026-05-11**: original brief was BLOCKED because it cited `argo-skill/src/registry.rs` (doesn't exist — actual is `store.rs`) and a `min_trust_grade` field that didn't exist. Three precursor briefs now author the missing surfaces; this rewrite delivers the propagation demo on top.
>
> SOURCE: MISSION_BACKLOG.md Section 3 Claim 4 + Tier M6 in BUILD_STATUS.md. Mission promise: skill learned by Worker A in tenant T propagates to Worker B in same tenant (via shared skill store + trust-gated discovery). Worker C with too-low trust grade does NOT see it.

## Scope

Prove the end-to-end propagation loop via an integration test:

1. **Setup**: 3 workers in tenant T with trust grades (A=80, B=60, C=20). Skill X with `min_trust_grade=50`.
2. **Worker A creates Skill X**: via `SkillStore::install` (existing API).
3. **Worker B's session boots**: `available_for_worker` returns Skill X (60 ≥ 50). Boot wire injects "Available skills" block including Skill X.
4. **Worker C's session boots**: `available_for_worker` excludes Skill X (20 < 50). Block omits X.
5. **Assertions**: B's prompt contains skill X; C's doesn't. B can invoke X; C cannot discover X.

This is **integration-only** — no new core APIs. All three precursors deliver the primitives; this brief is the wire-up demo + integration test + CLI surface for inspecting per-worker available-skills.

**NEW CLI**: `argo worker skills <worker_id> [--available-only] [--json]` — calls `SkillStore::available_for_worker` and prints. Useful for operators debugging propagation.

This is NOT in scope: cross-tenant propagation (covered by `mission-cross-tenant-trust-graph`); negative trust signals; auto-revocation when worker's trust drops; per-skill version selection.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `crates/argo-cli/src/handlers/worker_skills.rs` (NEW, ≤ 200 LOC) — `argo worker skills` CLI handler.
- `crates/argo-cli/src/cli_worker_cmds.rs` (verify path) — narrowly scoped: ADD `skills` subcommand enum variant.
- `crates/argo-cli/src/handlers/worker_skills_tests.rs` (NEW, sibling) — 5 unit tests on the CLI rendering: empty-output; populated-output; --json-format; missing-worker-errors; --available-only-flag-respected.
- `crates/argo-cli/tests/skill_propagation_e2e.rs` (NEW, ≤ 300 LOC) — THE end-to-end integration test described in Scope. Seeds 3 workers + 1 skill, boots each agent session, asserts prompt contents.

## Files you must NOT touch

- `crates/argo-skill/src/store.rs` `available_for_worker` — provided by precursor 2/3; reuse only
- `crates/argo-agent-runner/src/turn*.rs` — boot-wire is provided by precursor 3/3; reuse only
- `crates/argo-store/migrations/**` — no new schema (precursor 1/3 already added the column)
- Any non-cli crate

## Acceptance criteria

- [ ] `argo worker skills <id>` CLI works, lists skills via `available_for_worker`
- [ ] `--available-only` filters out NULL-min-trust skills (shows only restricted ones)
- [ ] `--json` outputs structured envelope
- [ ] Missing worker_id → explicit error
- [ ] Integration test seeds 3 workers (trust 80, 60, 20) + skill with min_trust=50
- [ ] Test asserts Worker A (80) and Worker B (60) see skill X in boot prompt
- [ ] Test asserts Worker C (20) does NOT see skill X
- [ ] Test asserts the worker prompt's "Available skills" block format is stable
- [ ] 5 unit tests + 1 integration smoke
- [ ] No new public API outside the listed CLI surface
- [ ] `cargo build -p argo-cli` clean
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli` passes (skip pre-existing flakes)
- [ ] Final `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] Every modified file < 500 LOC
- [ ] BUILD_STATUS.md updated with PR number

## Branch + PR

- Branch: `claude-mission-skill-propagation-loop`
- PR title: `mission: cross-worker skill propagation loop + argo worker skills CLI (mission-skill-propagation-loop)`

## Notes for the implementer

- This brief is the **payoff** of the 3-precursor chain. By the time you're claiming it, all three should be MERGED. Verify in first 5min: `from_audit_history` exists (#1276), `available_for_worker` exists (precursor 2), `turn_skills.rs` exists (precursor 3).
- The integration test is the most important deliverable — it's what proves the mission claim. Make it ironclad: deterministic, no flaky timing, no network deps.
- The `argo worker skills` CLI is for operator debugging — keep output simple and consistent with `argo skill list` style.
- The `--available-only` flag is for "show me what's been GATED for this worker" — useful when an operator's worker isn't seeing an expected skill.
- DO NOT try to add core APIs here. If you find precursor surfaces don't actually meet your needs, BLOCK and request precursor revisions rather than expand scope.
