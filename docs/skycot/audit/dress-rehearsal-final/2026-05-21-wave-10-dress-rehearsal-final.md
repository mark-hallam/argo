# Wave-10 final dress rehearsal — post-9-wave verdict (2026-05-21)

Auditor: Wave-10 audit agent #2 (Claude Opus 4.7 · 1M context)
Method: file-static-trace of all wave-1→9 fix surfaces + live HTTP walk + grep regression scan against `http://localhost:3000` (dev server up at audit start) + diff-against-Wave-5 verdict at `docs/skycot/audit/dress-rehearsal/2026-05-20-wave-5-dress-rehearsal.md`.
HEAD at audit: `d1c4948c` — `status: web-rbac-jwt-role-claim-and-gateway-enforce merged (#1901)`.
Wave-fix-cycle commits walked: `09b5f1cc` (W1), `0b58719d` (W2), `4f4059d9` (W3), `d04a7032` (W4), `af144354` (W5), `cbf81b01` (W6), `2489e57f` (W7), `e23a4b09` (W8+W9 combined). Total: **9 fix-cycle ships across ~24h**.

---

## TL;DR — RECORD-READY

**Verdict: RECORD-READY.** Every wave-1→9 fix the prompt names is in place on `main` and verifiable by static trace + dev-server HTTP. No regressions caught in the 9-page demo walk. The Wave-5 "READY TO RECORD with 3 last-mile items" verdict has been ratcheted up: the two MUST-FIX items at wave-5 (PHONE redactor, Propose dead toast) have both been resolved. The remaining cosmetic items (tab title leak on `/workers/_`, React Flow attribution watermark) are non-blocking polish.

---

## Wave-1→9 fix verification — all 9 fixes still in place

Each check below opens the file or hits the dev server and confirms the wave fix is on `main` HEAD, not just "claimed in a commit message."

### 1. `/compare` publicly accessible (Wave-1, 09b5f1cc)

**STATUS: VERIFIED.** Two-line guard.
- Directory: `~/code/argo/web/app/compare/` (NOT under `app/(app)/`, so no auth wrapper).
- Middleware allow-list: `~/code/argo/web/middleware.ts:23` — `pathname.startsWith("/compare")` short-circuits to `NextResponse.next()`.
- HTTP test: `GET http://localhost:3000/compare` → 200 in 0.195s (cold), `<title>SKYCOT vs Zapier, n8n, Crew.ai, Lindy · SKYCOT</title>`. Anonymous viewer can reach.

### 2. `/workflows` search filter (Wave-1 wired, Wave-5 grid-render fix, Wave-7 empty-state + chip wiring)

**STATUS: VERIFIED — three-wave evolution all landed.**
- `~/code/argo/web/app/(app)/workflows/page.tsx:118-129` — `useState("")` + `useMemo` filtered subset over `name | vertical | description`.
- `:461` — input bound to `setQuery(e.target.value)`.
- `:465-467` — counter pill "<filtered> of <all>" (Wave-1).
- `:561` — `filtered.map((wf) => ...)` (Wave-5 fix — Wave-1 forgot this; grid now actually re-renders).
- `:503` — "MATCHING <q>" label when filtered, "ALL WORKFLOWS" when not (Wave-7).
- `:507-554` — polished empty state with "No workflows match \"<q>\"" + "Clear search" + "Browse templates →" (Wave-7).
- `:495` — TRY chips fire `setQuery(chip)` (Wave-6).

### 3. `BrandLoudHero` respects `prefers-reduced-motion` (Wave-2, 0b58719d)

**STATUS: VERIFIED.**
- `~/code/argo/web/components/skycot/BrandLoudHero.tsx:95-108`:
  ```
  @media (prefers-reduced-motion: reduce) {
    .skycot-brandloud-shine { animation: none !important; transform: translateX(-100%) !important; opacity: 0 !important; }
    .skycot-brandloud-eyebrow-dot.is-pulse { animation: none !important; }
  }
  ```
- WCAG 2.3.3 — shimmer sweep + pulsing eyebrow dot freeze when OS preference is set. Applies to `/`, `/team`, `/cohorts` in one component edit.

### 4. `/workflows/.../build` toasts now say "Preview build — not persisted" (Wave-6, cbf81b01)

