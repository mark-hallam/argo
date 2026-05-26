# fix-worker-tabs-placeholder-bodies-leak-brief-ids-e4

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-worker-tabs-placeholder-bodies-leak-brief-ids-e4`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-1-and-2-claim-1-and-2.md` F4.

🟦 **P2 — Three worker-detail tabs (Memory, Skills, Settings) render placeholder bodies that read "Memory — coming in E4-d.", "Skills — coming in E4-e.", "Settings — coming in E4-f." The brief-IDs E4-d / E4-e / E4-f are engineering tokens that should never reach a customer surface. If the camera lands on any of these tabs during exploratory framing (it can, the runbook doesn't lock the click order), the viewer reads a project-management code.**

## Root cause

`web/app/(app)/workers/[workerId]/[tab]/page.tsx:38-49` is the dynamic-tab fallback for any worker tab without a static sibling page. It renders a dashed-border panel with body text like:

```
Memory — coming in E4-d.
Skills — coming in E4-e.
Settings — coming in E4-f.
```

The tabs that DO have static implementations (`overview`, `activity`, `trust`, `escalation`) bypass this fallback. But Memory, Skills, Settings (and possibly others) fall through.

The brief-IDs `E4-d / E4-e / E4-f` come from the BUILD_PLAN_V11 phased-plan internal scheme. They're meaningful inside the swarm; meaningless and slightly alarming to a customer ("E4 of what? are we beta?").

Per memory `feedback_no_theater_in_demo` + the function-vs-dressing rule, "coming soon" placeholders are allowed for non-camera-drilled tabs, but the language must be brand-voiced — no internal codes, no engineering jargon. The current strings violate this in three places.

Note: `escalation/page.tsx` (444 LOC) DOES exist as a real implementation; the audit confirmed Trust is also implemented. So this brief is bounded to Memory + Skills + Settings (the three confirmed-leaky tabs).

## Findings this brief closes

- scene-1-and-2-claim-1-and-2 F4 [P2] — Three worker-detail tabs are explicit "coming in <brief>" placeholders the camera could land on

## Fix recipe

Two options. Pick (b) for the safest end-state.

**(a) Hide nav tabs whose body is still a placeholder.** Filter the `WorkerTabs` declarations in `web/components/skycot/WorkerTabs.tsx:19-39` so Memory / Skills / Settings are NOT visible until their static pages ship. Lightest touch but loses navigational signal.

**(b) Replace placeholder bodies with brand-voiced "coming soon" copy that doesn't leak brief-IDs.** Preferred — keeps the nav surface intact for design pacing.

1. Edit `web/app/(app)/workers/[workerId]/[tab]/page.tsx:38-49` to replace the brief-ID-keyed text with brand-voiced equivalents:
   - Memory: "Memory · {workerName} hasn't logged remembered context yet. As they handle tasks, what they learn shows up here."
   - Skills: "Skills · {workerName}'s skill library will appear here once they install or are taught one."
   - Settings: "Settings · Configure {workerName}'s permissions, notification routing, and escalation policy. Coming soon."
   - Strip every reference to "E4-" / "coming in <brief>" / engineering codes.

2. Wrap the "Coming soon" framing in a `<SoonShell>` component (or similar) at `web/components/skycot/worker/SoonShell.tsx` so future tab placeholders follow the same pattern. Component takes `{title, body, sketch?}` and renders the locked C2 register: dashed-border panel + warm gradient touch + a small "We're building this" eyebrow.

3. Vitest pin: `WorkerTabs` for an unwired tab id renders `<SoonShell>` (not a div whose text contains "E4-").

4. Grep guard in CI: `grep -rE 'coming in E[0-9]+-[a-z]' web/app web/components` → exit 1 if matches.

## Acceptance criteria

- [ ] Navigating to `/workers/<id>/memory`, `/workers/<id>/skills`, `/workers/<id>/settings` shows brand-voiced "coming soon" copy, NOT brief-ID strings.
- [ ] No string of the form `E[0-9]+-[a-z]` exists in any user-facing component under `web/app/(app)/**` or `web/components/skycot/**`.
- [ ] Vitest pin passes.
- [ ] CI grep guard added.

## May-touch

- `web/app/(app)/workers/[workerId]/[tab]/page.tsx` (the fallback)
- `web/components/skycot/worker/SoonShell.tsx` (NEW — small)
- `web/components/skycot/WorkerTabs.tsx` (no change unless option (a) is picked)
- Vitest test + CI grep guard

## Out of scope

- Building the real Memory / Skills / Settings tabs (separate briefs: E4-d, E4-e, E4-f in the build plan)
- Hide-vs-replace policy across the whole SPA (this brief is bounded to worker-detail tabs)
- Trust + Escalation tabs (both already implemented per the audit)

## Effort

XS — ~30-50 LOC. ~30-45 min.
