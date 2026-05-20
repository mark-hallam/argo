# Wave-13 Audit #3 — Side-Panel Node-Config

**Date:** 2026-05-21
**Auditor:** Wave-13 audit agent #3
**Scope:** the side panel that opens when a node is clicked in
`/workflows/<vertical>/<name>/build`
**Subject file:** `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`
**Trigger-drawer family:** `web/components/workflows/triggers/`

---

## 1. Methodology

Read the full `build/page.tsx` (1121 LOC), then every file in
`web/components/workflows/`. The `workflows/` component directory
contains exactly **one** subfolder: `triggers/`. That single fact is
the headline of the audit.

For each step kind I evaluated:

1. Does the side panel render anything kind-specific beyond
   `name + description + inputs + outputs`?
2. Are the rendered fields editable?
3. Does editing persist (in-memory? gateway?)?
4. Is there validation?
5. Is the `Edit step` button wired to a tailored editor?

---

## 2. Side-panel anatomy

The panel (lines 1027-1102 of `page.tsx`) is the **same DOM for every
step kind**. It shows:

| Section | Source | Editable? |
| --- | --- | --- |
| Eyebrow tag (kind label) | `STEP_KIND_META[kind].label` | No (derived) |
| Title (`<h2>`) | `step.name` | **No — static `<h2>`, not an input** |
| Description (`<p>`) | `step.description` | **No — static `<p>`** |
| INPUTS code block | `step.inputs` | No (read-only `<code>`) |
| OUTPUTS code block | `step.outputs` | No (read-only `<code>`) |
| APPROVAL GATE banner | `step.approvalGate.channel + .reason` | No (read-only) |
| Edit-step button | always rendered | Click handler exists |

The button label flips between `"Configure trigger"` (trigger steps)
and `"Edit step"` (every other kind). The `onClick` handler is the
**same line** for both:

```tsx
onClick={() => setTriggerConfigOpen(true)}
```

But the drawer below only renders when:

```tsx
{selectedIsTrigger && triggerConfigOpen && selectedStep ? (
  <TriggerConfigDrawer ... />
) : null}
```

**Consequence: for skill / approval / tool_call / notify / decision /
data / loop / wait, clicking "Edit step" does literally nothing
visible.** No drawer opens, no toast fires, no inline form appears.

---

## 3. Per-kind score (1-10)

Scoring rubric: 1 = name/description display only (no config UX),
10 = fully tailored config, editable, validated, persistence wired.

| Kind | Score | Notes |
| --- | ---: | --- |
| **trigger** | **6** | Only kind with a tailored drawer. `TriggerConfigDrawer` dispatches to 4 forms (webhook / schedule / email / form) + 2 no-config kinds (manual / slack). Webhook + Schedule + Form forms are rich (zod validation, copy/rotate/test-ping, cron preview, drag-reorder fields). Persistence is in-memory only (`useState<Record<string, TriggerConfig>>` at line 620) — gateway PUT is explicitly TODO. -4 for no persistence; would be 9 if wired. |
| **skill_invoke** | **2** | Panel shows the same name/description/inputs/outputs strip every kind gets. **No skill-catalog picker.** **No skill-version selector.** **No input-mapping editor.** **No retry/timeout config.** "Edit step" button is dead (clicks nothing). +1 because INPUTS/OUTPUTS at least surface schema hints. |
| **approval** | **3** | Panel renders the `APPROVAL GATE` banner (channel + reason) read-only — slightly more than the baseline. **Channel is NOT editable** (no dropdown/segmented control). **Reason is NOT editable** (no textarea). **No SLA / escalation / approver-role config.** "Edit step" button is dead. +1 over baseline only because the gate banner exists. |
| **tool_call** | **2** | Same baseline panel. **No tool picker.** **No argument/parameter editor.** **No connector/credential selector.** **No timeout or retry.** "Edit step" dead. Tool-calls are second-class to triggers despite being equally important for v1. |
| **notify** | **2** | Same baseline panel. **No channel picker** (Slack vs email vs SMS). **No message template editor.** **No recipient field.** "Edit step" dead. Notify steps are where buyers most expect "edit the message" — currently impossible in UI. |
| **decision** | **1** | Worst-scored kind. **No condition editor visible.** **No branch list.** **No expression builder.** **No "Branch A / Branch B" YAML or DSL surface.** Buyer cannot see, let alone edit, the condition that controls flow. "Edit step" dead. The `description` field is the only signal of what the decision does — and it's not editable. |
| **data** | **2** | Same baseline panel. **No schema editor.** **No source/destination field-mapping UI.** **No transform expression.** "Edit step" dead. Data nodes appear in the kind enum + palette but are unwired beyond display. |

Bonus kinds beyond the brief's 7:

| Kind | Score | Notes |
| --- | ---: | --- |
| **loop** | **2** | Panel shows the same baseline. `loopConfig` (`items_path`, `item_var_name`, `max_iterations`) is rendered visually on the **node body** (line 281-298), NOT in the side panel — and is not editable anywhere. The side panel doesn't even surface the loop's config. |
| **wait** | **2** | Same: `waitConfig` (`duration_seconds` / `until_iso` / `until_field`) is summarized on the node body via `formatWaitSummary()`. Panel shows nothing wait-specific. Not editable. |

**Average score across the 7 kinds in the brief: 2.6 / 10.**

---

## 4. TriggerConfigDrawer per-trigger-kind audit

