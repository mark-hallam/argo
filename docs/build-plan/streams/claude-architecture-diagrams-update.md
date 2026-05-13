# V1.4 — Architecture diagrams update

**Owner**: claude. **Depends on**: claude-docs-site-build (#934).

**Drift-fix amendment 2026-05-13** (vs HEAD 6159b789): brief-cleanup sweep updated may-touch + source-check paths. (1) `book/src/architecture/` → `book/architecture/` (current `book.toml` sets `src = "book"`); existing pages: `protocol.md`, `worker-v3.md`, `memory-and-skills.md`. SUMMARY.md is at `book/SUMMARY.md`. (2) Source-check paths updated to real `argo-*` crate layout: gateway routes live in `crates/argo-cli/src/handlers/gateway/routes_*.rs` (not `crates/gateway/`); executor is `crates/argo-executor-computer-use/`; memory is `crates/argo-memory/`; judge is `crates/argo-judge/`; "dreams" is not a standalone crate — pipeline is split across `crates/argo-skill/src/dreams_distillation.rs`, `crates/argo-runtime/src/dreams_plan_templates.rs`, `crates/argo-memory/src/dreams_backend.rs` (find with `rg dreams crates/`). (3) `.github/workflows/docs.yml` is currently `.github/workflows/docs.yml.disabled` — if Mermaid pre-processor needs CI install, either re-enable that workflow as a side-effect or document the manual `cargo install mdbook-mermaid` requirement in the PR description.

## Why

`book/architecture/` currently links the protocol doc and
a couple of investigations but lacks visual reference for how
the pieces fit. Reviewers (new contributors, ops, evaluators)
ask the same five questions every time:

1. What HTTP routes does the gateway expose, and how do they
   map to handlers?
2. How does the executor dispatch a task to a runner?
3. What are the memory layers and how do they interact?
4. Where does the judge plug in?
5. How does the dreams pipeline flow?

A diagram per area cuts a 20-minute conversation to a
30-second glance.

## Scope

Add Mermaid diagrams (mdBook renders Mermaid via
mdbook-mermaid; if not installed, install in this PR) covering
the five areas above. Each diagram lives in its own page with
a short prose intro and links to the relevant source files
(by path, not line number — line numbers rot).

Diagrams must be:
- **Mermaid source**, not images. Mermaid renders
  responsively, diffs cleanly, and stays editable.
- **Accurate as of merge.** Cross-check against current
  `crates/argo-cli/src/handlers/gateway/routes_*.rs`,
  `crates/argo-executor-computer-use/src/`, `crates/argo-memory/`,
  `crates/argo-judge/`, and dreams sources scattered across
  `argo-skill::dreams_distillation`, `argo-runtime::dreams_plan_templates`,
  `argo-memory::dreams_backend` before pushing. If a diagram
  contradicts code, code wins.

Each page ends with a "see also" block pointing to the
canonical source files.

## Files you may touch
- `book/architecture/diagrams/gateway-routes.md` (NEW)
- `book/architecture/diagrams/executor-dispatch.md` (NEW)
- `book/architecture/diagrams/memory-layers.md` (NEW)
- `book/architecture/diagrams/judge-integration.md` (NEW)
- `book/architecture/diagrams/dreams-pipeline.md` (NEW)
- `book/architecture/diagrams/index.md` (NEW — landing)
- `book/SUMMARY.md` (add Architecture › Diagrams entries)
- `book.toml` (add `[preprocessor.mermaid]` if missing)
- `.github/workflows/docs.yml.disabled` (add mdbook-mermaid install
  step if needed for the build; note the workflow is currently
  disabled — adding this here is forward-looking, or re-enable
  the workflow if appropriate)

## Files you must NOT touch
- Any `crates/**` Rust source
- `docs/skycot/MISSION.md`
- `book/architecture/protocol.md`, `book/architecture/worker-v3.md`,
  `book/architecture/memory-and-skills.md` (existing pages, owned
  elsewhere)
- Other doc sections

## Acceptance criteria
- [ ] `mdbook build` succeeds, Mermaid renders in output
      HTML (verify locally)
- [ ] `mdbook test` passes
- [ ] All five diagram pages exist and render
- [ ] Each diagram cross-checked against current source
      (cite the inspected files in the PR description)
- [ ] No diagram references a function/route/layer that no
      longer exists at merge time
- [ ] Mobile-responsive (Mermaid SVGs scale; verify by
      narrowing viewport)
- [ ] CI docs build still succeeds end-to-end

## Branch + PR
- Branch: `claude-architecture-diagrams-update`
- PR title: `docs: architecture diagrams (gateway, executor, memory, judge, dreams) (claude-architecture-diagrams-update)`

## Notes

Mermaid syntax is forgiving but has gotchas — long node
labels break rendering, and some shapes only work in newer
versions. Test rendering in the actual `mdbook serve` output,
not just the Mermaid live editor.

Prefer simple flowcharts (`graph LR`) over sequence diagrams
unless the interaction is genuinely temporal. For routes, a
table-of-routes Markdown block alongside the diagram is more
useful than cramming 88 endpoints into one graph.

If a subsystem is mid-refactor and the "correct" diagram is
about to change, note that in the page (a `> Status:` block
at the top) rather than guessing.
