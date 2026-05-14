# L6 — `ghostwriter-prompt-to-ir` (Track L Phase 1, Tier 2)

**Owner**: claude. **Depends on**: L1 MERGED; L2 MERGED.
**Authored against**: origin/main `0ec56ca5` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L6-ghostwriter-prompt-to-ir`.

## Scope

Ghostwriter — the gen-from-prompt → IR pipeline. Operator types a description; LLM emits a structured `SkillGraph`; canvas opens with the draft loaded. Closes the "blank-grid paralysis" failure mode by ensuring canvas is never blank. LOC ~900. Branch: `claude-L6-ghostwriter-prompt-to-ir`. PR title: `feat(skill-ir): Ghostwriter — gen-from-prompt → IR (L6)`.

**Specifics (verbatim from L0 spec)**:

- Operator types: *"Build a skill that reconciles bank statements weekly: fetch the bank export, match line items against the GL, flag deltas over $500 for CFO approval."*
- Backend: LLM call (provider via existing manifest system) with structured-output prompt that returns a `SkillGraph` JSON.
- Validates against L1 schema; rejects malformed output.
- Schema-validation failures retry once with a self-correction prompt.
- Stores via L2; returns skill_id + version.
- Frontend: redirects to L4 canvas with draft loaded.
- LLM routing: **strongest available, regardless of the worker's runtime provider** (operator-locked per L0 §"Operator decisions" 3). Quality matters more than parity.

## Files you may touch

- NEW `crates/argo-skill/src/ir/ghostwriter.rs` — pipeline + LLM call.
- NEW `crates/argo-skill/src/ir/ghostwriter_prompts.rs` — prompt templates.
- NEW `crates/argo-skill/src/ir/ghostwriter_tests.rs` — tests.
- NEW `crates/argo-cli/src/handlers/gateway/routes_ghostwriter.rs` — `POST /api/v1/skills/draft-from-prompt`.
- NEW `web/app/(app)/workers/[id]/skills/new/page.tsx` — prompt entry surface.
- NEW `web/components/skycot/canvas/GhostwriterForm.tsx` — form component.
- NEW `web/lib/api/ghostwriter.ts` — client wrapper.
- `crates/argo-cli/src/handlers/gateway/routes_dispatch_post_*.rs` — register route. ≤10 modified lines.

## Files you must NOT touch

- L1 schema — read-only.
- L4 SkillCanvas — read-only (this brief routes INTO L4, doesn't modify it).
- Provider manifest system — use existing routing.

## Acceptance criteria

- [ ] Five sample prompts (one per role — Recruiter / AP Specialist / Vendor Manager / Support Specialist / Deal Desk Analyst) generate valid IR graphs.
- [ ] Generated graphs have ≥ 4 nodes for non-trivial prompts.
- [ ] Schema-validation failures retry once with a self-correction prompt.
- [ ] Tenant-scoped + audit-emitting.
- [ ] 8 unit tests pass.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `pnpm typecheck` clean in `web/`.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L6-ghostwriter-prompt-to-ir`
- PR title: `feat(skill-ir): Ghostwriter — gen-from-prompt → IR (L6)`

## Notes for the implementer

- LLM routing: pick strongest available model regardless of worker runtime provider.
- Frontend route must redirect to L4 canvas with draft loaded after success.
- See `streams/L0-skill-canvas-track-overview.md` for full track context (especially "Bi-directional canvas" + "Critical UX invariant: canvas is never blank").
