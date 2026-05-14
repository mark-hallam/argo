# claude-argo-acp-triage — Investigate argo-acp (873 LOC, zero workspace consumers)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `c928d8fe` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-argo-acp-triage`.

## Scope

Phase D audit found `argo-acp` (873 LOC) is a non-stub workspace member with zero workspace consumers (no `path = "../argo-acp"` in any other crate's Cargo.toml). Three possibilities:
- Deprecated — should be deleted.
- WIP — should be wired to a consumer or left with a TODO marker.
- Standalone-by-design — like the MCP connectors (dynamically loaded); should be documented as such.

This brief is a triage: investigate, decide, and execute one of three actions.

LOC: ~100 (investigation + execution).

## Files you may touch

**Investigation** (no production changes):
- `crates/argo-acp/Cargo.toml` — read description + history via `git log --follow`.
- `crates/argo-acp/src/lib.rs` — read public surface.
- `crates/argo-acp/README.md` (if exists).
- Workspace `Cargo.toml` — confirm membership.

**Execution** (one of three paths):

**Path A — DELETE** (if deprecated):
- `git rm -r crates/argo-acp/`
- Workspace `Cargo.toml` — remove member.
- Annotate Phase D audit report.

**Path B — WIRE** (if WIP):
- Identify the intended consumer (likely argo-cli or argo-gateway).
- Add `argo-acp = { path = "../argo-acp" }` to consumer's Cargo.toml.
- Author a follow-up brief for whatever the WIP intended.

**Path C — DOCUMENT** (if standalone-by-design):
- `crates/argo-acp/Cargo.toml` — append ` (standalone access control plane; dynamically loaded, not a compile-time workspace consumer)` to description.
- NEW `crates/argo-acp/README.md` — one-page rationale.
- Annotate Phase D audit report.

## Files you must NOT touch

- argo-acp's source code if Path B or C — only Cargo.toml / docs.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] PR description states which path was taken + why.
- [ ] If Path A: `cargo build --workspace` clean post-delete.
- [ ] If Path B: at least one new workspace consumer; build still clean.
- [ ] If Path C: README + description present; phase D report annotated.
- [ ] BUILD_STATUS.md row updated with PR number.

## Branch + PR

- Branch: `claude-argo-acp-triage`
- PR title: `triage: argo-acp orphan-crate disposition (claude-argo-acp-triage)`

## Notes for the implementer

- Mission-claim alignment: production hardening (workspace inventory hygiene).
- Phase D audit report: `docs/skycot/audit/2026-05-14-phase-D-cross-cutting-drift.md` §3.
- The crate name is "acp" — likely "Access Control Plane" or "Agent Control Protocol". Read first; decision flows from the content.
- Drift-prevention: brief makes a single disposition decision; no parallel surfaces added.
