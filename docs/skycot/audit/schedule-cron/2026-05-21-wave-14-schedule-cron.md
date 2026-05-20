# Wave-14 Audit #3 — Schedule trigger + cron picker (twice-daily UX)

Date: 2026-05-21
Auditor: claude (wave-14 schedule-cron audit)
Scope: cron picker readiness for the twice-a-day teammate use case
Source: `web/components/workflows/triggers/` + PR #1890 + workflow builder page
Operator context: wants to set up a teammate that runs a workflow **twice a day**.

---

## TL;DR

The cron picker shipped in PR #1890 (`web-workflow-trigger-config-ux`) is a
solid v1 frequency picker for ordinary single-firing-per-period schedules
(every hour / every day / specific days of the week / specific day of
month / custom cron). **It does not have a first-class "twice a day"
mode.** The operator's exact use case requires falling through to the
**Custom cron** option (`0 9,17 * * *`) which loses the friendly summary
and Next-5-Runs preview ("preview unavailable for custom cron").

Persistence is **not** wired — `triggerConfigs` lives in `useState` on the
builder page and is never sent to the gateway. On reload, the schedule
disappears. The workflow catalog card does not surface the schedule
either; cards show steps / versions / lastEdited but never "Runs 9am +
5pm AEST".

Twice-a-day is achievable today only by power users who know cron
syntax. The friendly-pill UX path stops one mode short of the operator's
ask.

---

## File map

| Surface                          | Path                                                                                | LOC  |
|----------------------------------|-------------------------------------------------------------------------------------|------|
| Cron picker                      | `web/components/workflows/triggers/ScheduleConfigForm.tsx`                          | 364  |
| Trigger drawer (dispatcher)      | `web/components/workflows/triggers/TriggerConfigDrawer.tsx`                         | 227  |
| Schedule types + zod schema      | `web/components/workflows/triggers/types.ts`                                        | 190  |
| Builder consumer + drawer wiring | `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`                        | 1119 |
| Test coverage                    | `web/__tests__/workflows/trigger-config-forms.test.tsx`                             | 317  |
| Workflow catalog (cards)         | `web/app/(app)/workflows/page.tsx`                                                  | 603  |

---

## 1. Cron frequency mode matrix

`SCHEDULE_MODES` defined at `types.ts:51`:

```ts
export const SCHEDULE_MODES = ["minute", "hour", "day", "week", "month", "custom"] as const;
```

| Operator intent       | Mode               | Cron produced       | First-class?       | Notes |
|-----------------------|--------------------|---------------------|--------------------|-------|
| Every minute          | `minute`           | `* * * * *`         | YES                | Useful for dev/test |
| Hourly                | `hour`             | `M * * * *`         | YES                | Pick the minute |
| **Twice a day**       | **(none)**         | **n/a**             | **NO — fall through to `custom`** | Operator must hand-write `0 9,17 * * *` |
| Daily                 | `day`              | `M H * * *`         | YES                | Pick hour + minute |
| Specific weekdays     | `week`             | `M H * * D1,D2,...` | YES                | Mon-Fri pill row |
| Monthly (day-of-month)| `month`            | `M H D * *`         | YES                | Pick day 1-31 |
| Custom cron           | `custom`           | (user-typed)        | YES (5-field validate only) | Loses summary + next-5-runs preview |

**Gap A1 — no "Twice a day" mode.** The `mode === "day"` branch (line 91-122
of ScheduleConfigForm) shows a single Hour + Minute field pair. There is
no UI affordance for picking two (or N) fire times per day.

