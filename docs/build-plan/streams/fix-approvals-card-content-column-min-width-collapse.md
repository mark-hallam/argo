# fix-approvals-card-content-column-min-width-collapse

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-approvals-card-content-column-min-width-collapse`
**Source:** UX e2e audit 2026-05-28-0252 (scene-6 F4)

🟠 **P1 — On `/approvals` the first HIGH-tier approval card collapses its content column to roughly 4-character width: title "W…" then "Peta", then body text wrapping vertically letter-by-letter ("Thr / cust / repo / the / sam / API …"). The right rail (`$ at risk · $48k`) is intact but the card body is unreadable. Captured at 1600×900 in the audit screenshot.**

## Root cause

The card layout in `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` uses a flex row with the body content column on the left and a fixed-width right rail (`$ at risk` + confidence donut). The right rail likely carries `flex-shrink: 0` (correct), but the left content column is missing `min-width: 0`. Without `min-width: 0` on a flex child, the column refuses to shrink below its longest unbroken inline-content width, and the longest word in the body text dictates the column's effective minimum. When that word is long (e.g. "reproductivity" or a URL), the column squeezes down to roughly the width of one character per line at the affected viewport.

This is a well-known CSS flexbox trap — flex children default to `min-width: auto` (= content-based), not `0`, and overflow handling on the inner text node doesn't kick in until `min-width: 0` is set.

Sibling brief `fix-approvals-row-layout-collision-sla-bar-vs-inline-actions-narrow-viewport` (MERGED #2471) addressed a related row-shape issue at narrow viewports but did not fix the content-column min-width on the wider HIGH-tier cards.

## Findings this brief closes

- scene-6 F4 [P1] — Approval card content column collapses to a single-character column at 1600×900

## Fix recipe

1. **Identify the offending flex row** in `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` — likely the `<article>` or `<div className="approval-card-row">` that wraps the left content + right rail.
2. **Add `min-w-0` (Tailwind) or `min-width: 0` (CSS module / styled-jsx)** to the LEFT content column:
   ```tsx
   <div className="flex-1 min-w-0">
     <h3 className="truncate">{worker} · {action}</h3>
     <p className="line-clamp-2 break-words">{body}</p>
   </div>
   ```
3. **Also add `break-words` (CSS `overflow-wrap: anywhere`)** to the body `<p>` so long words break instead of forcing column expansion. The default `overflow-wrap: normal` would still respect long URLs.
4. **Verify the right rail stays `flex-shrink: 0`** (`shrink-0` in Tailwind) and has its own explicit width so the left column knows what it's competing against.
5. **Add a vitest snapshot or DOM-width assertion** to `ApprovalsOpsDashboard.test.tsx` that the content column's `clientWidth` is > 200px when the row body contains a long unbroken token (e.g. `"reproductivity-of-the-customer-segment"`). Use `jsdom` width approximation — exact pixel matching not required, just guard against the column collapsing to < 50px again.
6. **Visual regression**: extend the existing approvals storybook page to include a `HighTierLongWordCard` story so the operator can eyeball it.

## Acceptance criteria

- [ ] On `/approvals` at 1600×900 the first HIGH-tier card renders its title and body in a readable column (≥ 50% of the row width).
- [ ] Long unbroken words (URLs, identifiers) wrap at character boundaries rather than expanding the column.
- [ ] The `$ at risk` right rail keeps its fixed width and does not move.
- [ ] vitest assertion guards against future regression (clientWidth > 200px on long-word body).
- [ ] Storybook story added for the long-word case.
- [ ] PR body cites scene-6 F4 and includes a before/after screenshot.

## May-touch

- `web/components/skycot/approvals/ApprovalsOpsDashboard.tsx` (~10 LOC class change)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.test.tsx` (extend, ~30 LOC)
- `web/components/skycot/approvals/ApprovalsOpsDashboard.stories.tsx` (extend with a long-word story)

## Out of scope

- Severity-tier card layout (separate scope)
- The KPI strip layout above the cards (covered by `fix-approvals-kpi-strip-label-set-converge-root-vs-approvals`)
- Narrow viewport (< 1024px) — covered by `fix-approvals-row-layout-collision-sla-bar-vs-inline-actions-narrow-viewport` (MERGED #2471)
- Channel chip on the row (covered by `fix-approvals-row-channel-chip-claim-6-visual-proof`)

## Effort

XS — ~10 LOC fix + ~30 LOC test + storybook addition. ~1-2 hours.
