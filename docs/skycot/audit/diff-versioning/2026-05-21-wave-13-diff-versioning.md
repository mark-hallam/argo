# Wave-13 Audit — Workflow Diff + Versioning + History UX

**Audit window**: 2026-05-21
**Auditor**: claude (Wave-13 audit agent #5)
**Scope**: `/workflows/[v]/[name]` (visual viewer) + `/verticals/[v]/workflows/[name]` (versions list + diff)
**Promise audited**: from `/landing` page line 46 — *"Every workflow change versioned like code. Side-by-side diff. Rollback shipping next."*

---

## TL;DR

| Surface | Status | Score |
| --- | --- | --- |
| Versioning (DB substrate) | SHIPPED — `argo-store::workflow_definition` + audit kinds wired | 9 / 10 |
| Side-by-side diff (component) | SHIPPED — `<WorkflowDiff>` LCS-driven, RSC-safe | 8 / 10 |
| Version history list | SHIPPED but ORPHAN-LIKE — buried at `/verticals/...` route, not the surface buyers will demo | 4 / 10 |
| Per-step added/changed/removed | **MISSING** — diff is TOML line-level only, not step-semantic | 2 / 10 |
| Rollback UI | **MISSING ENTIRELY** — no button, no "coming soon", invisible to operator | 1 / 10 |
| Author + timestamp on history | TIMESTAMP only, no AUTHOR rendered | 4 / 10 |
| Audit-row connection | WIRED — `WorkflowDefinitionCreated` / `WorkflowHeadChanged` / `WorkflowVersionArchived` emitted on routes | 9 / 10 |
| Versions dropdown in viewer | **MISSING** — viewer renders a hardcoded seed `version: "v6"` span; no way to switch versions in-place | 1 / 10 |

**Overall versioning score: 4.75 / 10** — substrate is real, surface is broken-promise territory for a demo.

---

## File inventory

### Promise sources (marketing copy)
- `web/app/landing/page.tsx:46` — *"Every workflow change versioned like code. Side-by-side diff. Rollback shipping next."*
- `web/app/(app)/workflows/layout.tsx` — *"versioned like code, edited in chat"*
- `web/app/(app)/workflows/page.tsx` — *"version them like code, ship to production in one click"*

### Diff infrastructure (shipped, working)
- `web/components/skycot/WorkflowDiff.tsx` — 230 LOC pure-render component, side-by-side on ≥ md viewports, stacked on narrow. RSC-safe. Uses `<DiffStats>` for +/- counts and `<HunkBlock>` for grouped hunks with `@@ -from,len +to,len @@` headers.
- `web/lib/diff/unified.ts` — 162 LOC self-contained Hunt–Szymanski LCS implementation, no external dep. Returns `{ hunks: DiffHunk[], unchanged: boolean }`.
- `web/app/(app)/verticals/[verticalId]/workflows/[name]/diff/page.tsx` — 100 LOC server-rendered route at `?from=<id>&to=<id>`. Calls `apiGet` against `/api/v1/verticals/.../workflows/.../versions/<id>`.

### Version-history list (shipped, working — but orphaned)
- `web/app/(app)/verticals/[verticalId]/workflows/[name]/page.tsx` — 161 LOC server-rendered route. `<VersionTable>` renders `<ol>` newest-first with `is_head` badge, `created_at` timestamp, "Compare with head" link per row. Calls `/api/v1/verticals/.../workflows/.../versions`.

### Audit-row emission (verified)
- `crates/argo-audit/src/event.rs` — defines `WorkflowDefinitionCreated`, `WorkflowHeadChanged`, `WorkflowVersionArchived`.
- `crates/argo-cli/src/handlers/gateway/routes_workflows_handlers.rs:152, 199, 244` — emits these on POST (new definition), PUT head (publish/promote), DELETE (archive).

### Backend brief shipped
- `mission-pm-workflow-rollback-cli` — codex CLAIMED at row 848 of BUILD_STATUS.md as of 2026-05-19. Status: still CLAIMED 2 days later — has been unblocked since deps merged but not closed.

### Web rollback brief unshipped
- `web-workflow-rollback-feature` — TODO, ZOMBIE-demoted 2026-05-21 by conductor in row 920. The brief itself is excellent (clear DB op → gateway → modal → audit-row plan, 500-800 LOC) but no one is claiming it.

### Visual viewer — the "demo path" `/workflows/<v>/<name>`
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` — 946 LOC. **This is what buyers see in the demo**.
  - Renders a hardcoded seed `version: "v6"` static `<span className="wfd-version">` (line 749).
  - "Version history" button (line 760-765) links to `/verticals/${vertical}/workflows/${name}` — the orphaned versions list.
  - **No versions dropdown**, no in-place version-switcher, no per-version diff.
  - **No rollback button**, no rollback CTA, no "coming soon" placeholder.

---

## Findings (5)

### F1 — Versions dropdown missing on the demo-facing viewer (CRITICAL)
The `/workflows/<v>/<name>` viewer is where every buyer demo lands. It currently renders a static seed `version: "v6"` (or v3, v4, etc.) as plain text. There is **no UI affordance to switch to a different version in-place**. Operator must:

1. Click "Version history" button (jumps off to `/verticals/<v>/workflows/<n>` — a different URL slug entirely).
2. Click "Compare with head" on a row (jumps to `/verticals/.../diff?from=X&to=Y`).
3. Read the diff to understand what changed.
4. Hit back twice to return.

This is 4 clicks to do the most basic versioning task. Class-leading expectation: dropdown on viewer header, switch in place, "viewing v4 (3 versions behind head)" badge.

**Fix size**: 80-150 LOC in `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` — add `<select>` with version list + URL `?version=<id>` + server fetch.

### F2 — Diff is TOML line-level, not step-semantic (HIGH)
`compute_unified()` in `web/lib/diff/unified.ts` operates on raw TOML strings. A workflow that has step #3 modified shows as:

```
- [[steps]]
- id = "s3"
- kind = "decision"
- name = "Branch on BANT score"
+ [[steps]]
+ id = "s3"
+ kind = "decision"
+ name = "Branch on BANT score (>= 80)"
```

A buyer looking at this thinks "why are 4 lines removed and 4 added when only one word changed?" Class-leading UX surfaces step-level adds/changes/removes:

> **Step 4 modified**: `name` "Branch on BANT score" → "Branch on BANT score (>= 80)"

**Fix size**: ~300 LOC. Parse both TOMLs into `WorkflowStep[]`, diff by step id, classify as added/changed/removed. The seed data already declares `WorkflowStep` interface — reuse it.

### F3 — Rollback UI MISSING ENTIRELY (CRITICAL — buyer-catchable lie)
Landing-page copy says "Rollback shipping next." Operator clicks through to `/workflows/<v>/<name>` looking for the affordance — sees nothing. Clicks "Version history" — sees a list of versions with "Compare with head" links but **no "Rollback to v3" button, no greyed-out placeholder, no "Coming soon"**.

The wave-5 close-the-loop audit (`docs/skycot/audit/close-the-loop/2026-05-20-wave-5-close-the-loop.md`) softened the landing copy from "One-tap rollback" → "Rollback shipping next" to make the gap survivable. The brief `web-workflow-rollback-feature.md` exists and is TODO (zombie-demoted 2026-05-21). It's the highest-leverage unshipped P1 in this audit.

**Fix**: claim and ship `web-workflow-rollback-feature.md` — 500-800 LOC across argo-store rollback op + gateway endpoint + `<RollbackConfirmModal>` + viewer wire-up + audit row + tests.

### F4 — Author (created_by) not rendered (MEDIUM)
`VersionListItem` declares `created_by?: string` (line 21 of `/verticals/.../workflows/.../page.tsx`) but the `<VersionTable>` never renders it. Operator sees "2026-05-20T14:32:00Z" but not "Mariah". The audit page's competitor (Etherscan-style) renders both. Class-leading UX expectation:

> **v6** `head` — Mariah · 4 hours ago — "Added approval before posting"

**Fix size**: 30 LOC. Render `v.created_by` next to `v.created_at`; add a `commit_message` field to the schema (the brief for that doesn't exist yet — flag).

### F5 — Version-list orphaned at non-discoverable URL (MEDIUM)
`/verticals/<v>/workflows/<n>` is a different URL than the buyer's primary `/workflows/<v>/<n>`. The breadcrumb at the top of the version page goes to `/orchestration` (line 46) which is yet ANOTHER URL slug. Information architecture is fractured. A buyer/operator doesn't intuit that `/workflows/...` and `/verticals/.../workflows/...` are the same thing displayed differently.

**Fix**: merge the two surfaces. Either route `/workflows/<v>/<n>?tab=history` displays the versions list (one URL, two tabs), or the version history lives in a side drawer on the viewer page.

---

## Demo readiness verdict

If a buyer asks "show me how you handle versioning":

1. The visual viewer at `/workflows/<v>/<n>` shows version "v6" as static text — **fail**.
2. Clicking "Version history" opens a brand-mismatched route under `/verticals/...` — **looks like a different app**.
3. Side-by-side diff works once you reach `/verticals/.../diff` — **wins**, but only after 2 wrong clicks.
4. Asking "can I roll back?" — **no affordance anywhere** — confirms the landing claim is unmet.
5. Asking "who changed this?" — **timestamp only**, no author — confirms `versioned-like-code` is half-built.

**Demo recommendation**: Wave-13 should add F1 (versions dropdown in `/workflows/<v>/<n>`) + F3 (claim and ship `web-workflow-rollback-feature`) before any v1.0 demo recording. F2, F4, F5 can land v1.x.

---

## Top 3 versioning gaps (prioritized for v1.0)

1. **Rollback UI completely invisible** — `web-workflow-rollback-feature.md` brief is authored and unclaimed. Buyer-catchable broken promise. P0.
2. **No versions dropdown in the buyer-facing viewer** — they have to leave the page to compare versions. P1.
3. **TOML line-diff is not step-semantic** — class-leading competitors show "Step 4 changed", not 8 lines of TOML noise. P2 polish for v1.x.

---

## 250-word executive summary

SKYCOT's "every change versioned like code, side-by-side diff, rollback shipping next" promise is half-built: the DB substrate is solid (workflow_definition table, three audit kinds wired into the gateway), the side-by-side diff component is excellent (pure-LCS, RSC-safe, ~230 LOC, stacks on mobile, hunk-grouped, +/- stats), and the version-list route renders. But the UX is fractured across two URL roots — buyers demo at `/workflows/<v>/<n>` while the version-list and diff live at `/verticals/<v>/workflows/<n>/...`, which feels like a different app entirely. The visible viewer shows version "v6" as static text with no dropdown to switch in place; the version-history button jumps the operator to the orphan route. Rollback is completely invisible — no greyed-out button, no "coming soon" copy, the landing-page promise ("Rollback shipping next") finds zero surface to back it up; the `web-workflow-rollback-feature.md` brief is authored and zombie-demoted, waiting to be claimed. The diff itself is TOML line-level, not step-semantic, so a buyer sees 8 lines of churn for a 1-word change. Author (created_by) exists in the type but isn't rendered on the version list — only timestamp. Versioning score: 4.75 / 10. Diff quality: 7 / 10 (great component, wrong abstraction layer). Rollback status: 0 / 10 (DB ready, no UI). Top 3 gaps: (1) ship `web-workflow-rollback-feature`, (2) add versions dropdown to the demo-facing viewer, (3) make the diff step-semantic.