The trigger drawer (#1890) is genuinely the only piece of working
config UX in the builder. Per-kind breakdown:

| Trigger kind | Form file | Working fields | Validation | Persistence |
| --- | --- | --- | --- | --- |
| **manual** | n/a (NoExtraConfig fallback) | Info message only | n/a | n/a |
| **schedule** | `ScheduleConfigForm.tsx` | mode (6 pills), hour, minute, days-of-week, day-of-month, timezone, custom cron, **next-5-runs preview + plain-English summary** | zod (`scheduleConfigSchema`) | In-memory; cron-engine on gateway is authoritative |
| **webhook** | `WebhookConfigForm.tsx` | URL (read-only), copy, **rotate secret with confirm step**, **test-ping with HTTP response surface**, cURL snippet, methods (5 pills), auth (none/header) + header name/value | zod with superRefine | In-memory; comment confirms gateway PUT not wired |
| **email** | `EmailConfigForm.tsx` | IMAP host, port, username, password, TLS toggle, filter from/subject/has-attachment, test-connection button (UI-only — endpoint not wired) | zod | In-memory; password field has TODO for secret-ref |
| **form** | `FormConfigForm.tsx` | Public URL + copy, field list (add/remove/reorder), per-field label/type/required/placeholder, visibility, submit label, success message | zod | In-memory |
| **slack** | n/a (NoExtraConfig fallback) | Info message routing operator to Settings → Integrations | n/a | n/a |

**Trigger drawer verdict:** functionally excellent UX (zod validation,
plain-English summaries, test-ping affordances). The gap is purely
gateway persistence — even a Save click only closes the drawer
(`onSave={() => setTriggerConfigOpen(false)}` at line 1113). The
`triggerConfigs` state map IS keyed by step ID so it survives drawer
close+reopen within a session, but a page reload wipes everything.

---

## 5. Top 3 missing fields (buyer-catchable in any sales demo)

1. **Skill-catalog picker for `skill_invoke`.** A buyer clicks "Validate
   inventory" expecting to see "skill: validate-inventory@v3 — picked
   from catalog, can swap to v4". Instead they see a static `<h2>`
   with the step name and a button that does nothing. This is the
   single biggest credibility hit for the workflow builder.

2. **Condition editor for `decision`.** A `decision` node says "Branch
   on BANT score" with description "≥75 = demo. 50-74 = nurture. <50
   = decline." Where do those numbers live? They're a free-form
   string in `description` with no structured backing. The buyer
   cannot edit the thresholds, cannot see the branches as actual
   edges out of the node, and cannot test which branch fires.

3. **Approval channel + reason editability for `approval`.** The
   approval-gate banner shows `Channel: Slack` and `Posting public
   listings needs seller signoff.` as read-only text. Both are
   first-citizens of the workflow contract and must be editable
   from the panel. Buyers will ask "can I switch this to email
   approval?" in the first 60 seconds of a demo.

**Honorable mentions:** message template for `notify`, tool argument
editor for `tool_call`, the step's own `name` and `description` (NOT
editable on the side panel for ANY kind — the user cannot rename a
step from this surface).

---

## 6. Verdict — "is the side panel usable as a real product?"

**No.** The side panel is a read-only summary card with one working
sub-drawer (triggers).

Concretely:

- **For 1 of 9 step kinds (trigger), the side panel makes the node
  meaningfully editable** — and even that flow doesn't persist to
  the gateway.
- **For 8 of 9 step kinds, the "Edit step" button is dead.**
  It calls `setTriggerConfigOpen(true)`, but the drawer's render
  guard (`selectedIsTrigger && triggerConfigOpen`) immediately
  rejects it. There's no toast, no fallback drawer, no inline form.
- **Step name and step description are not editable from the panel
  for any kind**, including triggers. Renaming a step requires
  hand-editing TOML.
- **No persistence path exists yet.** All save/publish/test buttons
  are honest demo stubs (good — wave-6 action-fires audit forced
  honesty), but it means even the trigger drawer's "Save" is purely
  cosmetic.

The builder canvas is class-leading (drag, pan, mini-map, brand-loud
nodes). The trigger config drawer family is class-leading. **The 6
non-trigger node kinds have zero config UX.** A buyer demo that picks
ANY step other than a trigger will reveal the gap in one click.

**Recommended priority order to close the gap:**

1. P0 — wire `decision.condition` editor (the most embarrassing
   missing surface; lives on the workflow's correctness path)
2. P0 — wire `approval` channel + reason + approver-role editor
   (read-only banner already exists; just needs to become inputs)
3. P0 — make step `name` + `description` editable from the panel
   for every kind (one-line change: swap `<h2>`/`<p>` for `<input>`
   and pipe via the existing onChange pattern)
4. P1 — `skill_invoke` skill-catalog picker + version selector
5. P1 — `notify` channel + message template editor
6. P1 — `tool_call` tool picker + arg editor (canonical MCP tool
   list already lives in the gateway; this is a UI brief)
7. P2 — `data` schema/mapping editor
8. P2 — `loop` and `wait` config editors (config already renders on
   the node body, just needs to mirror into the panel)
9. P0 cross-cutting — gateway PUT wiring so any of the above can
   persist past page reload

---

## 7. Score summary

| Kind | Score |
| --- | ---: |
| trigger | 6 |
| skill_invoke | 2 |
| approval | 3 |
| tool_call | 2 |
| notify | 2 |
| decision | **1** |
| data | 2 |
| loop (bonus) | 2 |
| wait (bonus) | 2 |

**Brief-7-kind average: 2.6 / 10. Brief-7-kind median: 2.**
