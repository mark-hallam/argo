# fix-first-hire-cta-copy-promises-weekly-schedule-not-parsed

**Owner**: claude.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-first-hire-cta-copy-promises-weekly-schedule-not-parsed`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/onboarding-hire-an-ai-teammate.md` F9.

🟡 **P1 — First-hire copy promises "Give them a name, a role, and what they should do this week." The placeholder reads "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans." But the wizard sends only `{description, vertical_id}` to the gateway — no schedule / duty / cron is parsed out of "weekly". The worker's Schedules tab shows the vertical pack's default duties, NOT what the operator just typed. The copy is making a promise the wiring doesn't keep.**

## Root cause

`web/app/(setup)/onboarding/first-hire/page.tsx:15-16` ships sub-copy + placeholder that promise schedule extraction:

> "Give them a **name, a role, and what they should do this week**."
> Example: "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans."

But `web/lib/onboarding/use-first-hire.ts` POSTs to `/api/v1/onboarding/first-hire` with body `{description: <full string>, vertical_id: <id>}` — no parse, no cron extraction, no LLM-side schedule detection.

The worker detail page that lands after hire shows the vertical pack's default duties (e.g. for `marketing-intern`: "Draft weekly digest" Mondays, "Run brand-mention scan" daily). If the operator typed "Maya runs weekly competitive scans", the resulting Schedules tab has the canned duties, not a "weekly competitive scan" duty. The promise → reality gap is camera-visible if Scene 3 narration reads the placeholder text aloud and the camera then pans to the Schedules tab.

Two clean fixes:
1. **Make the wiring meet the copy** — parse the description on the gateway side (or via Genesis) into a charter + 1-N schedule entries.
2. **Soften the copy to match the wiring** — drop "this week"; let scheduling happen on the worker detail page after hire.

Per memory `feedback_genesis_is_ai_teammate_onboarding_studio` (operator-clarified 2026-05-25): Genesis IS the studio that takes the role description and turns it into a real worker. So path (1) is the operator-locked direction; (2) is the short-term cut.

## Findings this brief closes

- onboarding-hire-an-ai-teammate F9 [P1] — Hire CTA copy promises "this week" but no schedule/duty is captured from the description

## Fix recipe

**Phase 1 — Soften the copy (short-term, 5-min fix).**

1. `web/app/(setup)/onboarding/first-hire/page.tsx:15-16`:
   - Sub-copy: "Give them a **name and a role**. You can set their schedule after they're hired."
   - Placeholder: "A marketing intern named Maya who drafts blog posts and asks before publishing."
   - Drop the "weekly competitive scans" example to keep the promise honest.

2. After the wizard, the worker detail page's Schedules card (per sibling brief `fix-worker-detail-responsibilities-schedules-tab-end-to-end.md`) becomes the explicit place to add cadence — drives the operator there with a soft callout ("Add their first schedule →").

**Phase 2 — Wire Genesis to parse the description (operator-locked direction).**

3. Per `feedback_genesis_is_ai_teammate_onboarding_studio`, the first-hire flow should invoke Genesis (the AI Teammate Onboarding Studio) on the gateway side. Tracked separately in `fix-onboarding-first-hire-invoke-genesis-plan.md` and `fix-onboarding-genesis-manifest-memory-eval-persistence.md`. This brief stops at Phase 1.

4. Once Phase 2 lands, restore the original "this week" copy + add a "Schedule extracted from your description: ..." confirmation step.

## Acceptance criteria

- [ ] First-hire step copy no longer promises schedule extraction in Phase 1.
- [ ] Placeholder example doesn't mention "weekly" / "daily" cadence.
- [ ] Post-hire CTA on the worker detail page surfaces "Add a schedule" prominently (deep-link into the Schedules card).
- [ ] Vitest pin: the first-hire page renders the softened sub-copy.

## May-touch

- `web/app/(setup)/onboarding/first-hire/page.tsx` (copy)
- `web/app/(app)/workers/[workerId]/overview/page.tsx` (post-hire callout — small)

## Out of scope

- Genesis schedule parsing (Phase 2; see sibling briefs above)
- Schedules card design itself (covered by `fix-worker-detail-responsibilities-schedules-tab-end-to-end.md`)
- Marketing landing-page copy that promises "tell us what they should do this week"

## Effort

XS — ~10-30 LOC copy + 1 vitest case. ~15-30 min.
