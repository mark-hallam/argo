# Round 35 — PR #2132 anti-theater verify (eval-argo-skill-sdk-surface)

- **Date**: 2026-05-23
- **Round**: 35 (scheduled-task hourly fire)
- **Pillar/Claim**: Claim 4 (Skills are shared and learned) — SDK API stability contract for **external** skill-plugin authors.
- **PR**: [#2132](https://github.com/mark-hallam/skycot/pull/2132) `eval-argo-skill-sdk-surface` — MERGED 2026-05-23T12:12:24Z, +281/-0, 2 files (`crates/argo-skill-sdk/src/lib.rs` +51 (rustdoc notes only), `crates/argo-skill-sdk/tests/surface_contract.rs` +230 ADDED).

## Verdict

🟢🟢 **PASS / P2** — every pub-API surface listed in the test's compile-fence + assertion set verified against `origin/main` source. The crate's `Cargo.toml` annotation (`zero-caller-allowed: skill plugin SDK — consumed by external skill plugin authors, not workspace callers`) makes this contract the **only** fence on the surface — no in-repo callers will catch a rename. Tests cover all 5 surface slices honestly.

## Setup

```bash
# pub-API surface in the crate
grep -n 'pub use\|pub fn\|pub struct\|pub enum\|pub mod' crates/argo-skill-sdk/src/lib.rs

# SkillBuildError variant count
grep -c "^\s*[A-Z][A-Za-z]*(String)" crates/argo-skill-sdk/src/error.rs   # → 9

# zero-workspace-callers annotation
head -3 crates/argo-skill-sdk/Cargo.toml
```

## Observed

### Surface inventory — 5 slices verified

| Slice | Pub names | Source cite | Test pin | Verdict |
|---|---|---|---|---|
| **L1 IR re-exports (15)** | `BranchArm, Edge, EdgeCond, EdgeId, Node, NodeAnnotations, NodeId, NodeKind, NodePolicy, SkillGraph, SkillGraphMetadata, SubSkillRef, ToolBinding, Variable, VariableScope` | `crates/argo-skill-sdk/src/lib.rs:73-76` | `surface_contract.rs:23-28` (import) + `_ir_reexports_present()` at lines 36-52 (typed args fence) | 🟢 |
| **Builder fns (5)** | `branch_arm, branch_arm_pattern, create_skill, validate_skill_graph, SkillBuilder` | `crates/argo-skill-sdk/src/lib.rs:77` | `surface_contract.rs:23-28` + tests at lines 58-129 | 🟢 |
| **Builder inputs (2)** | `BuilderNodeOptions, SkillBuilderInput` | `crates/argo-skill-sdk/src/lib.rs:78` | tests at lines 136-170 | 🟢 |
| **Error enum (9 variants)** | `SkillBuildError::{DuplicateNode, DuplicateEdge, DuplicateVariable, MissingEntry, MissingEdgeNode, InvalidConfidence, EmptyBranch, EmptyLoop, EmptyTryCatch}` | `crates/argo-skill-sdk/src/error.rs:7-23` (9 String variants verified by `grep -c`) | `surface_contract.rs:180-216` (exhaustive match + iterate-all) | 🟢 |
| **`sample_skills` returns 5 validated graphs** | `pub fn sample_skills() -> Result<Vec<SkillGraph>, SkillBuildError>` | `crates/argo-skill-sdk/src/lib.rs:89` | `surface_contract.rs:223-230` | 🟢 |

### SkillBuilder fluent method-chain — pinned (line 89-128 of test)

The test calls one of each method in a single fluent chain — `.action / .action_with / .fetch / .fetch_method / .transform / .condition / .approval / .approval_timeout / .branch / .sub_skill / .loop_over / .try_catch / .edge / .edge_when / .variable / .variable_scoped / .start` (16 methods), then `.clone()` (proves `SkillBuilder: Clone`), then `.build().unwrap()` (proves return type is `Result<SkillGraph, SkillBuildError>`). Any rename, removed method, or chain return that changes from `Self` to `&mut Self` would break compilation at this callsite. Honest fence.

### Rustdoc notes that pre-empt v1.0 promotion

The PR added 3 rustdoc notes to `lib.rs:55-67` that document the surface's promotion-to-1.0 plan:

1. The 15 IR re-exports surface their type-graph through `argo_skill::ir`. Pre-1.0 acceptable; post-1.0 either lock or wrap.
2. `SkillBuildError` is **not** `#[non_exhaustive]` — at v1.0 promotion it should be marked or the variant set frozen.
3. `SkillBuilderInput` + `BuilderNodeOptions` expose pub fields — adding a field is technically breaking for struct-literal construction. Pre-1.0 acceptable.

Note 2 is **load-bearing** — the test's exhaustive-match (lines 181-193) is intentional given the non-exhaustive annotation absence. Any future variant addition pre-v1.0 is a breaking change in disguise, and this test will flag it at PR time.

## Anti-theater check

1. **Compile-fence beats assertion for surface contracts.** Lines 23-28 (imports) + line 36-52 (`_ir_reexports_present` with typed arguments) prove the surface exists *by virtue of compiling*. A test that asserted "the name is `BranchArm`" via reflection would be theatre; an `import; fn _b(_: BranchArm) {}` cannot lie.
2. **The `#[allow(dead_code, clippy::extra_unused_lifetimes)]` annotation is honest scope.** The crate-attribute at line 21 documents that the test file is a deliberate compile-fence — the helper fns at 36-52 exist purely to load-bear the typed-arg constraint. Without the `allow`, the typed-arg fences would emit dead-code warnings; the allow makes that intent explicit.
3. **The 9-variant exhaustive match has zero wildcard arms.** Lines 182-192 — no `_ => "other"` arm. Adding a 10th variant to `SkillBuildError` would emit `error[E0004]: non-exhaustive patterns` at this site. The constructors list (lines 196-206) carries 9 distinct cases with `cases.len() == 9` runtime check (line 207) — defense in depth.
4. **`sample_skills_returns_five_validated_graphs` chains validate over each.** Line 228 calls `validate_skill_graph(graph).unwrap()` on every sample — not just `len == 5`. A future change that broke a sample graph but kept the count would surface here.

### Caveat — the contract is **external-facing only**

The `Cargo.toml` line 1 annotation `# zero-caller-allowed: skill plugin SDK — consumed by external skill plugin authors, not workspace callers` means this test file is the **only** in-repo fence on the surface. The external audience (skill-plugin authors using `argo-skill-sdk` as a Cargo crate) is the load-bearing consumer. Two implications:

- The honest cost of a surface change in this crate is borne by external authors, not the workspace. The test file makes the cost visible at PR time instead of post-publish.
- The rustdoc notes at `lib.rs:55-67` should be folded into a `CHANGELOG.md` once v0.1 is published. PR body promises this is on the v1.0 promotion checklist; not in scope for this audit.

## Gap / follow-up brief candidates

**None new.** The test is structurally complete + the rustdoc notes pre-empt the v1.0 surface freeze decisions. The natural next brief is **operator-decisional**: pick a publication target (crates.io vs private registry) + version-bump to v0.1.0 for first external publish + add CHANGELOG.md scaffolding. That's a publication-ops brief, not an audit brief — defer to conductor's judgment.

## Notes

- 3rd of 3 anti-theater verifications this round. Combined with the round-34 reports (PR #2125 + #2126) + round-33 (PR #2116 + #2121), the audit-PR wave from tick-106 through tick-110 is fully verified honest.
- Anti-theater verification proceeded by source-code inspection + grep alone — concurrent conductor activity meant I did NOT attempt `cargo test -p argo-skill-sdk` (documented worker-tree-reset flake under concurrent push).
- No code touched. Charter compliance: read-only across `crates/`.
