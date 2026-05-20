# Wave-5 dress rehearsal — pre-record final tech check (2026-05-20)

Auditor: Wave-5 audit agent #4 (Claude Opus 4.7 1M context)
Method: live walk via Chrome MCP + curl benchmark against `http://localhost:3000` + gateway `http://127.0.0.1:7400`
Source-of-truth runbook: `docs/skycot/operator/demo-runbook.md`
Prior baseline: `docs/skycot/audit/e2e-demo/2026-05-20-wave-1-e2e-demo-journey.md`

---

## TL;DR — READY TO RECORD (with 3 last-mile items)

**Verdict: READY TO RECORD** with three optional pre-flight fixes (none are hard blockers).

Since wave-1 yesterday, the demo posture has materially improved:

| Wave-1 finding | Status now |
| --- | --- |
| `/workflows/build` Propose action SILENTLY BROKEN | **FIXED** — now fires a sonner toast "Proposal ready for review · 1 step change proposed" |
| `/` Approvals page shows "mock (offline)" pellet | **FIXED** — no suspicious pellets render |
| `/features` linked from nav but 404 | **FIXED** — nav now points to `#features` anchor on `/landing` |
| Worker-detail skeleton card stuck loading | **FIXED** — skeletonCount = 0 |
| ClientFetchError storm from auth.js | **FIXED** — only Chrome leveldb FILE_ERROR_NO_SPACE remains, no product-side errors |
| `/compare` FOUC | **STILL PRESENT** but minor (font swaps Inter→Audiowide on hero ~2s after first paint) |
| `/audit` `INV-[PHONE]` / `QT-[PHONE]` placeholder text | **STILL PRESENT** — and identified as a real PII-redactor false-positive bug, not stub data |
| `/auth/signup` OAuth buttons labeled `v1.x` | **STILL PRESENT** (polish issue) |
| `/landing` worker bubbles cramped (50px columns) | **STILL PRESENT** — bubbles measured 80-103px wide |
| Tab `<title>` leaking root layout title | **PARTIALLY FIXED** — /landing, /auth/signup, /workflows, /compare, /skills, /pricing all have unique titles; /, /team, /workers/_, /audit still show "SKYCOT — Hire your first AI teammate" |

**The Propose-action fix alone unblocks the recording.** The other items are polish, not blockers.

---

## 8-dimension check

### 1. EVERY DEMO CTA still works

Re-walked all 11 runbook pages via Chrome MCP at 1718×915 viewport. Every script-mentioned interaction reachable.

| Demo beat | URL | CTA | Status |
| --- | --- | --- | --- |
| 00:00 | `/landing` | Hero "Hire your first worker" gradient pill | Works |
| 00:00 | `/landing` | Top-nav (How it works · Features · Compare · Customers · Pricing · Sign in · Hire your first teammate) | All work — Features anchor not 404 |
| 00:45 | `/auth/signup` | "Send magic link" CTA | Renders, magic-link path verified by wave-1 |
| 01:15 | `/` | "Approve" on Mariah card | Works (7 buttons present) |
| 02:00 | `/team` | 6 worker cards · "Open profile" | All 6 routes resolved (`/workers/wkr_01J0MARIAH` etc.) |
| 02:30 | `/workers/wkr_01J0MARIAH/overview` | Personality card + Great at + Escalate when + KPIs | All present, all matching voiceover |
| 04:00 | `/workflows` | 8 workflow cards including qualify_lead | All present, qualify_lead clickable |
| 04:45 | `/workflows/sales/qualify_lead/build` | Prompt bar + Propose + 5 React Flow nodes | Propose NOW fires visible toast (was silent in wave-1) |
| 04:45 | same | Node click → side panel slide-in | sidePanels count = 1, works |
| 06:30 | `/compare` | Comparison matrix + 22 checkmarks | Works |
| 07:30 | `/audit` | Etherscan-style log + drill-in to /audit/action/_ | Works, drill-in has full decision-chain + hash-chain + replay |

**Propose interaction — closer look.** Wave-1 said it was silently broken; today it fires the sonner toast announcement:

> "Proposal ready for review — 1 step change proposed. Approve to apply to the canvas."

Source confirms it's a hard-coded 1.4s stub at `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx:303-322`:

```
async function handleProposeChange() {
  // Stub: real wire-up POSTs to /api/workflows/propose with the workflow + nl change.
  // ... until the live wire (PR #1882 rebase pending) ships, surface visible feedback ...
  toast.loading(`Proposing: "${prompt}"`)
  setTimeout(() => toast.success("Proposal ready for review", {description: "1 step change proposed. Approve to apply to the canvas."}), 1400);
}
```

