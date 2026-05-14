# claude-canonicalize-parallel-names-round2-high — Canonicalize 7 HIGH-severity duplicate types (Phase D round 2)

**Owner**: claude. **Depends on**: nothing (but most useful after round-1 canonicalization briefs land).
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-canonicalize-parallel-names-round2-high`.

## Scope

Phase D audit found **19 ADDITIONAL parallel-name surfaces** beyond Phase A's 11 (`AuditRow`, `Decision`, `ApprovalDecision`, `JudgeVerdict`, `Role`, `Outcome`, `RestartDecision`). This brief covers the 7 HIGH-severity ones (semantic reuse, not just name overlap). The 10 LOW-severity duplicates are deferred to `mission-canonicalize-parallel-names-round2-low` (post-v1).

**HIGH-severity duplicates** (verified by grep at HEAD):
1. `WorkerId` — `argo-identity/src/types.rs:25` AND `argo-store/src/models.rs:20`
2. `Session` — 7+ crates including `argo-executor-computer-use-linux/src/session.rs:9` + `argo-skill/src/audit_reader.rs:59`
3. `RunResult` — `argo-executor-computer-use/src/executor_types.rs:81` AND `argo-sandbox/src/sandbox.rs:59`
4. `RunRequest` — `argo-sandbox/src/sandbox.rs:28` AND `argo-executor-computer-use/src/executor_types.rs:15`
5. `SkillCandidate` — `argo-skill/src/learn.rs:56` AND `argo-memory/src/skill_candidate.rs:32`
6. `MemoryEntry` — `argo-memory/src/entry.rs:55` AND `argo-vertical/src/memory_schema.rs:52`
7. `JsonRpcRequest` / `JsonRpcResponse` / `JsonRpcError` — `argo-mcp/src/protocol.rs` AND `argo-plugin/src/protocol.rs`

**Per-type strategy** (each is a separate atomic refactor within the PR):

- **WorkerId** → canonical to `argo-identity` (identity owns the concept); `argo-store` re-exports.
- **Session** → canonical to `argo-types` (shared protocol-level concept). Other 6 crates either re-export or rename (some may be domain-specific `CuSession` / `SkillReviewSession` distinct types).
- **RunResult / RunRequest** → canonical to `argo-sandbox` (sandbox owns the run protocol); executor-computer-use imports.
- **SkillCandidate** → canonical to `argo-skill` (skill owns candidate lifecycle); argo-memory either imports or renames to `MemorySkillCandidate` if it's a memory-specific shape.
- **MemoryEntry** → canonical to `argo-memory`; argo-vertical/memory_schema.rs imports or renames its variant.
- **JsonRpc{Request,Response,Error}** → canonical to `argo-mcp/protocol.rs` (MCP is the protocol owner); argo-plugin imports.

LOC: ~800 across the 7 types.

## Files you may touch

For each duplicate type cited above + that type's call sites surfaced by `rg '<TypeName>'`. Tests + Cargo.toml dep edges as needed. ≤5 modified Cargo.toml lines per crate.

## Files you must NOT touch

- Anything outside the listed types' surfaces.
- The 10 LOW-severity duplicates (those are post-v1: `mission-canonicalize-parallel-names-round2-low`).
- The 4 Phase A canonicalization briefs' types (already addressed: AuditRow, Decision, ApprovalDecision, JudgeVerdict/Role/Outcome/RestartDecision).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] For each of the 7 HIGH types: ONE canonical home with re-exports, OR clearly-named distinct types (e.g., `CuSession` vs `SkillReviewSession` if Session genuinely splits 7 ways).
- [ ] `rg 'pub (struct|enum) (WorkerId|Session|RunResult|RunRequest|SkillCandidate|MemoryEntry|JsonRpcRequest|JsonRpcResponse|JsonRpcError)\b' crates/` returns minimum-necessary matches (1 per concept).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes.
- [ ] PR description documents the per-type decision.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-canonicalize-parallel-names-round2-high`
- PR title: `refactor: canonicalize 7 HIGH-severity duplicate types (Phase D round 2) (claude-canonicalize-parallel-names-round2-high)`

## Notes for the implementer

- Mission-claim alignment: production hardening (data integrity + type clarity across substrate).
- Phase D audit report: `docs/skycot/audit/2026-05-14-phase-D-cross-cutting-drift.md` §2.
- The **Session** duplicate is the worst (7+ crates). Read each definition; if 4+ are genuinely distinct (CU session vs skill review session vs MCP transport session), prefer renaming each over forcing a merge.
- Drift-prevention: brief removes parallel surfaces; adds zero new ones.
