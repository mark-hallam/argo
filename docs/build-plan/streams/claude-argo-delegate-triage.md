# claude-argo-delegate-triage — Investigate argo-delegate (582 LOC, zero workspace consumers)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-argo-delegate-triage`.

## Scope

Phase D audit found `argo-delegate` (582 LOC) is a non-stub workspace member with zero workspace consumers. Phase A had mapped it to Claim 2 (Delegation is the unit), which made it suspicious — a delegation crate that nothing imports.

**Possibilities**:
- WIP for sub-agent delegation (Phase A noted "Subagent delegation, budget tree").
- Deprecated remnant of earlier delegation design before argo-orchestrator absorbed responsibility.
- Standalone-by-design (less likely).

This brief: investigate, decide, execute. Same 3-path structure as `claude-argo-acp-triage`.

LOC: ~100.

## Files you may touch

**Investigation:**
- `crates/argo-delegate/Cargo.toml`, `src/lib.rs`, `README.md` (if exists).
- Workspace `Cargo.toml`.
- `crates/argo-orchestrator/src/lib.rs` — confirm whether orchestrator now owns delegation (to inform whether argo-delegate is duplicated logic).

**Execution** (one of three paths — same template as argo-acp-triage):
- **Path A — DELETE**: `git rm -r crates/argo-delegate/` + workspace Cargo.toml update + annotate audit.
- **Path B — WIRE**: identify consumer (likely argo-orchestrator), add dep, author follow-up brief.
- **Path C — DOCUMENT**: Cargo.toml description + README + annotate audit.

## Files you must NOT touch

- argo-orchestrator's source (if Path B, only touch its Cargo.toml).
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] PR description states which path was taken + why.
- [ ] If Path A: build clean post-delete; if Phase A claim-2 mapping pointed to argo-delegate, update Phase A audit to point to orchestrator instead.
- [ ] If Path B: workspace consumer added; orchestrator's delegation logic NOT duplicated against argo-delegate.
- [ ] If Path C: README + annotated audit.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-argo-delegate-triage`
- PR title: `triage: argo-delegate orphan-crate disposition (claude-argo-delegate-triage)`

## Notes for the implementer

- Mission-claim alignment: production hardening (Claim 2 coverage integrity — if argo-delegate is deprecated, Phase A's claim-2 mapping needs updating to reflect the real owner).
- Phase D audit report: `docs/skycot/audit/2026-05-14-phase-D-cross-cutting-drift.md` §3.
- **Special case**: if argo-delegate is duplicate of argo-orchestrator's delegation logic, this is a Drift Pattern #1 (parallel-system drift). Document explicitly in PR; consider whether to migrate argo-delegate logic INTO argo-orchestrator or DELETE in favor of orchestrator's implementation.
- Drift-prevention: brief MAY remove a parallel surface if Path A taken; otherwise neutral.
