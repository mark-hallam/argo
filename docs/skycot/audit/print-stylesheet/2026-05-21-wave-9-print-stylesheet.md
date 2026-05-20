# Wave-9 Audit: Print Stylesheet — `/audit` + `/audit/action/[id]` + workflows

- **Date**: 2026-05-21
- **Auditor**: Wave-9 agent #2 (read-only)
- **Scope**: print-readiness of compliance-relevant surfaces (audit feed,
  audit action detail, workflow definition viewer)
- **Method**: ripgrep across `web/` for `@media print`, `@page`,
  `print-color-adjust`, Tailwind `print:` variants, `window.print()`,
  and any `Print*` triggers; full read of the audit page + audit
  action detail page + their dependency components (`AppShell`,
  `AuditPageHeader`, `ActionSummaryHero`, `DecisionChainCard`,
  `ProvenanceBlock`, `AuditDayGroup`).

---

## Verdict

**FAIL — zero print support.** The web app has no print stylesheet
of any kind. Compliance auditors who print `/audit` or
`/audit/action/<id>` will receive a paper artifact that is
**unverifiable, incomplete, and badly clipped**. This is a P1 finding
for enterprise audit-log consumers: provenance hashes truncate to
8 chars on screen (full hash only in a tooltip + clipboard), and on
paper the truncated `…` is what prints. The compliance binder ends
up with hashes that cannot be checked against a chain-of-custody.

---

## Evidence

### 1. `@media print` rules — NONE

```
$ rg "@media\s+print" web/  -> 0 hits
$ rg "@page"          web/  -> 0 hits
$ rg "print:"         web/  -> 0 hits   (Tailwind print variant)
$ rg "print-color-adjust|color-adjust" web/ -> 0 hits
```

Files searched: `app/globals.css` (the only stylesheet in the project — no module CSS, no `*.scss`, no Tailwind print: utilities anywhere in any TSX).

`app/globals.css` defines theme tokens, base reset, body defaults, and
two tenant theme examples. There is **no `@media print` block, no `@page`
margin rule, no `print-color-adjust: exact`**. Tailwind v4 is present
(`@import "tailwindcss"`) but no `print:` variant utilities are used in
any `.tsx` file across `app/` or `components/`.

### 2. Background-color preservation — UNSAFE

Browsers strip background colors and gradients from printouts by default
unless `print-color-adjust: exact` (or `-webkit-print-color-adjust: exact`)
is declared. None of these declarations exist anywhere in the codebase.

Practical impact on `/audit/action/[id]`:

- **StatusPill** (`succeeded` / `failed` / `refused` / `pending`)
  conveys status by background color. On paper, all pills become
  identical white blobs — auditors cannot tell what happened.
- **Worker avatars** (`components/ui/Avatar.tsx`) render gradient
  fallbacks. They print as empty squares.
- **Brand wordmark** (`linear-gradient(135deg, #8b5cf6, #ec4899)` in
  AppShell) prints as a blank square next to "SKYCOT" — first
  impression on a compliance binder cover is a missing logo.
- **Hero card** `bg-[var(--card)]` (a near-white card on a near-white
  page) probably prints OK by accident, but any token that resolves
  to a `--muted` background (decision-chain step rows, search-input
  background, range-select chrome) will silently vanish.

### 3. Nav / sidebar hidden in print — NO

`components/skycot/AppShell.tsx` wraps every authed page in
`<NavDrawer>` (left sidebar, 64-unit width: `md:pl-64`). The drawer
has no `print:hidden` class and no print-media rule scoping it out.

On paper the print-out will include:
- 256px-wide left column of nav items (Approvals, Audit, Workers,
  Cohorts, Skills, Workflows, etc.) on every page,
- the "SKYCOT" wordmark + "AI teammates" tagline,
- the `<WorkspaceFooter>` user button with "Sam Harlow · Acmeco · Owner",
- a sticky page header bar that re-prints at the top of every paginated
  page (because nothing demotes its `position: sticky` to `static` for
  print),
- the `<HelpWidget>` floating button (from layout.tsx) in the corner
  of every page.

