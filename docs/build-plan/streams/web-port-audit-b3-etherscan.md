# web-port-audit-b3-etherscan

**Owner**: claude.
**Authored against**: origin/main `a2a8405f4` on `2026-05-24`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-port-audit-b3-etherscan`.

🟠 **P1 design-system parity** — port the operator-locked B3-enterprise audit mockup to JSX. Currently `/audit` returns 500 under demo-mode bypass (no audit rows yet), so the locked Etherscan-style layout (compact table + filter chips + pop-out modal per row) isn't visible at all.

## Why this brief exists

Operator memory `project_audit_page_design_locked` (locked 2026-05-18) specifies the B3-enterprise layout. The 2026-05-24 demo SWOT walked the surface and observed:
1. The page 500s on first load (graceful error page renders — that part is good).
2. Even with audit rows present, the locked B3 design hasn't been ported.

The 25-audit synthesis on 2026-05-21 flagged this gap; the 2026-05-24 SWOT confirms it's still open.

## Source of truth

- **HTML mockup**: the locked design file under `docs/skycot/design-explorations/2026-05-18/audit/` (the option ending in `-b3-enterprise.html`).
- **Memory**: `~/.claude/projects/-Users-operator-code-argo/memory/project_audit_page_design_locked.md`
- **Brand pass note**: per `project_brand_consistency_pass`, the D3 brand-consistency pass SUPERSEDES the original "Inter only / no Audiowide" rule on this page. Use Audiowide on the hero + purple→pink gradient.
- **Backend**: `GET /api/v1/audit?range=today` returns the audit chain (used today per network trace).

## Scope

### 1. Implement the B3 Etherscan-style table

Replace the current `/audit/page.tsx` body with the locked layout:

- **Audiowide hero** "Audit chain." with brand-loud register (via `BrandLoudHero`)
- **Filter chips strip** below the hero: Range (1h / 24h / 7d / 30d / custom) · Kind (all / action / approval / login / skill / cu) · Principal (all / human / worker / system) · Outcome (success / failure)
- **Compact table** with columns: timestamp (relative + absolute on hover) · row hash (first 8 chars + copy button) · principal · kind · summary (one-line excerpt) · outcome chip
- **Click a row → pop-out modal** with: full hash, prev-hash + chain link, full payload JSON (syntax-highlighted), audit trail location (the file/line that wrote the row), "verify integrity" button that calls `GET /api/v1/audit/sessions/<uuid>/verify`

### 2. Fix the 500 for empty audit chains

The current error page is friendly but masks the issue. When `GET /api/v1/audit?range=today` returns an empty array, render a "No audit rows in this range yet" empty-state with a link to widen the range. Only render the 500 page on actual server errors.

### 3. Wire the "verify integrity" CTA

The pop-out modal needs `GET /api/v1/audit/sessions/<uuid>/verify` integration. Show the verification result (✅ chain intact / ❌ tampering detected at row N) in a banner.

## Files you may touch

- `web/app/(app)/audit/page.tsx`
- `web/app/(app)/audit/error.tsx` (only to differentiate empty vs server-error paths)
- `web/components/skycot/audit/*.tsx` (create the directory; full page will exceed 500 LOC across files)
- `web/lib/audit/*.ts` for data hooks

## Files you must NOT touch

- `crates/argo-cli/src/handlers/gateway/routes_audit.rs` (backend is correct — empty list is a 200, not a 500; the 500 means a real server error)
- The audit-write side (this is read-only UI)

## Acceptance criteria

- [ ] `/audit` with ≥1 audit row renders the Etherscan-style table + filter chips strip + Audiowide hero
- [ ] Filter chips work client-side + server-side (range hits the backend; others can be client-only)
- [ ] Clicking a row opens the pop-out modal with full hash + payload + verify button
- [ ] Verify button calls the verify endpoint and shows the result
- [ ] Empty-range case shows a "No rows in this range" empty-state, not a 500
- [ ] Server-error case still renders the friendly 500 page
- [ ] Mobile: table reflows to a card list; filter chips become a single dropdown
- [ ] No source file >500 LOC

## Verification

```bash
pnpm --dir web build
pnpm --dir web test audit
bash scripts/check-brief-freshness.sh web-port-audit-b3-etherscan
```

Manual:
```bash
open http://localhost:3000/audit
# Seed audit rows by triggering signup/approval/skill-install via the SPA, then reload
```

## Cross-references

- Locked design memory: `project_audit_page_design_locked` (2026-05-18)
- Brand pass: `project_brand_consistency_pass` (2026-05-19)
- 2026-05-24 SWOT: this session's conversation log
