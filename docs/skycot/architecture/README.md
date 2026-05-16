# SKYCOT architecture diagrams — snapshot index

**Snapshot SHA:** `ed1ac928` (origin/main 2026-05-16)
**Workspace state:** 91 crates · 73 monotonic migrations · 37 MCP connectors · 5 active GitHub workflows · 9 vertical packs · 502 PRs merged.

These diagrams describe what's actually shipped on `origin/main` at the snapshot SHA. They are read-only snapshots; when the architecture evolves, the diagrams are updated — each file's snapshot stamp gets bumped to the new SHA.

## Diagram catalogue

### A — Bird's-eye orientation
- [`00-system-overview.md`](00-system-overview.md) — single-page system map
- [`01-mission-claim-map.md`](01-mission-claim-map.md) — 6 mission claims → owning crates
- [`02-deployment-topology.md`](02-deployment-topology.md) — `skycot.com` / `app.` / `api.` / `try.` / `status.`

### B — Runtime flows
- [`10-worker-turn-loop.md`](10-worker-turn-loop.md) — agent runner inner loop
- [`11-action-execution-flow.md`](11-action-execution-flow.md) — manifest action → executor → outcome → audit
- [`12-approval-flow.md`](12-approval-flow.md) — action → policy gate → channel → reply → resume
- [`13-skill-loop-claim4.md`](13-skill-loop-claim4.md) — teach → propose → eval → review → publish → reload
- [`14-onboarding-funnel.md`](14-onboarding-funnel.md) — signup → verify → provision → wizard → first hire

### C — Cross-cutting
- [`20-multi-tenant-data-flow.md`](20-multi-tenant-data-flow.md) — `tenant_id` plumbing through every primitive
- [`21-audit-chain.md`](21-audit-chain.md) — hash-chained audit trail per session
- [`22-capability-rbac.md`](22-capability-rbac.md) — capability gate enforcement
- [`23-policy-as-code-resolution.md`](23-policy-as-code-resolution.md) — policy resolution order

### D — Surface inventories
- [`30-api-routes.md`](30-api-routes.md) — gateway HTTP endpoints
- [`31-spa-routes.md`](31-spa-routes.md) — Next.js app + marketing
- [`32-in-core-tool-surface.md`](32-in-core-tool-surface.md) — 15 in-core tools + MCP-first connectors

### E — Ops/dev
- [`40-ci-pipeline.md`](40-ci-pipeline.md) — 5 active GitHub workflows + gates
- [`41-swarm-architecture.md`](41-swarm-architecture.md) — operator / conductor / workers

## How to update

When the architecture evolves and a diagram becomes stale:

1. Identify the affected diagram(s).
2. Update its Mermaid source.
3. Bump the file's snapshot stamp: `**Snapshot SHA:** `<new-short-sha>` (origin/main YYYY-MM-DD)`.
4. Note the `**Changed in this update:**` block at the top so readers see the delta.
5. Update `README.md` workspace-state line if any major counts changed.

## Conventions

- **Mermaid** for all diagrams — renders inline on GitHub, IDEs, most markdown viewers.
- **Crate names** in `argo-*` form (e.g. `argo-cli`, `argo-store`).
- **File:line citations** where the diagram references concrete code.
- **Mission claims** numbered 1–6 per `docs/skycot/MISSION.md`.
- **Severity tags** P0/P1/P2/P3 match `docs/skycot/audit/2026-05-16-one-shot-architecture-audit.md`.