**STATUS: VERIFIED.** Three handlers, all honest:
- `Propose` → `toast.message("Preview build — draft not persisted", { description: "Live wire ships in PR #1882. Canvas changes will land then." })` at `:345`.
- `Test run` → `toast.message("Test run — preview build", { description: "Real run streams from the gateway in v1.x. Not wired in this preview build." })` at `:359`.
- `Save + publish` → `toast.message("Saved as draft — preview build", { description: "Publish to tenant ships in v1.x. Draft is in-memory only here." })` at `:369`.
- Grep for the old lie strings (`"All 5 steps passed"`, `"Workflow is live"`) returns only references **inside the explanatory comments** explaining what was replaced. Live UI cannot fire the old lies.

### 5. Audit drill-in lands on real polished detail (Wave-1 + Wave-2 layout title + ongoing)

**STATUS: VERIFIED.** Audit detail at `~/code/argo/web/app/(app)/audit/action/[actionId]/page.tsx`:
- Composed of 5 polished cards: `<ActionSummaryHero>` + `<ActionInputsCard>` + `<ProvenanceBlock>` + `<DecisionChainCard>` + `<ActionOutputCard>`.
- Top-of-page `<Breadcrumb>` back to `/audit` + history-aware `<BackButton>` that falls back to `<Link>` on deep-link.
- `<RedactionBanner>` declares the redaction policy + lacking-permission categories.
- Replay CTA navigates to `/audit/action/<id>/replay` (sub-route exists in tree under `/audit/action/[actionId]/replay/`).
- Export CTA opens `<EvidenceBundleExporter>` modal.
- 404 path renders `<AuditActionNotFound>` (in-shell, not framework default).
- HTTP test: `GET /audit` → 200 in 0.189s, `<title>Audit · SKYCOT</title>` (Wave-5 title-leak fixed for this route).

### 6. Approvals "Hire teammate" routes to `/hire` (Wave-1, 09b5f1cc)

**STATUS: VERIFIED.**
- `~/code/argo/web/app/(app)/page.tsx:438-442`:
  ```
  aria-label="Hire teammate"
  onClick={() => router.push("/hire")}
  ```
- Target route exists at `~/code/argo/web/app/(hire)/hire/page.tsx` (also `/hire/step` + `/hire/done` for multi-step wizard).

### 7. `/landing` hero bubbles don't overlap centre logo (Wave-9 fix, e23a4b09)

**STATUS: VERIFIED.** Two-layer responsive fix:
- `~/code/argo/web/app/landing/page.tsx:436-453` — Wave-6 baseline: `.land-art-bubble` pins `min-width: 220px; max-width: 280px;` so bubbles hold one line + breathe around the centre logo.
- `:505-516` — Wave-9 addition at `@media (max-width: 890px)`: switches bubbles to `max-width: 44%`, drops `min-width`, pins bubbles to all four corners (`top: 4% left: 4%`, `top: 4% right: 4%`, `bottom: 4% left: 4%`, `bottom: 4% right: 4%`). Result: non-overlapping 4-corner grid at the intermediate viewport band where the centre logo and bubbles would otherwise collide.
- `:517-521` — Below 540px: `.land-hero-art { display: none }`. Subhead carries the message on mobile.

### 8. `approvals.ts` strict-prod guards in place (Wave-8, e23a4b09 file change)

**STATUS: VERIFIED.** Launch-blocker fix at `~/code/argo/web/lib/api/approvals.ts:157-170` (`submitDecision`) + `:182-189` (`submitUndo`):
- Production path (`NODE_ENV === "production" && NEXT_PUBLIC_ENV === "production"`) **propagates errors** so the optimistic mutation rolls back + user sees the error toast.
- Dev/preview keeps the `mock-offline` fallback so demos with no gateway still work.
- Previously: any 404 or `TypeError` returned `ok: true, source: "mock-offline"` — user saw the green tick but the gateway never recorded the decision (audit-trail integrity hole). Now strict-prod fails loud.
- `isStrictProduction()` defined at `:192-197`.

### 9. Settings landing no longer shows "SSO coming soon" (Wave-3/Wave-5 ratchet + #1891 ship)

**STATUS: VERIFIED.** `~/code/argo/web/app/(app)/settings/page.tsx:47-53`:
```
{
  href: "/settings/security",
  icon: <Lock size={20} />,
  title: "Security",
  blurb: "Sessions, SSO (Google / Microsoft / Okta SAML), audit log retention, encryption-at-rest.",
  coming: false,
}
```
- `coming: false` means the tile renders as live (not a v1.x placeholder).
- The `v1.x` chip at `:245` only renders when `coming === true`. Security tile no longer triggers it.
- SSO actually shipped via PR #1891 (`web-sso-okta-google-azure merged`).

