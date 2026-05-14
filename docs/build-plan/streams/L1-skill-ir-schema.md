# L1 — `skill-ir-schema-contract` (Track L Phase 1, **CONTRACT, Tier 0**)

**Owner**: claude. **Depends on**: nothing.
**Authored against**: origin/main `0ec56ca5` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh L1-skill-ir-schema`.

> **CONTRACT-TIER brief — must claim FIRST. Suppresses L2/L3/L4/L5/L6/L4-gw claims until L1 MERGED.** Drives Track L (Skill Canvas, 17 briefs total per `streams/L0-skill-canvas-track-overview.md`).

## Scope

Define the **Skill IR schema** — pure types only. No storage, no runtime, no UI. The IR is the canonical structured representation of every skill regardless of how it entered the system (watch, demo, natural-language teaching, canvas editor, brief, gen-from-prompt).

LOC budget: ~500. Branch: `claude-L1-skill-ir-schema`. PR title: `feat(skill-ir): IR schema + types (L1 CONTRACT)`.

**Specifics (verbatim from L0 spec)**:

- Define `SkillGraph`, `Node`, `Edge`, `NodeKind` (9 variants: action / condition / branch / await-approval / fetch / transform / sub-skill-call / loop / try-catch), `EdgeCond` (always / on-success / on-failure / on-pattern), `NodeAnnotations` (confidence, sample-count, last-used, success-rate, policy, tool binding), `Variable`, `SubSkillRef`.
- JSON serialization with `serde(rename_all = "camelCase")` for TS interop.
- Canonical JSON form for hashing (sorted keys, no whitespace).
- Canonical hash: `sha256(canonical_json(graph))` → 256-bit hex.
- TS mirror types in `web/lib/types/skill-ir.ts` — hand-maintained; Rust is canonical; TS must match field-for-field.

## Files you may touch

- NEW `crates/argo-skill/src/ir/mod.rs` — module entrypoint.
- NEW `crates/argo-skill/src/ir/types.rs` — Node/Edge/Graph types.
- NEW `crates/argo-skill/src/ir/serde.rs` — JSON serialization with stable field ordering.
- NEW `crates/argo-skill/src/ir/types_tests.rs` — schema roundtrip tests.
- `crates/argo-skill/src/lib.rs` — narrowly: `pub mod ir;` declaration. ≤5 modified lines.
- NEW `web/lib/types/skill-ir.ts` — TS mirror types.

## Files you must NOT touch

- `crates/argo-skill/src/version.rs` — locked; L2 extends it.
- Any storage layer — locked; L2 owns sqlite write path.
- Any gateway or web component — locked; later phases own those.

## Acceptance criteria

- [ ] All 9 node kinds typed + JSON-roundtrip-tested.
- [ ] Canonical hash function: `sha256(canonical_json(graph))` → 256-bit hex string.
- [ ] TS types compile + structurally match Rust (one fixture round-trips Rust→JSON→TS→JSON→Rust).
- [ ] `cargo doc -p argo-skill` clean; rustdoc on every public type.
- [ ] `cargo clippy -p argo-skill --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-skill ir::` passes.
- [ ] No file > 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-L1-skill-ir-schema`
- PR title: `feat(skill-ir): IR schema + types (L1 CONTRACT)`

## Notes for the implementer

- This is **Tier 0 CONTRACT**: L2/L3/L4/L4-gw/L5/L6 all wait for this to MERGE. Ship clean and fast.
- Pure-types brief; no behavior change in the runtime.
- See `streams/L0-skill-canvas-track-overview.md` for full architecture context.
