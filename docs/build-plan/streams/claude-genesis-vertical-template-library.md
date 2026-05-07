# Genesis vertical template library — pre-built starting points for common roles

**Owner**: claude. **Depends on**: none (extends existing Genesis crate).

## Scope

Genesis should not start from a blank slate every time. For common
roles (sales-sdr, customer-support, marketing-intern, finance-ar,
marketplace-seller) it should load a pre-built **template** that
already has a sensible system prompt skeleton + action shortlist +
memory layer + ~10 eval seeds. The role description then **overrides**
specific fields rather than synthesising everything from scratch.

The 5 demo verticals (#937) are the seed set.

## Approach (concrete)

1. **NEW** `crates/argo-genesis/templates/<role>.toml` (5 files):
   - `sales_sdr.toml`, `customer_support.toml`,
     `marketing_intern.toml`, `finance_ar.toml`,
     `marketplace_seller.toml`
   - Each ~80 LOC: `[meta]`, `[prompt_skeleton]`, `[[actions]]`,
     `[memory_layers]`, `[[eval_seeds]]`
2. **NEW** `crates/argo-genesis/src/templates.rs` (~250 LOC):
   - `struct VerticalTemplate { meta: TemplateMeta, prompt_skeleton: String, actions: Vec<String>, memory_layers: Vec<String>, eval_seeds: Vec<TemplateSeed> }`
   - `pub fn load_all() -> ArgoResult<Vec<VerticalTemplate>>` — walks `templates/` via `include_dir!`
   - `pub fn match_template(role_description: &str) -> Option<VerticalTemplate>`
     — keyword match against template `meta.match_keywords`
   - `pub fn merge_with_overrides(template: &VerticalTemplate, role_description: &str) -> ArgoResult<GeneratedVertical>`
3. **Extend** `lib.rs` to `pub mod templates;`
4. Wire `pipeline.rs` to consult `match_template` before falling
   through to free-form generation.

## Files you may touch

- `crates/argo-genesis/templates/sales_sdr.toml` (NEW, ≤ 100 LOC)
- `crates/argo-genesis/templates/customer_support.toml` (NEW, ≤ 100 LOC)
- `crates/argo-genesis/templates/marketing_intern.toml` (NEW, ≤ 100 LOC)
- `crates/argo-genesis/templates/finance_ar.toml` (NEW, ≤ 100 LOC)
- `crates/argo-genesis/templates/marketplace_seller.toml` (NEW, ≤ 100 LOC)
- `crates/argo-genesis/src/templates.rs` (NEW, ≤ 280 LOC)
- `crates/argo-genesis/src/templates_tests.rs` (NEW, ≤ 200 LOC) — 5 tests:
  - load_all() returns exactly 5 templates
  - "sales SDR for B2B SaaS" matches sales_sdr template
  - "AR clerk reconciling invoices" matches finance_ar template
  - unknown role returns None
  - merge_with_overrides preserves prompt_skeleton when no override hint
- `crates/argo-genesis/src/lib.rs` (narrowly: 1 pub mod line)
- `crates/argo-genesis/src/pipeline.rs` (narrowly: 1 call site)
- `crates/argo-genesis/Cargo.toml` (narrowly: add `include_dir` if not present)

## Files you must NOT touch

- `verticals/<demo>/` — demo verticals are reference data, not templates
- Existing `archetype.rs` — templates and archetypes can co-exist

## Acceptance criteria

- [ ] 5 template TOML files compile-time-included via `include_dir!`
- [ ] `load_all()` returns 5 templates, parsing each successfully
- [ ] `match_template("sales SDR for SaaS")` returns the sales_sdr template
- [ ] `merge_with_overrides()` produces a `GeneratedVertical` with all 4 sub-parts populated
- [ ] 5 unit tests pass
- [ ] `cargo build && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p argo-genesis` clean
- [ ] Every modified file < 500 LOC

## Branch + PR

- Branch: `claude-genesis-vertical-template-library`
- PR title: `genesis: vertical template library + match-by-keyword (claude-genesis-vertical-template-library)`

## Notes for the implementer

- Look at the 5 demo verticals (#937) to crib their tone/structure for
  the templates — this is the cheapest way to get realistic content.
- `include_dir` may or may not already be a workspace dep — check
  workspace `Cargo.toml` first; reuse if present, add narrowly if not.
- `match_keywords` is a simple `Vec<String>` per template — first
  template whose keywords overlap by ≥ 2 with the role description wins.
