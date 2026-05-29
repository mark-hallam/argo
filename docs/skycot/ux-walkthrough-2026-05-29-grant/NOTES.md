# Full-site e2e walkthrough — post Grant + roster→7 (2026-05-29, later)

Live walkthrough against the **demo-live** stack (gateway :7400 fresh build
from `8e107faf4`, SPA :3000, 7 heroes seeded, swarm paused). Driven by
`web/walkthrough-grant.mjs` (Playwright, 1440×900) → screenshots + `_report.json`
in this directory. Plus an interactive `/gtm` submit test.

## Result: clean. 14/14 routes HTTP 200, every content probe passed.

| Route | Status | Console errs | Notes |
|---|---|---|---|
| /team | 200 | 0 | Renders all 7 incl. **Grant + Portia** ✅ |
| /approvals | 200 | 2 | Grant's draft present; errs = known WS auth-fail (×2) |
| **/gtm** (new) | 200 | 0 | Hero + form render clean; **submit→draft works live** (below) |
| /pitch | 200 | 0 | Portia rehearsal surface ✅ |
| /genesis | 200 | 0 | clean |
| /cohorts | 200 | 0 | **Growth** dept (Grant) + Sales & Investor Relations (Portia) present ✅ |
| /skills | 200 | 1 | known 404 sub-resource |
| /workflows | 200 | 0 | clean |
| /workflows/build | 200 | 0 | clean |
| /audit | 200 | 3 | known "Element type is invalid" hero error (page renders) |
| /settings | 200 | 0 | clean |
| /onboarding/welcome | 200 | 0 | clean |
| /workers/<grant>/overview | 200 | 1 | Grant profile renders; err = known 404 sub-resource |
| /workers/<portia>/overview | 200 | 1 | Portia profile renders; err = known 404 sub-resource |

## /gtm interactive e2e (the new hero surface for Grant)

Filled "Avery Stone" + a fintech-VP context → clicked **Draft follow-up**:
- **793-char real draft** rendered, context-aware ("…on-call triage at a
  120-person fintech is exactly the kind of standing responsibility…").
- Disposition badge: **Brand-safe** (Allow). Model: `claude-sonnet-4-6`.
- **"Filed to approvals" link present** → the draft was written to the queue.
- Full path proven: browser → SPA `/api/v1/gtm/draft-followup` rewrite →
  gateway → live LLM → truth-boundary → approval write → render.

## Findings

**No new breakage from the Grant + roster→7 changes.** All console errors are
the SAME four pre-existing issues already logged in the session handoff:

1. `/audit` — **FIXED (commit 04ca2a4a5).** This was NOT "renders OK" — the
   "Element type is invalid (Check render method of ChannelBadge)" error was
   caught by the route error boundary and replaced the WHOLE feed with a 500
   shell. Root cause: the live feed emits channel kinds `agent`/`mcp`/
   `computer-use` outside the SPA's ChannelKind union → `ICONS[kind]` undefined
   → `<undefined/>`. ChannelBadge now resolves a fallback (and proper icons for
   those 3 surfaces). Re-verified: /audit renders the feed, 50 channel badges
   across web/slack/agent/mcp/computer-use, 0 console errors.
2. `/approvals` — `ws://…/inbox/ws` WebSocket auth-fail (no creds). Data renders
   fine; only live-push is missing. **Pre-existing.**
3. `/skills` — one 404 sub-resource. Page renders. **Pre-existing.**
4. worker-detail (both Grant + Portia) — one 404 sub-resource (the
   coming-soon worker-trust / workflow-runs tab with no live endpoint).
   **Pre-existing.**

**Demo-state note:** the interactive `/gtm` test added a 2nd real Grant draft
(Avery) to `/approvals`, so the queue now shows Grant's two real drafts (Dana +
Avery) on top of the 8 seeded = 10 items, zero duplicates. Reads well for the
demo (Grant visibly producing). To return to exactly the curated 9, re-run the
fresh-DB reset sequence (see handoff doc).

## Post-fix re-run (same session) — clean baseline

After fixing `/audit` (commit 04ca2a4a5) and `/approvals` (commit ad913bf25),
the walkthrough was re-run. **12 of 14 routes are now 0-error**; both
demo-anchor pages are clean:

| Route | errs (before → after) |
|---|---|
| /audit | 3 + 500 shell → **0, feed renders** |
| /approvals | 2 (WS auth) → **0** |
| /skills | 1 → 1 (`GET /api/v1/skills/learned-this-month` 404 — gateway endpoint not implemented) |
| worker-grant / worker-portia | 1 each → 1 each (coming-soon tab 404, no live endpoint) |
| all 9 others | 0 → 0 |

## Verdict

Demo-ready, with Grant + Portia as the two real working teammates and both
Scene-6 anchor pages (`/audit`, `/approvals`) now error-free. The two
remaining console errors are single secondary-resource 404s
(`skills/learned-this-month`, a worker-detail coming-soon tab) that need a
gateway endpoint each (evaluator-lane Rust) and are camera-invisible with dev
tools closed. Not POC blockers.
