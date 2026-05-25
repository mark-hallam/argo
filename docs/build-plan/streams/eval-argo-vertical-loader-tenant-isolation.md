# eval-argo-vertical-loader-tenant-isolation — audit argo-vertical loader surface + tenant isolation

**Owner**: claude or codex. **Depends on**: nothing.
**Authored against**: origin/main `289d3077f` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-vertical-loader-tenant-isolation`.

## Why this is P2

Mission gap manifest row `argo-vertical` is 🔴 NOT AUDITED. `argo-vertical` loads `.workflow.toml` and `vertical.toml` files into runtime — the authoritative source for v1 vertical loading per `crates/argo-vertical/src/workflow_definition.rs` docstring. Mission claim 3 (Genesis-builds-workers) requires Genesis output to round-trip through this loader; mission claim 6 (multi-tenant) requires the loader to scope discovered verticals by tenant. Neither is currently audit-pinned.

## Scope (read-only audit)

1. **Surface inventory** of `crates/argo-vertical/src/lib.rs` — pub fns, structs, enums; produce a table.
2. **Loader entry points** — find every call site of `parse_workflow` / `load_dir` / `parse_vertical` etc. in the workspace; document who reads from where.
3. **Tenant isolation check** — does any loader path bind `tenant_id`? If a vertical is loaded from disk, can two tenants see overlapping vertical IDs?
4. **Genesis round-trip** — drive Genesis on 3 distinct role descriptions (sales-rep, support-tier-1, project-coordinator), write trees to tempdirs, call `argo_vertical::load_dir` on each, assert no parse errors.
5. **Author pin tests** at `crates/argo-vertical/tests/loader_surface_pin_test.rs` (~150-250 LOC) — surface re-export pins + tenant-isolation contract pins (FINDINGS-ONLY `#[ignore]` is acceptable if a wire is broken).
6. **Write an evals report** at `docs/skycot/evals/2026-05-25-argo-vertical-loader-tenant-isolation.md` summarising findings + ≤5 follow-up brief candidates.

## Files you may touch

- `crates/argo-vertical/tests/loader_surface_pin_test.rs` — new file.
- `docs/skycot/evals/2026-05-25-argo-vertical-loader-tenant-isolation.md` — new file.

## Files you must NOT touch

- Anything under `crates/argo-vertical/src/` — this is a read-only audit.
- Production callers of the loader.

## Acceptance criteria

- [ ] Surface table in evals report covers all `pub fn` / `pub struct` / `pub enum` in `argo-vertical::lib`.
- [ ] At least 5 pin tests in the new test file; all PASS on a clean `cargo test -p argo-vertical`.
- [ ] Tenant-isolation question answered Yes/No with a code citation either way.
- [ ] Report ends with a numbered "follow-up briefs" list (≤5 items).

## Branch + PR

- Branch: `claude-eval-argo-vertical-loader-tenant-isolation` or `codex-…`.
- PR title: `eval(vertical): audit argo-vertical loader surface + tenant isolation (eval-argo-vertical-loader-tenant-isolation)`.

## Notes

- ~150-250 LOC test code + ~600-1000 word report.
- Read the brief-authoring pre-flight rules in [`feedback_brief_authoring_preflight_rules.md`](memory) before grepping for new types.
- This is a READ audit — do not introduce new production surface.