**Gap A2 — no compound "morning + afternoon" preset.** Tools like n8n,
Make, Zapier ship presets ("Every 12 hours", "Twice daily", "Three times
daily") that map to `0 9,17 * * *` or `0 9,13,17 * * *`. SKYCOT has none.

---

## 2. Cron-preview ("next 5 fire times")

`nextNRuns()` at `ScheduleConfigForm.tsx:280-347`.

- For `minute / hour / day / week / month` modes: returns 5 accurate ISO
  timestamps. **Always rendered in UTC** (`.toISOString()`), regardless
  of the user's chosen timezone. So a Melbourne operator picking 9am
  Melbourne sees the preview list as `22:00Z` (the night before, UTC).
  This is a UX trip-hazard — the summary line says "in Australia/Melbourne"
  but the preview list reads UTC.
- For `custom` mode: returns the literal string
  `"preview unavailable for custom cron"` five times (line 343-345).
  **The operator's twice-daily path through `custom` gets NO preview at
  all.**

**Gap B1 — preview ignores the user's timezone.** All next-5-runs are
rendered as UTC ISO strings even when timezone is e.g. `Australia/Melbourne`.

**Gap B2 — DST handling is silently incorrect.** The naïve
`d.setDate(d.getDate() + 1)` / `d.setMonth(d.getMonth() + 1)` loop walks
the **local-time JS Date** of the operator's browser, not the configured
schedule timezone. On a DST spring-forward day (e.g. Melbourne moves
clocks forward on a Sunday in early October), a 9am Melbourne schedule
will silently shift to 8am or 10am in the preview depending on which
timezone the browser is set to. There is no DST-aware cron evaluator in
the client.

**Gap B3 — custom cron is the only path to twice-daily AND it gets no
preview.** Compound failure: the one frequency the operator needs
(twice-daily) requires the one mode that has zero next-runs preview.

The codebase calls this out honestly at line 277-279:
`// A full cron evaluator lives on the gateway side; this is just UX confidence.`
But the gateway side does not yet exist for workflow triggers (see §4
persistence below).

---

## 3. Timezone selector

`ScheduleConfigForm.tsx:38-52` hardcodes 13 IANA zones:

```ts
const COMMON_TIMEZONES = [
  "UTC",
  "Australia/Melbourne",
  "Australia/Sydney",
  "Australia/Brisbane",
  "Australia/Perth",
  "America/New_York",
  "America/Los_Angeles",
  "America/Chicago",
  "Europe/London",
  "Europe/Berlin",
  "Asia/Singapore",
  "Asia/Tokyo",
  "Asia/Kolkata",
];
```

- `Australia/Melbourne` **is present** as the second entry — good for the
  operator's use case.
- The brief acknowledges this is a stub: `// Full IANA list is fetched
  from the workspace settings in a follow-up brief once that endpoint
  exists.`
- The drawer's parent passes `workspaceTimezone="Australia/Melbourne"`
  hard-coded at builder page line 1105 — so the default for ANY tenant
  loading the builder is Melbourne, not the workspace's actual tz. (A
  separate audit-able defaulting bug.)
- No user-locale auto-detection — the picker does not call
  `Intl.DateTimeFormat().resolvedOptions().timeZone` to pre-select the
  user's browser timezone.

**Gap C1 — workspace timezone is hard-coded to Australia/Melbourne in
the builder.** Should read from session / workspace settings.

**Gap C2 — no auto-detect of operator's browser timezone.** A US-based
admin onboarding their Indian team has to scroll through the 13 zones
manually.

**Gap C3 — only 13 zones. The full IANA list is ~600 zones.** If a
tenant operates from, say, `Africa/Lagos` or `Pacific/Auckland`, they
hit `(no options)` and must fall back to `UTC` + cron math.

---

## 4. Persistence — does the schedule survive a reload?

**No.** Three independent breakage points:

1. **Builder page state-only** — at line 620 of build/page.tsx:
   ```ts
   const [triggerConfigs, setTriggerConfigs] = useState<Record<string, TriggerConfig>>({});
   ```
   Click "Save" in the drawer (line 154 of TriggerConfigDrawer) calls
   `onSave()` which simply closes the drawer. There is no `fetch()` to
   any gateway endpoint, no localStorage write, no Server Action.

2. **No backend types for schedule trigger config.** Searched
   `crates/argo-workflow/src/*.rs` and `crates/argo-cron/src/*.rs` —
   neither contains a `ScheduleTrigger`, `TriggerConfig`, or
   `schedule_config` definition keyed to workflows. `argo-cron` exists
   but services skill regression cron + per-worker learning cron, NOT
   workflow triggers.

3. **The "Save + publish" button stubs out** — lines 708-714 of
   build/page.tsx:
   ```ts
   function handleSaveAndPublish() {
     // Honest demo stub — wave-6 audit
     toast.message("Saved as draft — preview build", {
       description: "Tenant-wide publish lands in an upcoming release.",
     });
   }
   ```

