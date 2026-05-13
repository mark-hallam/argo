# Architecture diagrams

Visual reference for the five subsystems reviewers, operators, and
new contributors ask about most. Each page pairs a Mermaid diagram
with a short prose intro and a "see also" block that names the
canonical source files. Diagrams are accurate as of merge — line
numbers are deliberately omitted because they rot.

If a diagram contradicts the code, the code wins; please open a PR
to fix the diagram.

## Pages

- [Gateway routes](./gateway-routes.md) — HTTP dispatch from the
  gateway listener through the per-method routers down to per-resource
  handlers.
- [Executor dispatch](./executor-dispatch.md) — how
  `ComputerUseExecutor::run` drives an action through the
  Anthropic ↔ E2B step loop, with the budget, audit, and screenshot
  persistence side-channels.
- [Memory layers](./memory-layers.md) — `MemoryStore` over three
  layers (Recent, Indexed, Summary) plus locks, supersession, hybrid
  retrieval, and the Dreams summary hook.
- [Judge integration](./judge-integration.md) — where `Judge::adjudicate`
  fits in the skill creation / refinement path, what evidence flows
  in, and how the verdict is persisted.
- [Dreams pipeline](./dreams-pipeline.md) — the three Dreams
  call sites (memory summary, skill distillation, plan templates),
  their cohort gates, and the daily-cap accounting.

## Format notes

Mermaid is rendered server-side by
[`mdbook-mermaid`](https://github.com/badboy/mdbook-mermaid). Each
diagram is its own ```` ```mermaid ```` fence so it can be edited or
copied independently. Diagrams use `graph LR` / `graph TD` flow
shapes rather than sequence diagrams unless the interaction is
genuinely temporal — flowcharts diff more cleanly under code review
and render reliably on narrow viewports.
