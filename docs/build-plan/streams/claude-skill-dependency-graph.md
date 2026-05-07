# claude-skill-dependency-graph — `argo skill deps <id>` DAG output

**Owner**: claude. **Depends on**: existing `Skill` body parsing
(`crates/argo-skill/src/parse.rs`); existing `SkillStore`.

## Why

A skill body can reference other skills (by id, by name) or call patterns
that imply dependence (workflow includes, sub-skill invocations). Today there
is no way to ask "what does this skill rely on?" or "what would break if I
rolled back skill X?" Building that picture from raw markdown by hand is
error-prone. A dependency graph turns "this skill is risky" into a defensible
claim.

## Scope

- `argo skill deps <id>` outputs the local DAG for skill `<id>`:
  - Markdown form: indented tree (each level prefixed with `└─` / `├─`),
    capped at depth 5 (`--max-depth <N>` to override).
  - JSON form (`--json`): `{ root: <id>, edges: [{ from, to, kind }], depth: <N> }`.
- `kind` is one of `calls` (explicit invocation), `references` (mention only),
  `extends` (frontmatter-declared base).
- `--reverse` flag flips: "what skills depend ON this one?" — same output shape.

## Approach

1. **NEW** `crates/argo-skill/src/deps.rs` (~280 LOC):
   - `pub fn extract_refs(body: &str, frontmatter: &SkillFrontmatter) -> Vec<SkillRef>`.
   - `pub async fn build_graph(store: &SkillStore, root: &str, max_depth: u32) -> ArgoResult<DepGraph>`.
   - `pub async fn build_reverse_graph(store: &SkillStore, target: &str, max_depth: u32) -> ArgoResult<DepGraph>`.
   - Recognised patterns (regex-light, no new deps): `argo skill (run|use) <id>`,
     ``\`skill:<id>\` ``-style mentions, frontmatter `extends:` field.
2. **NEW** `crates/argo-cli/src/handlers/skill_deps.rs` (~220 LOC) renders
   tree (markdown) + JSON.
3. Cycle detection: refuse to recurse into already-visited nodes; mark them
   with `(cycle)` annotation in markdown output.

## Files you may touch

- BLANKET CLI ALLOWLIST applies.
- `crates/argo-skill/src/deps.rs` (NEW, ≤ 320 LOC).
- `crates/argo-skill/src/lib.rs` — narrowly: 1 `pub mod deps;` line + re-export.
- `crates/argo-cli/src/handlers/skill_deps.rs` (NEW, ≤ 280 LOC).
- `crates/argo-cli/src/cli.rs` — narrowly: add `SkillDeps` variant.
- `crates/argo-cli/tests/skill_deps_graph.rs` (NEW, ≤ 250 LOC).

## Files you must NOT touch

- `crates/argo-skill/src/parse.rs` — read-only consumer; do NOT change parser semantics.
- Migration files — derivation is from in-memory bodies, no new schema.
- `crates/argo-skill/src/conflicts.rs` — orthogonal concern.

## Acceptance criteria

- [ ] `argo skill deps <id>` returns a tree of references.
- [ ] `--reverse` flips direction.
- [ ] `--max-depth <N>` honored; default 5.
- [ ] Cycles detected and annotated, not infinite-looped.
- [ ] JSON output validates against documented schema.
- [ ] 7+ tests: linear chain, branching, cycle, depth cap, reverse, JSON shape, no-deps.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] Every authored file < 500 LOC.

## Branch + PR

- Branch: `claude-skill-dependency-graph`
- PR title: `skills: argo skill deps <id> dependency DAG (claude-skill-dependency-graph)`

## Notes

- Pattern set is deliberately small — parser is heuristic, not a true AST.
  Prefer false negatives over false positives. Document the exact patterns in the file header.
- Future work: refresh on body change via a stored cache table; out of scope here.