**Verdict: zero persistence. Refresh the page → the schedule the
operator just set up is gone.** (This is honestly stubbed; not a buyer-
catchable lie. But it does mean the operator's "twice a day" end-state
cannot actually be achieved on main.)

---

## 5. Workflow catalog — does the schedule show up on the card?

`web/app/(app)/workflows/page.tsx` is the `/workflows` index. Cards
render:

- vertical tag (top-left)
- risk badge (top-right): "Approval required" / "Autonomous"
- workflow name (`wf.name`)
- description
- step count + gradient dots
- versions / lastEdited / lastEditor

**There is no `schedule` field on `SEED_WORKFLOWS` (lines 22-103), no
"Runs 9am + 5pm AEST"-style label, and no cron-summary surface.** Even
if persistence existed, the catalog would not show the operator that
this workflow runs twice a day.

The closest existing affordance is the `lastEdited` timestamp; nothing
indicates "next run at ..." or "scheduled twice daily".

---

## 6. User journey walk — operator wants `sales/qualify_lead` to run twice a day

The brief asked me to walk this specific journey. There's one extra
trip-hazard worth flagging up front:

1. **`/workflows/sales/qualify_lead/build`** — this workflow's seed
   trigger is `{ id: "s1", kind: "trigger", triggerKind: "webhook", ... }`
   (build/page.tsx line 143).
2. Clicking the trigger node opens the side panel with "Configure
   trigger" button.
3. Clicking "Configure trigger" opens **`TriggerConfigDrawer` with
   triggerKind=webhook**, NOT schedule.
