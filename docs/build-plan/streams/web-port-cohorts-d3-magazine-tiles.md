# web-port-cohorts-d3-magazine-tiles

**Owner**: claude.
**Authored against**: origin/main `a2a8405f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-port-cohorts-d3-magazine-tiles`.

🟠 **P1 design-system parity** — port the operator-locked D3 cohorts mockup to JSX. Currently `/cohorts` renders a generic empty state instead of the locked D3 magazine-tiles design (1 lead tile + 2 stacked + 4 compact, asymmetric Bloomberg-feature aesthetic, 6 department tabs).

## Why this brief exists

Operator memory `project_cohorts_page_design_locked` (locked 2026-05-19) specifies the D3 layout. The 25-audit synthesis on 2026-05-21 flagged that the locked design hasn't been ported to JSX. The 2026-05-24 demo SWOT confirmed the gap is still open: visiting `/cohorts` lands the operator on a single empty-state card with a "Create cohort" button — none of the magazine tiles, department tabs, or lead/stacked/compact structure render.

## Source of truth

- **HTML mockup**: the locked design file under `docs/skycot/design-explorations/2026-05-19/cohorts/` (the option ending in `-d3-magazine-tiles.html`). The brief implementor MUST open and screenshot this mockup before authoring any JSX.
- **Memory**: `~/.claude/projects/-Users-operator-code-argo/memory/project_cohorts_page_design_locked.md`
- **Existing infra**: `web/components/skycot/BrandLoudHero.tsx` (gradient banner — already in use on `/cohorts`), `web/components/skycot/cohorts/CohortMagazineStyles.tsx` (some magazine styles already exist).

## Scope

### 1. Implement the D3 magazine grid

Replace the current `/cohorts/page.tsx` empty-state with the locked D3 layout:

- **6 department tabs** across the top: Engineering, Sales, Marketing, Support, Operations, Project & Ops (per memory `project_pm_cohorts_peta_patrick`).
- **Asymmetric grid** below the tabs:
  - **1 lead tile** (left, large, lead cohort with headline metric + 2-line description + 3 KPI chips + featured worker chips)
  - **2 stacked tiles** (right column, medium, supporting cohorts)
  - **4 compact tiles** (bottom row, small, secondary cohorts)
- Bloomberg-feature editorial aesthetic — generous whitespace, asymmetric column widths, large display numbers.
- Brand-loud-editorial register.

### 2. Wire the tabs to real data

Currently the cohorts endpoint at `GET /api/v1/cohorts` returns the list. Department filtering is a client-side filter on the response shape. When zero cohorts match a department, show a per-tab empty-state.

### 3. Preserve the empty-state for zero-cohort workspaces

When the workspace has zero cohorts across ALL departments, keep the existing "No cohorts" empty state with the "Create cohort" CTA — operator can build out the surface from there.

## Files you may touch

- `web/app/(app)/cohorts/page.tsx`
- `web/components/skycot/cohorts/*.tsx`
- `web/components/skycot/cohorts/*.css`

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/routes_cohorts.rs` (the backend is already correct)
- `web/components/skycot/BrandLoudHero.tsx` (shared component — extending it belongs in a separate brief)

## Acceptance criteria

- [ ] Visiting `/cohorts` with ≥1 cohort renders the D3 magazine grid (1 lead + 2 stacked + 4 compact)
- [ ] 6 department tabs render and filter client-side
- [ ] Per-tab empty state appears when a department has zero matching cohorts
- [ ] Zero-cohort workspace still shows the simple "No cohorts" empty state with "Create cohort"
- [ ] Audiowide hero retained (the existing `BrandLoudHero` mount stays)
- [ ] Mobile: lead tile + stacked + compact reflow to a single column
- [ ] No new `__tests__` file >500 LOC; no source file >500 LOC

## Verification

```bash
pnpm --dir web build              # type-check + bundle
pnpm --dir web test cohorts       # if tests exist
bash scripts/check-brief-freshness.sh web-port-cohorts-d3-magazine-tiles
```

Then manually:
```bash
ARGO_DEMO_MODE=1 ARGO_GATEWAY_AUTH_BYPASS=1 SKYCOT_DB_KEY_BASE64="$(openssl rand -base64 32)" \
  argo gateway start --bind 127.0.0.1 --port 7400 &
NEXT_PUBLIC_ARGO_DEMO_MODE=1 NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1 NEXT_PUBLIC_GATEWAY_URL=http://127.0.0.1:7400 \
  pnpm --dir web dev
open http://localhost:3000/cohorts
```

## Cross-references

- Locked design memory: `project_cohorts_page_design_locked` (2026-05-19)
- 25-audit synthesis: `docs/skycot/audit/2026-05-21-25-audit-synthesis.md` (cohorts gap row)
- 2026-05-24 SWOT: this session's conversation log