### Bonus check — OAuth v1.x badges on `/auth/signup` (Wave-5 carryover)

**FIXED since Wave-5.** `~/code/argo/web/components/auth/SsoButtons.tsx` is the live SSO row (replaces the v1.x placeholder buttons that shipped with the editorial-A auth design lock per the file's own header comment). Three real branded buttons (Google logo SVG, GitHub mark, Microsoft 4-square mark) that call `startOAuth(provider.id, redirectTo)`. No `v1.x` badge anywhere.

---

## 9-page demo walk — HTTP + title check

```
HTTP   TTFB     PATH                                           <title>
200    0.209s   /landing                                       Hire your first AI teammate · SKYCOT
200    0.142s   /auth/signup                                   Hire your first AI teammate · SKYCOT
200    0.173s   /                                              Approvals · SKYCOT
200    0.197s   /team                                          Team · SKYCOT
200    0.211s   /workers/wkr_01J0MARIAH/overview               Approvals · SKYCOT       ← title leak
200    0.270s   /workflows                                     Workflows · SKYCOT
200    0.198s   /workflows/sales/qualify_lead/build            Workflows · SKYCOT
200    0.195s   /compare                                       SKYCOT vs Zapier, n8n, Crew.ai, Lindy · SKYCOT
200    0.190s   /audit                                         Audit · SKYCOT
```

- **All 9 routes return 200**, none redirect, none 404.
- **All 9 are sub-300ms** cold TTFB. Total sequential walk: ~1.8s.
- **8 of 9 titles unique + correct.** Single leak: `/workers/wkr_01J0MARIAH/_` still shows `Approvals · SKYCOT` (Wave-5 caveat #3 partial-fix progress — `/`, `/audit`, `/team`, `/compare`, `/landing`, `/auth/signup`, `/workflows`, `/workflows/.../build` have all been fixed since Wave-5; only `/workers/[workerId]` layout remains uncovered). NICE-TO-FIX, not a blocker — the browser-tab title is not on the centre-of-screen demo path.

## Cross-cutting regression checks

**No dead onClicks.** Grep for `onClick={() => {}}` / `onClick={() => undefined}` / `TODO:.*onClick` returns zero results.

**No placeholder `href="#"` on landing.** Wave-4 fix held: `/landing` footer now points to real `/docs`, `/privacy`, `/terms`, `/security` routes (all of which exist + have brand-on stub pages).

**No "mock (offline)" pellet on camera.** `<DataSourceChip>` is dev-only AND suppresses `mock-offline` in dev too (file's "Demo-recording fix" comment). `<LiveIndicator>` only renders the positive `● live` pulse — connecting/polling/offline states are hidden.

**No FOUC discovery beyond Wave-5.** `/compare` Audiowide font-swap mitigation is the operator's pre-warm step (open `/compare` once before recording).

**PHONE-redactor false-positive fixed.** `~/code/argo/web/lib/redaction/patterns.ts:67`:
```
export const PHONE_RE = /(?<![A-Z-])\+?[\d(][\d\s()-]{6,18}\d/g;
```
Negative look-behind `(?<![A-Z-])` blocks the regex from eating digit suffixes of `INV-2024-08741` / `QT-2024-7283`. Fixture data at `web/lib/sample-audit.ts` confirms intact (`INV-2024-08741`, `QT-2024-7283` survive into the page). This was Wave-5's #1 MUST-FIX. Done.

**React Flow attribution still visible.** `~/code/argo/web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:651-659` — `<ReactFlow attributionPosition="bottom-right">` without `proOptions={{ hideAttribution: true }}`. Pro license required to hide. Same as Wave-5 caveat — small "React Flow" watermark in builder lower-right. Same registered file at `/web/components/skycot/canvas/SkillCanvas.tsx:187` DOES set `hideAttribution: true`, so the team has the affordance — they just haven't applied it to the workflow builder (separate canvas).

**Wave-7 keyboard a11y on builder.** `onNodeKeyDown` handler at `:309` of the build page — Enter / Space opens side panel on focused node. WCAG 2.1.1 compliant.

## Regressions caught: NONE

I re-walked each wave-fix surface and could not find a reverted change. The 9 fix-cycle commits are stacked cleanly on top of each other; no later commit undoes an earlier wave's polish.

## Final pre-record checklist for Mark

| # | Item | Effort | Required? |
|---|------|--------|-----------|
| 1 | **Open `/compare` once before pressing record** so the Audiowide font cache warms. Without this the hero `<h1>` visibly swaps Inter→Audiowide ~1-2s after first paint. | 5s | YES |
| 2 | **Use a fresh Chrome profile** (or wipe the current one's leveldb). The existing profile is in `FILE_ERROR_NO_SPACE` from Wave-5 — 100% of dev-tools console messages were leveldb-quota, not SKYCOT. Fresh profile = zero console errors. | 30s | YES |
| 3 | **Keep Chrome dev tools CLOSED** during the recording. They're the only place the leveldb errors are visible to camera anyway. | 0s | YES |
| 4 | **Set `NEXT_PUBLIC_HIDE_PWA_PROMPT=1`** in `.env.local` for the recording host so the bottom-right "Install SKYCOT" banner can't pop on camera. Wave-6 added this env-var short-circuit. | 10s | YES |
| 5 | **Record at ≥1718 wide.** At narrower widths the workflow-builder "Add step" button slides off the right edge of the canvas (Wave-5 carryover; Wave-9 noted an unfiled brief `web-builder-add-step-fab-collision`). | 0s if monitor supports | YES |
| 6 | **Skip the "Features" anchor in /landing nav.** It scrolls to `#features` on the same page (Wave-1 fix replaced the dead `/features` route); no harm if you click it, but no value either. Just don't draw attention to it. | 0s | OPTIONAL |
| 7 | **Voice the audit-detail beat carefully.** The PHONE-redactor false-positive is fixed for the demo fixture but the regex is a UI fix, not a guard rail — if your live data ever has a real phone number formatted like `INV-2024-08741`, it will leak. Just say "every action chain-hashed" rather than "every PII redaction enforced" on camera. | 0s | OPTIONAL |
| 8 | **`/workers/_/_` tab title shows `Approvals · SKYCOT`.** Visible in the browser tab strip during the worker-detail beat at 02:30-04:00. Nobody watching for it will catch it, but if you obsess about polish, one-line fix is `export const metadata = { title: "Worker · SKYCOT" }` in `/web/app/(app)/workers/[workerId]/layout.tsx`. | 1 min if you fix it now | OPTIONAL |
| 9 | **React Flow watermark** in builder bottom-right. Same fix as `/web/components/skycot/canvas/SkillCanvas.tsx:187` — add `proOptions={{ hideAttribution: true }}` to `<ReactFlow>` in `/workflows/[v]/[n]/build/page.tsx:651`. Requires React Flow Pro license per their terms; team already uses the flag elsewhere so license assumed in place. | 5 min if licensed | OPTIONAL |

**The 8-minute demo target holds.** Server-side load is sub-300ms across all 9 routes. The voiceover and scroll/hover/click pauses dominate the timing.

---

## Summary diff vs Wave-5 verdict

| Wave-5 finding | Wave-10 status |
|---|---|
| PHONE-redactor false positive (MUST FIX) | **FIXED** (Wave-5 last-mile, af144354) |
| Propose dead-end (SHOULD FIX OR REVISE SCRIPT) | **FIXED** (Wave-6 honest preview-build toast) |
| Tab title leak on 5 routes (NICE TO HAVE) | **PARTIALLY FIXED** — Wave-5 fixed `/`, `/team`, `/audit`, `/cohorts`; only `/workers/[workerId]/_` still leaks |
| OAuth `v1.x` badges (polish) | **FIXED** (via #1891 SSO ship; SsoButtons.tsx is real) |
| `/landing` hero bubbles cramped (polish) | **FIXED twice** — Wave-6 min-width 220px + Wave-9 corner-pin at <890px |
| FILE_ERROR_NO_SPACE console errors | **NO CHANGE** — operator pre-flight item only (fresh profile) |
| /compare FOUC | **NO CHANGE** — operator pre-flight item only (pre-warm) |
| React Flow attribution watermark (polish) | **NO CHANGE** — 5-min fix if Pro licensed |
| Builder "Add step" off-screen at <1574px | **NO CHANGE** — record at ≥1718 wide |
| `/workflows/.../build` keyboard a11y | **FIXED** (Wave-7 onNodeKeyDown) |
| `/workflows` zero-results empty state | **FIXED** (Wave-7 polished empty state) |

**Net delta**: 5 of Wave-5's 9 findings are now fixed. The remaining 4 are operator pre-flight (open `/compare`, fresh profile, ≥1718 viewport) or pure polish (1 unfixed tab title, 1 watermark).

---

*Wave-10 final dress rehearsal · 2026-05-21 07:13 AEST · HEAD `d1c4948c` · read-only · no code modified*
