# V2.4 — Skill Marketplace MVP

**Owner**: claude. **Depends on**:
claude-skill-share-cross-tenant-poc and
claude-web-dashboard-mvp merged. NOT a vertical brief —
platform feature.

## Scope

Web UI for browsing and installing skills across tenants.
Builds on the cross-tenant share PoC (which proved the data
model + signing works) and the dashboard MVP (which proved
the gateway can render HTML). The marketplace is the
public-facing entry point for the skill ecosystem:
- `/marketplace` lists all published skills with search +
  filter (vertical, executor, popularity)
- `/marketplace/<skill-id>` shows skill detail + install
  button
- Install action calls into the existing skill-share
  install path (no new wire protocol)
- Tenant isolation enforced — users see published skills
  but can only install into tenants they own

V3 candidates (NOT in MVP): skill ratings, paid skills,
verified-publisher badges, skill versioning UI, telemetry
opt-in flags.

## What lands

- `crates/argo-cli/src/handlers/gateway/marketplace.rs`
  (NEW file, ≤ 400 LOC) — three handlers:
  - `GET /marketplace` (list view)
  - `GET /marketplace/:skill_id` (detail view)
  - `POST /marketplace/:skill_id/install` (delegates to
    skill-share install path)
- `crates/argo-cli/src/handlers/gateway/templates/marketplace_list.html`
  (NEW) — list template with search bar
- `crates/argo-cli/src/handlers/gateway/templates/marketplace_detail.html`
  (NEW) — detail template with install button
- `crates/argo-cli/src/handlers/gateway/routes.rs` — add
  the 3 routes (DELTA only, respect 500-LOC budget)
- E2E smoke: `crates/argo-e2e/tests/marketplace_smoke.rs`
  (NEW) — list / detail / install round-trip
- Help text update: `argo gateway --help` notes the
  marketplace path

## Acceptance criteria
- [ ] `GET /marketplace` returns 200 with at least one
      skill listed when seed skills published
- [ ] Search by free-text against skill name + description
- [ ] Filter by vertical (drop-down sourced from
      vertical.toml manifests)
- [ ] Filter by required executor (e.g. "needs gmail")
- [ ] `GET /marketplace/:skill_id` renders detail page with
      install button (disabled if already installed)
- [ ] `POST /install` returns 200 + redirects to detail
      page with "installed" state on success
- [ ] Tenant isolation: install posts authenticated as
      tenant A cannot install into tenant B (403 test in
      marketplace_smoke.rs)
- [ ] Install path delegates to existing skill-share code —
      NO duplicate logic
- [ ] Help-smoke pins the new commands
- [ ] E2E smoke passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
      clean
- [ ] No new crates; routes.rs stays ≤ 500 LOC

## Branch + PR
- Branch: `claude-skill-marketplace-mvp`
- PR title: `gateway: Skill marketplace MVP (claude-skill-marketplace-mvp)`

## Notes
- This is NOT a payments / billing brief. Install is free
  and gated only on tenant ownership.
- Search uses simple `contains()` on the in-memory skill
  index — not a real search engine. V3 candidate: meilisearch.
- Templates are server-side rendered (no client framework);
  matches the dashboard MVP pattern.
- The "installed" state lives in tenant skill registry
  already — no schema change.
- No skill-author-side workflow in this brief; publishing
  is assumed to happen via existing CLI from the share
  PoC. V3 candidate: marketplace publish UI.
