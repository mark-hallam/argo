# web-port-approvals-e-ops-dashboard

**Owner**: claude.
**Authored against**: origin/main `a2a8405f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-port-approvals-e-ops-dashboard`.

🟠 **P1 design-system parity** — port the operator-locked E ops-dashboard mockup to JSX. The standalone `/approvals` page currently renders an empty-state instead of the locked layout (5-KPI strip + 3 severity tiers + $-at-risk + confidence donuts + 7-day approval-rate ring).

## Why this brief exists

Operator memory `project_approvals_page_design_locked` (locked 2026-05-19) specifies the E ops-dashboard layout. The 25-audit synthesis on 2026-05-21 confirmed the locked design hasn't been ported. The 2026-05-24 demo SWOT verified the gap remains: `/approvals` shows a single "No approvals waiting" empty-state card with two CTAs — not the ops-dashboard.

This brief covers the **standalone `/approvals` page** (deep-link landing). The canonical inbox at `/` is in scope for the SAME treatment if scoping fits inside the file budget.

## Source of truth

- **HTML mockup**: the locked design file under `docs/skycot/design-explorations/2026-05-19/approvals/` (the option `-e-ops-dashboard.html`).
- **Memory**: `~/.claude/projects/-Users-operator-code-argo/memory/project_approvals_page_design_locked.md`
- **Brand pass note**: per `project_brand_consistency_pass`, the D3 brand-consistency pass SUPERSEDES the original "enterprise-restrained (Inter only)" register. Use Audiowide on the hero (already done in 2026-05-24 polish via `BrandLoudHero`) + purple→pink gradient + #1e1b4b ink for body.

## Scope

### 1. Implement the E ops-dashboard grid

Replace the empty-state body of `/approvals/page.tsx` with the locked layout:

- **5-KPI strip** at the top: Pending count · $ at risk · Median approval time · Approval rate (7-day) · Active reviewers
- **3 severity tiers** below (High/Medium/Low) with per-tier counts + colour-coded chips
- **$-at-risk donut** + **Confidence donut** (right rail or row 2)
- **7-day approval-rate ring** (small radial bar in the KPI strip OR larger ring as a hero metric)
- **Recent approvals list** (compact row layout — timestamp · vertical · action · severity · $ · time-to-decide)

### 2. Wire to live data

- `GET /api/v1/approvals/bundles?status=pending` returns the active inbox (already called per network trace).
- KPI strip needs aggregates — author the brief to extend the endpoint or compute client-side from the list.
- $-at-risk + confidence values come from each approval's `expected_value_micros` + `confidence_score` fields (extend the bundle if missing).

### 3. Preserve zero-state graceful path

When ZERO approvals exist, hide the KPI strip + donuts + recent list, and show only the existing "No approvals waiting" empty-state with the same two CTAs.

## Files you may touch

- `web/app/(app)/approvals/page.tsx`
- `web/components/skycot/approvals/*.tsx` (create the directory if it doesn't exist; per-component split since the full page will exceed 500 LOC)
- `web/lib/approvals/*.ts` for the data-shape hooks

## Files you must NOT touch

- `web/components/skycot/BrandLoudHero.tsx` (shared component — extending it belongs in a separate brief)
- The canonical inbox at `/` (out of scope; covered by a sibling brief if scoping demands)

## Acceptance criteria

- [ ] `/approvals` with ≥1 pending approval renders the 5-KPI strip + 3 severity tiers + 2 donuts + ring + recent list
- [ ] Zero-state path preserved (no KPIs/donuts/list — just the empty-state)
- [ ] Audiowide `BrandLoudHero` retained (already mounted as of 2026-05-24)
- [ ] Mobile: KPI strip + severity tiers stack to a single column; donuts shrink
- [ ] Recent list rows are keyboard-navigable + accessible
- [ ] No source file >500 LOC; tests live under `__tests__/`

## Verification

```bash
pnpm --dir web build
pnpm --dir web test approvals
bash scripts/check-brief-freshness.sh web-port-approvals-e-ops-dashboard
```

Manual:
```bash
open http://localhost:3000/approvals
# Seed an approval via gateway curl + reload
```

## Cross-references

- Locked design memory: `project_approvals_page_design_locked` (2026-05-19)
- Brand pass: `project_brand_consistency_pass` (2026-05-19)
- 2026-05-24 SWOT: this session's conversation log
- The 2026-05-24 polish pass already swapped the hero — see `web/app/(app)/approvals/page.tsx`