The page-content column is forced to `max-w-5xl` (1024px) inside an
`md:pl-64` parent. On US Letter (8.5") with default 0.5" margins
that's ~750px usable width — meaning the **right side of every
audit row is clipped** because the layout was sized for a 1280px+
viewport, not paper.

### 4. Page-break behavior — NO RULES

```
$ rg "page-break|break-before|break-after|break-inside" web/ -> 0 hits
```

No `break-inside: avoid` anywhere. Specific consequences:

- **AuditDayGroup**: the day header (`TODAY`, `MAY 9`) uses
  `sticky top-0` which becomes nonsensical on paper. Browsers print
  sticky elements as static at their first natural position, so a
  long day group will lose its header at the page break and the
  reader cannot tell which day the second-page rows belong to.
- **DecisionChainCard**: 8-step ordered list, each step is a
  `<li>` with no `break-inside: avoid`. A long step body can split
  mid-paragraph with the step number on page N and the body on page
  N+1, breaking the forensic narrative.
- **ProvenanceBlock**: 5 rows in a `<dl>` — these can also split
  mid-row (label on page N, value on N+1).
- **ActivityRow** (the audit feed): every row in the feed is a
  flex container; they can break mid-row leaving worker avatar/name
  on one page and verb/status/timestamp on the next.

### 5. Print-friendly URL footer / provenance — NO

There is no `@page` rule with content margins, no
`content: counter(page) " of " counter(pages)`, and no static URL
footer block. Printouts will lack:
- Source URL (so the auditor cannot trace the paper back to the
  query that produced it).
- Tenant identifier (compliance binders typically need this on every
  page for chain-of-custody).
- Generation timestamp (the moment the snapshot was taken).
- Page numbers ("Page 3 of 17").
- Active filter state on `/audit` (`range=7d`, `worker=...`, etc.)
  — without this the paper is not reproducible from the URL.

---

## Specific cut-off / data-loss risks for `/audit/action/[id]`

Beyond the layout issues, the action-detail surface has
**structural data hiding** that defeats printing:

1. **ProvenanceBlock truncates hashes to 8 chars on screen**
   (`SHORT_HASH_LEN = 8` in `components/skycot/audit/ProvenanceBlock.tsx`).
   The full value lives only in (a) the `title=` tooltip — invisible on
   paper, and (b) the "Copy hash chain" clipboard payload — also paper-
   invisible. **The printed compliance binder will contain hashes the
   auditor cannot verify**, because `abc12345…` does not collide-
   resistantly identify the model, prompt, skill, memory, or policy.

2. **DecisionChainCard hides step details behind `<button>` toggles**
   (`open ? <pre>{JSON.stringify(step.details)}` only when expanded).
   Steps with collapsed details print only the title + body; the
   raw alternative-plans / memory-hashes / policy-decision JSON
   silently vanishes.

3. **ActionInputsCard / ActionOutputCard** (not read in this audit but
   referenced from page.tsx) are likely subject to the same pattern.

4. **RedactionBanner** at the top of the page is a colored info
   banner — its background prints as white, so a printout that should
   loudly signal "this view has 2 redacted categories" looks identical
   to a non-redacted view. A reviewer might not realize PII was hidden.

5. **Avatar `src` images** load from `https:` per next.config.ts
   `images.remotePatterns` (`api.skycot.com`, `try.skycot.com`). If the
   printer pulls before images load, or the user prints offline, paper
   shows blank squares where worker faces should be.

---

## Top 3 fixes for the `/audit` printout

1. **Add a real `@media print` block to `app/globals.css`** that:
   - hides `nav`, the sidebar (target `[data-slot="nav-drawer"]` or the
     `<NavDrawer>` root), the `<HelpWidget>`, the sticky `AuditPageHeader`,
     the `<Toaster>`, every button (`button` selector except buttons
     inside `<details>`),
   - forces `position: static` on every `.sticky` element so day-group
     headers don't disappear and the page header prints once at the top
     and only the top,
   - sets `print-color-adjust: exact` (and the `-webkit-` prefix) on
     `body` and on `[data-status]` / `StatusPill` so red/green status
     pills survive the printout,
   - sets `break-inside: avoid` on `[data-testid^="audit-decision-step-"]`,
     `[data-testid="audit-provenance-card"]`, every `[data-testid^="audit-row-wrapper-"]`,
     and every `<section>` in the action-detail page,
   - sets `@page { size: A4; margin: 1.5cm; }` with a `@bottom-right`
     content rule for page numbers and a `@top-left` rule for a
     URL/timestamp footer (Chrome supports the legacy syntax and
     `@page`).

2. **De-truncate provenance hashes when printing.** In
   `components/skycot/audit/ProvenanceBlock.tsx`, gate the
   `{shortHash(row.full)}` to screen-only and reveal `{row.full}` on
   print. Either swap two spans with `print:hidden` / `hidden print:inline-block`
   utilities, or use a `@media print { .short-hash { display: none; }
   .full-hash { display: inline; } }` rule. Without this the entire
   compliance use-case is unsound.

3. **Force-expand all `<details>` in DecisionChainCard for print, and
   render a print-only URL footer block.** Two surgical changes:
   (a) inject `@media print { details { display: block; } details > *
   { display: block !important; } [data-testid^="audit-decision-step-"]
   pre { max-height: none !important; overflow: visible !important; } }`
   — this forces every collapsed step's JSON to render on paper, and
   removes the screen-only 256px `max-h-64` clamp that would otherwise
   keep paper from showing the rest of the JSON.
   (b) Add a `<footer data-print-only>` to the action-detail page rendered
   `hidden print:block` containing: the action id, the source URL
   (`useReportUrl()` / `window.location.href`), tenant id, generation
   timestamp, and a chain-of-custody signature placeholder. Without
   this, a printed action page is just a screenshot without
   provenance — useless for any binder that needs to be defended in
   court or auditor review.

(A fourth low-cost win: add a small `<PrintButton>` in
`ActionSummaryHero` next to Replay / Export that calls
`window.print()` — currently the only way to print is the browser
chrome's File menu, which compliance auditors will not discover.)

---

## 5-Dimension Scorecard

| # | Dimension                                                | Score | Notes                                              |
|---|----------------------------------------------------------|-------|-----------------------------------------------------|
| 1 | Has `@media print` rules?                                | FAIL  | Zero. Entire codebase.                              |
| 2 | Backgrounds preserved (`print-color-adjust: exact`)?     | FAIL  | Not declared anywhere; StatusPills + avatars lost.  |
| 3 | Nav/sidebar/widgets hidden in print?                     | FAIL  | Sidebar, HelpWidget, sticky headers all print.      |
| 4 | Sensible page breaks (no mid-row splits)?                | FAIL  | No `break-inside: avoid`; sticky day headers vanish.|
| 5 | Print-friendly URL footer (paper has provenance)?        | FAIL  | No `@page`, no footer, no URL/tenant/timestamp.     |

**Overall: 0/5 — total print blindness.** This needs a Wave-9.x brief
to author a `print.css` (or a `@media print` block in `globals.css`)
that covers all 5 dimensions, plus the per-component fixes for the
hash-truncation and details-collapse data-loss patterns.