4. **The operator's described journey is impossible on `sales/qualify_lead`
   as seeded.** To get a schedule trigger they would have to:
   - either change `triggerKind` from `webhook` to `schedule` (no UI
     affordance — `triggerKind` is set in the seed data, not editable);
   - or pick a different workflow where the seed trigger kind is
     `schedule` (none of the three seeded workflows in build/page.tsx
     uses schedule — `gtm-engineer/post_listing` uses `form`,
     `sales/qualify_lead` uses `webhook`, `support/qa_audit` has a
     trigger step with name "Nightly cron at 02:00 UTC" but **no
     `triggerKind` property at all**, so it falls through to the
     default "manual" — which means the drawer shows the "no extra
     config needed" message).

The closest path to actually exercising the schedule picker today:

- Land on `/workflows/<v>/<n>/build`
- The trigger node config UI doesn't expose a "change trigger kind"
  dropdown anywhere; the operator can't switch from webhook → schedule.
- The schedule picker is reachable only when the workflow's seed already
  declares `triggerKind: "schedule"`. **Zero seeded workflows do.**

If you bypass the seed (e.g. via the workflow-templates route at
`/workflows/templates/[templateId]`), most templates likely also default
to other trigger kinds; would need a template walk to confirm.

5. **Picking "Twice a day"** — as documented in §1, this is not an
   available pill. Operator must click the **Custom cron** pill and hand-
   type `0 9,17 * * *`. Validation accepts it (5 fields, syntax OK).
   Summary line reads: *"Runs on cron `0 9,17 * * *` in Australia/Melbourne."*
   (Honest but unhelpful — does not parse 9 and 17 into "9am and 5pm".)
   Next-5-Runs reads: *"preview unavailable for custom cron"* × 5.

6. **Picking timezone "Australia/Melbourne"** — works, it's in the
   dropdown.

7. **Save** — closes the drawer. Refresh: `triggerConfigs` is reset to
   `{}`. The schedule does not persist. Catalog card at `/workflows`
   does not show the schedule. Workflow card displays only steps /
   versions / lastEdited.

---

## 7. Twice-a-day verdict summary table

| Question                                                | Answer |
|---------------------------------------------------------|--------|
| Hourly supported?                                       | YES (first-class pill) |
| **Twice-a-day supported?**                              | **PARTIAL — only via custom cron, loses preview + plain-English summary** |
| Daily supported?                                        | YES (first-class pill) |
| Weekly supported?                                       | YES (day-of-week pill row) |
| Monthly supported?                                      | YES (day-of-month input) |
| Custom cron supported?                                  | YES (5-field syntax check, no semantic parse) |
| Next-5-runs preview for predefined modes?               | YES (but rendered in UTC, ignores selected tz) |
| Next-5-runs preview for custom cron?                    | NO ("preview unavailable") |
| Timezone selector?                                      | YES (13 hardcoded zones, AU/Melbourne present) |
| User locale auto-detect?                                | NO |
| DST-aware preview?                                      | NO (naïve JS Date math; will mis-render on DST transitions) |
| Schedule persists to gateway on save?                   | NO (useState only) |
| Schedule visible on /workflows catalog card?            | NO |
| Operator can switch a non-schedule trigger to schedule? | NO (no triggerKind picker UI) |

---

## 8. Top 3 schedule UX gaps (operator-impact ordered)

### Gap 1 — "Twice a day" / N-times-a-day is not a first-class mode

**Operator impact: high.** The exact use case the operator named ("set
up a teammate to run their workflow twice a day") requires falling
through to Custom cron, which loses the friendly summary and the
next-5-runs preview. A real product would ship a `times` mode with
a row of hour pickers like "9am, 5pm" or a preset row ("Twice daily ·
9am+5pm", "Three times daily · 9am+1pm+5pm", "Every 12 hours").

**Fix sketch:** add `times` to `SCHEDULE_MODES`; UI is N "Hour:Minute"
rows with "+ add another time" button. `buildCron` produces
`MM HH1,HH2,... * * *`. Preview engine extends naturally. Estimate:
50-80 LOC in ScheduleConfigForm.tsx + zod schema + types + tests.

### Gap 2 — Schedule does not persist (gateway/backend missing)

**Operator impact: catastrophic.** The drawer accepts input, validates,
shows summary, looks polished — and silently discards everything on
reload. The operator believes they set up twice-daily; nothing actually
runs. This is the worst kind of demo-trap: looks complete, isn't.

**Fix sketch:** add `TriggerConfig` (with `ScheduleConfig` variant) to
`crates/argo-workflow` or a new `crates/argo-trigger`; wire a gateway
POST/PUT route under `/api/v1/verticals/<v>/workflows/<n>/trigger`;
flip the builder page from `useState` to React Query + fetch. Pair
with `argo-cron` (or a new workflow-cron crate) for actual firing.
Estimate: substantial — 300-500 LOC across 2-3 crates, plus migration
for `workflow_triggers` table.

### Gap 3 — Preview ignores timezone + DST, custom cron has no preview

**Operator impact: medium-high — silent correctness bug.** The summary
line says "in Australia/Melbourne" but the next-5-runs list is rendered
as UTC ISO strings. On DST transition days the preview is silently
wrong. Custom cron — the only path to twice-daily — has no preview at
all, so the operator has zero confidence the cron string they typed
means what they think it means.

**Fix sketch:** (a) drop a lightweight cron parser like `cron-parser`
or write one ~100 LOC (predefined modes already work; just need custom
mode + tz-aware Date math); (b) for predefined modes, render the
preview list in the configured timezone using
`Intl.DateTimeFormat(tz, ...).format(d)` rather than `.toISOString()`;
(c) explicitly highlight the DST-transition fire in the preview row
("9:00 AEDT" vs "9:00 AEST"). Estimate: 80-120 LOC.

---

## 9. Out-of-scope-but-worth-noting

- **No "change trigger kind" picker.** Once a workflow's seed declares
  `triggerKind: "webhook"`, there is no UI to switch it to `schedule`.
  An operator who builds a workflow assuming manual run, then later
  decides they want it on a daily cron, cannot do this.
- **Hard-coded `workspaceTimezone="Australia/Melbourne"`** at
  build/page.tsx:1105. Every tenant gets Melbourne as their default
  regardless of where they signed up.
- **No "pause schedule" or "skip next run"** affordance once a schedule
  is set up. Standard enterprise feature missing.
- **`support/qa_audit` workflow** has a step labelled "Nightly cron at
  02:00 UTC" but its `triggerKind` is unset (so it falls through to
  `manual`). The label is aspirational — there's no schedule attached.
  This is a demo-time landmine if anyone reads the label and asks
  "how do I change the cron?" Answer: you can't.
- **PR #1890** ships the picker UI cleanly with good test coverage
  (lines 99-153 of trigger-config-forms.test.tsx), zod validation, and
  honest "preview unavailable for custom cron" copy. The work that
  landed is solid; the gaps are scope-deferred, not bugs.
