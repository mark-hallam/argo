# multi-tenant-theming-override-persist (FINAL SPRINT) — persist + apply tenant white-label theme

**Owner**: claude.
**Authored against**: origin/main `f491ca8da` on `2026-06-08`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh multi-tenant-theming-override-persist`.

## ⚠️ Brand guardrail (LOCKED)
The SKYCOT brand is LOCKED (`docs/skycot/brand/DESIGN_SYSTEM.md`, DEC-027): gradient
purple→PINK `#8B5CF6 → #EC4899`, Audiowide/Inter/JetBrains Mono. **This brief does
NOT touch the default brand tokens.** It adds an *operator white-label override* for a
tenant who wants their own accent — the SKYCOT default stays exactly as shipped when
no override is set. This is the per-tenant theming the operator already requested
(memory `project_multi_tenant_theming`).

## Why (evidence)
The per-tenant CSS-synthesis token infra **exists** but the override is **inert** — no
persist route, so a tenant's chosen accent never survives:
- `web/lib/theme/synthesize.ts:76` `export function synthesizeTenantCss(input:
  TenantThemeInput)` — synthesizes a full tenant theme from a single brand accent,
  emitting `data-theme="<id>-light"/"<id>-dark"` (line 16); designed for a
  tenant-scoped layout `<head>` (line 73-76).
- `web/lib/theme/cookie.ts` only persists the *color mode* (light/dark,
  `THEME_COOKIE_NAME = "skycot-color-mode"`, lines 15/29/45) — NOT a tenant accent
  override.
- There is no `tenant/theme` gateway route (grep returns nothing under
  `crates/argo-cli/src/handlers/gateway/`).

So: add `PATCH /api/v1/tenant/theme` (+ `GET` read) persisting a per-tenant theme
override, and wire the SPA to apply it via the existing `synthesizeTenantCss`.

## Grep-before-write (REQUIRED per DRIFT_PREVENTION)
- `rg -n 'synthesizeTenantCss|TenantThemeInput' web/lib/theme/synthesize.ts` — REUSE this synthesizer to render the override; do NOT write a second CSS generator.
- `rg -n 'tenant/theme|tenant_theme|theme.*override|TenantTheme' crates/` — confirm no existing route/type; if a tenant-settings store already exists, persist the override THERE rather than a new table.
- `rg -n 'workspace_settings|tenant.*settings|fn .*settings' crates/argo-cli/src/handlers/gateway/routes_workspace_settings*.rs crates/argo-store/src` — prefer extending the existing tenant/workspace-settings persistence (additive column/key) over a parallel `tenant_theme` table.
- `rg -n 'resolve.*tenant|tenant_principal|require_identity' crates/argo-cli/src/handlers/gateway/` — the route MUST resolve the caller's own tenant (no cross-tenant theme writes).

## Files you may touch
- new `crates/argo-cli/src/handlers/gateway/routes_tenant_theme.rs`
  (`GET /api/v1/tenant/theme` → the caller-tenant's override or empty;
  `PATCH /api/v1/tenant/theme` → persist `{accent, …}` for the caller's tenant,
  audited). Tenant-scoped — a caller can only read/write their OWN tenant's theme.
  Keep < 500 LOC.
- the GET + PATCH dispatch tables (register the routes).
- the tenant/workspace-settings persistence (additive key/column) OR an additive
  migration `crates/argo-store/src/migrations/vNN.rs` for a `tenant_theme` row —
  prefer extending existing settings; next free slot; registered in the registry.
- `web/lib/theme/` (a small `tenant-theme.ts`: fetch the override, feed
  `synthesizeTenantCss`, inject into the tenant layout `<head>`) + the layout that
  mounts it. Default brand tokens unchanged when override is absent.
- a tests sibling + a vitest.

## Acceptance (truth-bounded + testable)
- An integration test: `PATCH /api/v1/tenant/theme` with an accent, then `GET` it back
  → the override round-trips for that tenant; a write audit row is appended.
- A **tenant-isolation test**: tenant A's PATCH never changes tenant B's theme; a GET
  for tenant B returns B's own (or default), never A's accent.
- A test asserting that with NO override set, the response yields the locked SKYCOT
  default (the brand is unchanged for non-white-label tenants).
- SPA vitest: `synthesizeTenantCss` is fed the fetched override and produces the
  `data-theme` scoped CSS (the existing synthesizer is reused, not reimplemented).
- `cargo test -p argo-cli` green; `tsc` + the touched-file vitest green.

## On ship
Update the FEATURE_MATRIX multi-tenant-theming row (synth infra only → persisted
white-label override) with the commit SHA + route in the SAME commit, then
`python3 scripts/render-feature-matrix.py`.

## Effort
M (Rust + web). ~250–400 LOC incl. tests. Additive (prefer extending tenant settings;
one additive migration only if a dedicated row is chosen).
