# fix-audit-page-design-memo-vs-implementation-modal-route-decision

**Owner**: claude or codex.
**Authored against**: origin/main `2cc2ec42b` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-page-design-memo-vs-implementation-modal-route-decision`.
**Source**: ux-e2e-audit-T2309MEL `scene-6-claim-6-trust-and-approvals.md` F9 [P1].

🟢 **P2 — `/audit` page header reads "Audit / Forensic + compliance view". The locked-design memo `project_audit_page_design_locked` says "Etherscan-style + Inter + pop-out modal." The page IS Etherscan-shaped (sticky `TODAY` day header, monospace timestamps, single-column list). But the drill-in is a full route push (`/audit/action/<id>`), not the "pop-out modal" the memo describes. Memo-vs-implementation drift — operator decision needed: keep the route drill-in (current) or implement the modal (locked memo).**

## Root cause

Two simultaneously-true things create the question:

1. **The locked memo says "pop-out modal."** Per `project_audit_page_design_locked` (locked 2026-05-18, B3-enterprise design): "Etherscan-style + Inter + pop-out modal."

2. **The implementation routes.** `web/app/(app)/audit/page.tsx` rows are wired with `href={`/audit/action/${row.id}`}` — clicking pushes to a new route, not opening an overlay. The detail page at `web/app/(app)/audit/action/[actionId]/page.tsx` is a full-page surface.

The route-based drill-in has real advantages over a modal:
- Deep-linkable (operator can share `/audit/action/<id>` URLs)
- Back-button works naturally
- Replay sub-page (`/audit/action/<id>/replay`) chains cleanly
- Sub-tabs / nested content easier to author

But the locked memo specifies a modal. Either the memo needs updating OR the implementation needs to revert to a modal. This is an operator decision, not an engineering one.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F9 [P1 → likely P2] — Audit page header reads "Forensic + compliance view"; modal vs route drill-in drift

## Fix recipe

This is a TWO-OUTCOME brief — either path is fine; operator picks one.

### Option A (preferred — update the memo, keep the route)

1. **Update `project_audit_page_design_locked` memory** (operator action):
   - Replace "pop-out modal" with "full-route drill-in at `/audit/action/<id>` (deep-linkable, back-button native)"
   - Keep "Etherscan-style + Inter" — that part is implemented correctly

2. **Update the page header copy.** "Forensic + compliance view" is fine but feels infra-y. Suggested: `"Audit"` (h1) + subtitle `"Every action, hash-chained and replayable."` to match the locked Etherscan + chain-anchor framing.

3. **Document the deep-link contract** in `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 6:
   - URL: `/audit/action/<id>`
   - Routes preserved across reloads
   - Replay sub-page: `/audit/action/<id>/replay`

### Option B (implement the locked modal)

1. **Author `<AuditDetailModal />`** at `web/components/skycot/audit/AuditDetailModal.tsx`:
   - Renders the same content currently on `/audit/action/<id>`
   - Closes on Escape, backdrop click, X button
   - Updates URL fragment (`/audit#action-<id>`) for back-button without route push
   - Still allows deep-linking via `/audit#action-<id>` parsing on mount

2. **Migrate `/audit/action/<id>/page.tsx` to a fallback** — either delete (rely on modal entirely) or keep as a printable / deep-link landing surface that opens the modal on mount.

3. **Verify Replay still works.** The Replay sub-page is the one Scene-6 surface that genuinely works end-to-end (per audit F11). Don't break it.

## Acceptance criteria

- [ ] Operator decision documented in PR description (A or B)
- [ ] If A: memory `project_audit_page_design_locked` updated to reflect route-based drill-in; page subtitle replaced; runbook updated
- [ ] If B: modal component exists, opens on row click, closes correctly, URL fragment + deep-link works, Replay sub-flow unbroken
- [ ] No regression on `/audit` list rendering (per locked Etherscan-shape design)
- [ ] `pnpm --dir web exec vitest run app/(app)/audit` green
- [ ] `pnpm --dir web exec tsc --noEmit` green

## May-touch

- `docs/skycot/design-locks/audit-page.md` (or wherever the locked memo is canonicalized — verify; may only live in user memory)
- `web/app/(app)/audit/page.tsx` (~5 LOC subtitle edit if A; ~30 LOC modal mount if B)
- `web/components/skycot/audit/AuditDetailModal.tsx` (~150 LOC new component — only if B)
- `web/app/(app)/audit/action/[actionId]/page.tsx` (delete or keep as fallback — only if B)
- `docs/skycot/demos/v1-mission-mvp-runbook.md` (~10 LOC update Scene 6)

## Out of scope

- Audit row drill-in content (covered by `fix-placeholder-audit-detail-fabricated-decision-chain-and-provenance`, `fix-audit-action-detail-demo-mode-fallback`)
- Hash anchor render (`fix-audit-provenance-block-render-hash-chain-anchor`)
- Audit feed action variety (`fix-audit-feed-action-kind-variety`)
- Replay endpoint (`fix-replay-endpoint-contract-spa-vs-gateway-mismatch`)

## Effort

S (Option A) — ~15 LOC + memo edit + runbook edit. ~30 min.
M (Option B) — ~150 LOC component + ~50 LOC refactor + ~80 LOC test. ~3-4 hours.

P2 because the v1.0 demo records Scene 6 as a route-based drill-in already — the locked memo is descriptive of intent, not load-bearing for the take. Should be resolved before merging the memo into authoritative status though.