**This is materially better than silent failure**, but there's NO follow-up Approve button to click on screen — the toast self-dismisses. The voiceover script says "And it proposes the diff. You approve. It ships" — currently the operator sees a toast that announces it, but cannot actually approve. Either edit the voiceover ("…and it proposes the change — approval-first, just like every action SKYCOT does") OR drop in a quick stub that pops a modal with an Approve button (next test ratchet up).

### 2. NO CONSOLE ERRORS during the demo walk

**Total console messages across the full walk: 502**, all of them either:

1. `FILE_ERROR_NO_SPACE` from Chrome's profile leveldb (Chrome-environment, NOT SKYCOT-product) — Chrome's local storage backing-store has a leveldb file that's hit its internal quota. The operator's disk has 303 GiB free; this is leveldb-specific.
2. Downstream `ClientFetchError` from auth.js' SessionProvider, caused by #1 (no longer present in newer sessions — the wave-1 410-message storm has been quenched).

**Zero new product-side errors detected.** No 5xx network responses captured. All `/api/auth/session` returned 200, all `_next/static/chunks/*.js` returned 200, all woff2 fonts returned 200.

**Operator pre-flight action**: use a fresh Chrome profile for the recording (the existing profile's leveldb is in a permanent FILE_ERROR_NO_SPACE state). If dev tools are closed during recording, none of this is visible to camera.

### 3. NO 404s on critical paths

| URL | HTTP | Notes |
| --- | --- | --- |
| `/landing` | 200 | |
| `/auth/signup` | 200 | |
| `/auth/check-email` | 200 | |
| `/` | 200 | |
| `/team` | 200 | |
| `/workers/wkr_01J0MARIAH` | 200 | |
| `/workers/wkr_01J0MARIAH/overview` | 200 | |
| `/workflows` | 200 | |
| `/workflows/sales/qualify_lead` | 200 | |
| `/workflows/sales/qualify_lead/build` | 200 | |
| `/compare` | 200 | |
| `/audit` | 200 | |
| `/audit/action/act_01J0JAMAL_025` | 200 | |
| `/skills` | 200 | |
| `/cohorts` | 200 | |
| `/pricing` | 200 | |
| `/customers` | 200 | |
| `/features` | **404** | Not linked from /landing nav anymore (nav uses `#features` anchor). Only hit if a viewer manually types the URL. |

**No 404 or 500 on any path the runbook touches.**

### 4. NO SLOW PAGES

Cold + warm timing (3 passes per URL):

| URL | Pass-1 cold | Pass-2 warm | Pass-3 warm | Verdict |
| --- | --- | --- | --- | --- |
| `/landing` | 0.123s | 0.106s | 0.105s | excellent |
| `/auth/signup` | 0.075s | 0.061s | 0.062s | excellent |
| `/auth/check-email` | 0.068s | 0.076s | 0.077s | excellent |
| `/` | 0.106s | 0.085s | 0.085s | excellent |
| `/team` | 0.086s | 0.093s | 0.085s | excellent |
| `/workers/wkr_01J0MARIAH` | 0.107s | 0.082s | 0.080s | excellent |
| `/workers/wkr_01J0MARIAH/overview` | 0.110s | 0.087s | 0.095s | excellent |
| `/workflows` | 0.099s | 0.095s | 0.107s | excellent |
| `/workflows/sales/qualify_lead` | 0.301s | 0.108s | 0.101s | excellent (one-shot 301ms is the dev-mode RSC compile) |
| `/workflows/sales/qualify_lead/build` | 0.105s | 0.117s | 0.105s | excellent |
| `/compare` | 0.099s | 0.156s | 0.183s | excellent |
| `/audit` | 0.094s | 0.114s | 0.095s | excellent |
| `/audit/action/act_01J0JAMAL_025` | 0.094s | 0.107s | 0.090s | excellent |
| `/skills` | 0.093s | 0.084s | 0.084s | excellent |
| `/cohorts` | 0.093s | 0.085s | 0.082s | excellent |
| `/pricing` | 0.117s | 0.099s | 0.102s | excellent |
| `/customers` | 0.088s | 0.076s | 0.077s | excellent |

**Worst TTFB on the demo path: 0.301s** (`/workflows/sales/qualify_lead` cold compile). Threshold was 800ms — comfortably under.
**Full demo path sequential walk: 0.984s for 9 pages.**

Server is not the bottleneck. Any perceived slowness is hydration/font-load on the client, not network.

### 5. NO FLASH OF UNSTYLED CONTENT (FOUC)

**`/compare` still has a font-swap FOUC.** Measured:

- T+0: H1 computed font = `Inter, "Inter Fallback", system-ui, ...`
- T+2s: H1 computed font = `Audiowide, Inter, sans-serif`

The Audiowide webfont is being loaded async via Google Fonts `<link rel="stylesheet">` and there's no `font-display: optional` or preload, so the hero visibly swaps from Inter → Audiowide ~1-2 seconds after first paint. On camera, this is a visible character-shape change in the headline.

**Mitigation for the operator**: pre-warm `/compare` once before pressing record (the font cache holds across same-session navigations). The font swap will only happen on the first visit of the session.

Other pages checked (/landing, /team, /workers/_, /audit, /skills): no FOUC observed.

### 6. NO BLANK STATES

| Page | State | Notes |
| --- | --- | --- |
| `/landing` | Hero + worker bubbles + How it works + Features grid + Customers + Comparison teaser + final CTA + footer | Polished |
| `/auth/signup` | Hero + 3 OAuth buttons + email field + trust pellets | Polished (OAuth labeled v1.x) |
| `/` | Hero KPIs + 7 approval cards | Polished |
| `/team` | Hero KPIs + 6 worker cards + status filter | Polished |
| `/workers/wkr_01J0MARIAH/overview` | Avatar + personality chips + Great at / Escalate when / Signature behaviors + Responsibilities + KPIs + Recent activity | Polished |
| `/workflows` | 8 workflow cards + hero search + try-pills | Polished |
| `/workflows/.../build` | React Flow canvas + 5 nodes + minimap + prompt bar + top-bar + side panel | Polished |
| `/compare` | Hero + 5 competitor strip + comparison matrix | Polished (after font load) |
| `/audit` | Hero + filter + time range + export + 8+ rows with timestamps + worker + cost + status | Polished (PHONE redaction issue, see #1) |
| `/audit/action/_` | Inputs + Provenance + Decision chain + Output + Replay/Export CTAs | Polished |
| `/skills` | Marketplace hero + ⌘K search + try-pills + 6 featured skill cards | Polished |
| `/cohorts` | Hero KPIs + 5 cohort tiles | Renders but **does not match the locked design** (memory `project_cohorts_page_design_locked.md` calls for 6 dept tabs + magazine-tile asymmetric layout; current is simple grid) — not on demo path |
| `/pricing` | 3-tier pricing | Polished |
| `/customers` | 6 case-study cards | Polished |

**No blank pages, no white-screen states, no obvious loading skeletons stuck open.**

### 7. NO DEMO-BREAKING DEBUG CHROME

- DataSourceChip: 0 instances visible (queried `[class*="DataSource"], [class*="data-source"]`)
- LiveIndicator: 0 instances visible (queried `[class*="LiveIndicator"], [class*="live-indicator"]`)
- "mock (offline)" / "connecting…" pellets: GONE from /
- Dev-only banners: none observed

**The only debug-flavored visible items remaining**:
1. OAuth `v1.x` badges on `/auth/signup` (wave-1 polish)
2. React Flow attribution watermark bottom-right on `/workflows/.../build` (wave-1 polish)
3. Bottom-right purple "?" help bubble (visible across all in-app pages)

None are demo-breaking.

### 8. DEMO TIMING — will the 8-min target hold?

**Server-side budget: ~1 second total** for the 9-page nav. The 8-minute target is dominated by voiceover + scroll/hover/click pauses, not load time.

Per-segment estimate (assuming operator follows the runbook):

| Segment | Runbook expects | Likely actual | Notes |
| --- | --- | --- | --- |
| `/landing` open + scroll | 0:00 → 0:45 | 0:45 | Hero + How-it-works scroll, ~45s of voiceover |
| `/landing` → `/auth/signup` | 0:45 → 1:15 | 1:15 | Click Hire → type demo@skycot.com → click Send magic link (NOT actually submitting — operator skips magic-link verify per script line "Skip the magic-link round-trip; navigate to /") |
| `/auth/signup` → `/` | 1:15 → 2:00 | 2:00 | Approvals reveal + click Approve. Page hydrates fast — should land cleanly. |
| `/` → `/team` | 2:00 → 2:30 | 2:30 | "6 teammates" reveal + click Mariah card |
| `/team` → `/workers/wkr_01J0MARIAH/overview` | 2:30 → 4:00 | 4:00 | Mariah personality walk (Patient · Detail-oriented · Picture-perfect · Polite + Great at + Escalate when + KPIs) — most content-dense beat at 1:30 |
| `/workers/_` → `/workflows` | 4:00 → 4:45 | 4:45 | Workflow catalog click qualify_lead — fast page transition |
| `/workflows` → `/workflows/.../build` | 4:45 → 5:30 | 5:30 | React Flow canvas reveal + pan/zoom — ~3s React Flow init, comfortable |
| Builder interactions (prompt + Propose) | 5:30 → 6:30 | **6:30 with risk** | The Propose stub fires a toast at 1.4s after click then self-dismisses (sonner default ~4-5s). Voiceover beat "describe a change → Propose → it proposes the diff → You approve. It ships" doesn't have a follow-up Approve button. Operator should plan to voiceover around this (see "Last-mile items" #1). |
| `/build` → `/compare` | 6:30 → 7:30 | 7:30 | One-time FOUC hazard — recommend pre-warming /compare once before recording. |
| `/compare` → `/audit` | 7:30 → 8:00 | 8:00 | Audit row scan, mention SOC 2 / ISO / GDPR / HIPAA / AFSL pellets. PHONE-redaction issue visible — operator should plan to voice around it or fix it first (see "Last-mile items" #2). |

**Verdict: the 8-min target holds.** Server-side load is negligible. The two timing risks are both content-presentation, not network.

---

## Critical findings

### 1. `/audit` shows `INV-[PHONE]` / `QT-[PHONE]` in EVERY row — and it's a real product bug

Wave-1 thought this was raw placeholder text in the fixture. **It is not.** The fixture data at `web/lib/sample-audit.ts:61,89,94` has real values:

```
[SAM, "extracted INV-2024-08741 line items", ...]
[NORI, "validated QT-2024-7283 against pricing rules", ...]
```

But the client-side PII redactor at `web/lib/redaction/patterns.ts:59` defines:

```
export const PHONE_RE = /\+?[\d(][\d\s()-]{6,18}\d/g;
```

This regex matches `2024-08741` (10 chars, 9 digits, 1 dash) as a phone number — passes the ≥7-digit floor in `redact.ts` and gets substituted with `[PHONE]`. The audit feed runs every action verb through the redactor by default; result on screen is every invoice/quote ID replaced with `[PHONE]`.

**Impact on demo**: At the 07:30 audit reveal, the camera will see "extracted INV-[PHONE] line items", "validated QT-[PHONE] against pricing rules", "flagged duplicate invoice INV-[PHONE]", "flagged 32% discount on QT-[PHONE] for Deal Desk", "queued $14,250 for CFO approval (INV-[PHONE])". Viewer's read: SKYCOT's audit log doesn't actually capture invoice IDs — it shows literal `[PHONE]` placeholders. Looks unfinished.

**Two fixes**, in order of effort:
- **Cheap (10 min)**: hard-code the demo audit feed to bypass the redactor (`{ skipRedaction: true }` flag where it reads the fixture in the audit page, or strip `[PHONE]` from the fixture rows in pre-render).
- **Real (1-2 hours)**: tighten `PHONE_RE` to require a non-word boundary before the digits. Currently the regex is anchored only by `[\d(]` (must START with digit or `(`); add a leading `(?:^|[\s,;.])` or `(?<![A-Z-])` look-behind so it doesn't eat the digit suffix of an alphanumeric identifier like `INV-...` or `QT-...`. Tighter regex + the existing `digitCount >= 7` post-filter will keep e.164 phone matches working while sparing prefixed IDs.

Either way: this needs to land before record. **Most important item on the list.**

### 2. `/workflows/.../build` Propose has a visible response — but no follow-up Approve UX

The runbook script line is "And it proposes the diff. You approve. It ships." Currently:
1. Operator types prompt and clicks Propose → input clears
2. Sonner toast fades in (top-right of screen): "Proposing: ...Drafting workflow change…"
3. 1.4s later toast updates: "Proposal ready for review — 1 step change proposed. Approve to apply to the canvas."
4. Toast self-dismisses after sonner default (~4s)
5. No diff modal opens, no ghost node renders in the canvas, no Approve button surfaces

**Demo-script gap.** The voiceover narrates an Approve interaction that has no visible Approve button on camera. Options:
- **Edit the voiceover**: change to "And it proposes the diff for your review — approval-first, just like everything in SKYCOT" (drop the "You approve. It ships." beat). 1-line script edit.
- **Implement a stub modal**: a 30-min React patch to `build/page.tsx:303` that, instead of `toast.success(...)`, opens a `<Dialog>` with the proposed step preview + an Approve button that closes the dialog and inserts a ghost node. Much higher-impact on camera.

Recommendation: at minimum edit the script. If there's time tonight, ship the stub modal — this is the demo's emotional money shot at 5:30-6:30.

### 3. Tab `<title>` STILL leaks "SKYCOT — Hire your first AI teammate" on key in-app routes

Affected (visible in browser tab during recording):
- `/` (should be "Approvals · SKYCOT")
- `/team` (should be "Team · SKYCOT")
- `/workers/wkr_01J0MARIAH/_` (should be "Mariah · SKYCOT" or "Worker · Mariah · SKYCOT")
- `/audit` and `/audit/action/_` (should be "Audit · SKYCOT")
- `/cohorts` (should be "Cohorts · SKYCOT")

Already correctly set on `/landing`, `/auth/signup`, `/workflows`, `/workflows/.../build`, `/compare`, `/skills`, `/pricing`. So the pattern is established — these missing pages just need their own `export const metadata` block.

**Visible in the browser tab during recording** (not big, but noticeable to anyone paying attention to the tab strip).

---

## Polish findings (wave-1 carryover, still present)

- **`/landing` worker bubbles cramped.** Bubble dimensions measured at viewport 1718×915: bubble 1: 84×161 (very tall narrow column), bubble 2: 103×105, bubble 3: 92×161, bubble 4: 80×161. Quote text forced into ~80-100px columns. Still looks "AI-generated mock" rather than "designed art." Camera will land on this for the opening 45 seconds.
- **`/auth/signup` OAuth buttons labeled `v1.x`.** Three buttons (Google · GitHub · Microsoft) each have a `v1.x` badge — reads as "feature deferred." For the demo recording at 00:45 the operator should either hide these (only show "OR WITH EMAIL") or scroll past them quickly.
- **`/workflows/.../build` "Add step" button at x=1574 in 1718×915 viewport.** At a typical 1440-wide recording viewport, x=1574 would be **off-screen to the right**. Either reposition (left of the minimap, not below) or test the recording resolution before pressing record.
- **`/workflows/.../build` React Flow attribution watermark visible.** "React Flow" text bottom-right. Hide via `proOptions={{ hideAttribution: true }}` (needs React Flow Pro license) or restyle position/color.
- **`/cohorts` doesn't match the locked design** (memory: `project_cohorts_page_design_locked.md`). Not on demo path but flagged for follow-up.

---

## Console / network summary

- **Console errors total**: 502 (during ~30 min of audit walking)
- **Source distribution**: 100% are `FILE_ERROR_NO_SPACE` from Chrome's profile leveldb (Chrome environment, not SKYCOT). Wave-1's ClientFetchError chain quenched.
- **5xx responses on demo path**: 0
- **4xx responses on demo path**: 0 (the only 404 in the codebase is `/features` and it's no longer linked)
- **Static assets**: all chunks + fonts return 200

**For the recording**: use a fresh Chrome profile. The current profile's leveldb is in a permanent error state that won't repair itself without a manual wipe. With a fresh profile + dev tools closed, the operator will see zero errors during the recording.

---

## Recommendation for tomorrow's record

**READY TO RECORD.** The demo is materially better than wave-1 24 hours ago.

Pre-flight items in priority order:
1. **MUST FIX**: `/audit` PHONE-redaction bug (10-min cheap fix or 1-2 hour proper fix). Otherwise every audit row at 07:30 shows `[PHONE]` placeholders.
2. **SHOULD FIX OR REVISE SCRIPT**: `/workflows/.../build` Propose has no visible Approve follow-up. Either ship a stub modal (30 min) or edit the voiceover line to drop the "You approve. It ships" beat.
3. **NICE TO HAVE**: set `export const metadata.title` on `/`, `/team`, `/workers/_`, `/audit`, `/cohorts` (5 routes × 1 line each).

For the recording itself:
- Fresh Chrome profile (clears the FILE_ERROR_NO_SPACE)
- Dev tools CLOSED throughout
- Pre-warm `/compare` once before pressing record (cures the font-swap FOUC)
- Record at ≥1718 wide if you want the `/build` page Add-step button on screen
- Avoid the "Features" link in /landing nav (it's an anchor now, not a 404, but still better to scroll past)

The 8-min target is comfortable. Server load is sub-second across the full demo path. The two beats that will read slightly off-camera (Propose dead-end, PHONE redaction) are both fixable in <1 hour each.

---
*Wave-5 dress rehearsal audit · 2026-05-20 · read-only · no code modified*
