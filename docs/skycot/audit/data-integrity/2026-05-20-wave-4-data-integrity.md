# Wave-4 Audit: SKYCOT Seed Data Integrity

Date: 2026-05-20
Auditor: Wave-4 audit agent #5 (seed data integrity)
Scope: every `web/lib/sample-*.ts` file + `web/lib/workflow-templates.ts` + `web/app/(app)/workflows/page.tsx` SEED_WORKFLOWS + the demo runbook claims at `docs/skycot/operator/demo-runbook.md`.

Read-only audit. No code modified.

---

## Inventory — entities audited

| Entity         | File                              | Count |
|----------------|-----------------------------------|------:|
| Workers (team) | `lib/sample-team.ts`              |     6 |
| Workers (detail) | `lib/sample-workers-detail.ts`  |     6 |
| Worker overviews | `lib/sample-worker-overview.ts` |     6 |
| Worker activity | `lib/sample-worker-activity.ts`  |     5 (Rio not seeded) |
| Worker memory  | `lib/sample-worker-memory.ts`     |     6 |
| Worker skills  | `lib/sample-worker-skills.ts`     |     6 |
| Worker trust   | `lib/sample-worker-trust.ts`      |     5 (Rio not seeded) |
| Audit rows     | `lib/sample-audit.ts`             |    51 |
| Audit details (hand-crafted) | `lib/sample-audit-detail.ts` |  5 |
| Approvals      | `lib/sample-approvals.ts`         |     7 |
| Notifications  | `lib/sample-notifications.ts`     |     9 |
| Cohort summaries | `lib/sample-cohorts.ts`         |     5 |
| Cohort detail (support::trusted) | same           |     6 members + 8 skills |
| Replay outcomes | `lib/sample-replay.ts`           |     8 (5 worker + 3 sentinel) |
| Workflow templates | `lib/workflow-templates.ts`   |    50 |
| Workflows on /workflows index | `app/(app)/workflows/page.tsx` | 8 |
| **Total** |                                          | **~205** |

Worker roster (canonical): Mariah, Priya, Jamal, Nori, Sam, Rio.

---

## 1. Duplicate IDs / names — RESULT: **clean**

- Worker IDs (`wkr_01J0...`): 6 unique, no duplicates across team / detail / overview / skills / memory / trust.
- Audit row IDs (`act_01J0<PERSONA>_NNN`): 51 unique (built by indexing the seed array; no collisions possible by construction).
- Workflow template IDs in `WORKFLOW_TEMPLATES`: 50 unique IDs, 50 unique names.
- Workflow `/workflows` index `SEED_WORKFLOWS`: 8 unique names.
- Approval IDs (`ap_01H9Z*`): 7 unique.
- Notification IDs (`nt_01H9N*`): 9 unique.
- Cohort IDs: 5 unique.

No duplicate-id or duplicate-name bugs. The `approve_refund` duplicate that wave-1 caught is fixed.

---

## 2. Broken cross-references — RESULT: **3 severe, 4 cosmetic**

### CR-1. Audit feed IDs do NOT match hand-crafted audit-detail IDs (SEVERE — demo-blocker)

`sample-audit.ts` builds row IDs as:

```js
id: `act_${worker.id.replace("wkr_", "")}_${i.toString().padStart(3, "0")}`
// → "act_01J0PRIYA_001" (uppercase ULID-style)
```

`sample-audit-detail.ts` hand-crafts 5 rich detail records with IDs:

```js
id: "act_priya_001"   // lowercase persona
id: "act_sam_001"
id: "act_mariah_001"
id: "act_jamal_001"
id: "act_nori_001"
```

`findSampleAuditDetail(actionId)` looks the actionId up in the hand-crafted list first. Because the IDs don't match, **the demo always falls through to `synthesizeAuditDetail`** — generic decision chain, generic prompt template, generic policy ("policepolicepolicy" hash). The CEO never sees the rich pre-built provenance / decision chain / multi-skill plan that was specifically authored for the demo path.

`sample-replay.ts` has the same lowercase IDs (`act_priya_001` etc.) so the **Replay** drill-in from the audit feed also misses the hand-crafted outcomes (reproducible / drifted-aligned / drifted-misaligned / model-decommissioned / leak-detected) and falls through to `mockOutcomeForActionId`, which only knows three deterministic verdicts.

### CR-2. Worker-overview activity hrefs point to nonexistent audit IDs (SEVERE — demo-blocker)

`sample-worker-overview.ts` builds activity rows with IDs like `ap_priya_1` and sets `href: /audit/action/ap_priya_1`. These IDs don't exist in `SAMPLE_AUDIT_ROWS` or `SAMPLE_AUDIT_DETAILS`. Clicking any recent-activity row on a worker overview page hits `findSampleAuditDetail("ap_priya_1")` → undefined → 404.

Same problem in `sample-worker-memory.ts`, which uses `href: /audit/action/ap_priya_1` for active-conversations linking.

The runbook at 02:30 explicitly tells the CEO to click into Mariah's overview. If they click any recent-activity row, the page 404s.

### CR-3. Cohorts references `wkr_01J0MAYA` — worker does not exist (SEVERE)

`sample-cohorts.ts` `SAMPLE_COHORT_DETAIL.members[0]` = `wkr_01J0MAYA` with `trust_grade: 9`. There is no Maya in:

- `sample-team.ts`
- `sample-workers-detail.ts`
- `sample-worker-overview.ts`
- `sample-worker-skills.ts`
- `sample-worker-memory.ts`
- `sample-worker-trust.ts`

If the cohort detail page (when shipped or stubbed) links the worker_id to /workers/<id>, that link 404s. Maya appears in unrelated places (onboarding suggestion copy, toast test fixture, orchestration mock array) but no canonical worker entry.

### CR-4. Mariah is missing from `support::trusted` cohort (cosmetic)

`SAMPLE_COHORT_DETAIL` (cohort_id `support::trusted`) members list = Maya, Priya, Jamal, Rio, Nori, Sam. **Mariah is absent**. The team page lists Mariah as the first worker, so when the operator narrows by cohort they won't find her. Probably a copy-paste oversight where Maya was meant to be Mariah.

### CR-5. Cohort vertical labels don't match `Worker.role.vertical` (cosmetic)

| Worker | sample-team vertical | Cohort puts them in (support::trusted) |
|--------|----------------------|----------------------------------------|
| Priya  | recruiting           | support |
| Jamal  | support              | support ✓ |
| Sam    | ops                  | support |
| Nori   | marketing            | support |
| Rio    | real-estate          | support |

The cohort frames every worker as a support tier-1 agent which contradicts the team page. The runbook tells the CEO at 02:00: "Six teammates, 1,122 actions this week, saving you 72 hours" — but if the CEO navigates to /cohorts they see Nori-the-marketing-lead listed as a support agent.

Cohort summaries also don't add up: 6+4+3+2+1=16 member-slots across 5 cohorts vs. 6 actual workers. Cohorts allow overlap, so this can be defended — but the seed nowhere documents which worker is in which secondary cohort.

### CR-6. Approver Marta is named in 5 worker overviews and 0 audit rows (cosmetic)

`sample-worker-overview.ts` has Priya's screenings, Jamal's billing escalation, Sam's incidents "approved by Marta". `sample-audit.ts` knows only three approver people: Mark (operator), Jenna (CFO), Aria (compliance). When the CEO drills from a worker overview into the matching audit row, Marta's name disappears — the audit log says "approved by Mark" instead.

### CR-7. Hiring manager "Marta" in conversation list — no other context (cosmetic)

Priya's `sample-worker-memory.ts` has an active conversation with "Marta (Hiring Manager)". Marta is not introduced anywhere in the user-visible model. If the operator hovers / clicks the conversation row for context, there is no Marta-profile to land on.

---

## 3. Missing entities — RESULT: **3 referenced-but-missing**

- **Maya** — referenced in `sample-cohorts.ts` (worker_id `wkr_01J0MAYA`) and in onboarding copy. No canonical worker record.
- **Peta** (Project Coordinator) — `docs/build-plan/streams/M21-vertical-pack-project-coordinator.md` exists, design mockup `docs/skycot/design-explorations/2026-05-18/worker-detail/option-peta-project-coordinator.html` exists, and an operator memory note dated 2026-05-19 says "First non-vertical cohort family. Peta... cohorts page updated with Project & Ops as 7th dept tab." But there is **no Peta seed entry** in `web/lib/sample-*.ts` and no "Project & Ops" department label anywhere in the seed.
- **Patrick** (Status Reporter) — same situation. M22 brief, mockup, operator memo, no seed.

If the operator wants to record a demo that includes the "Project & Ops" tab to show breadth of departments, the data isn't there.

---

## 4. Stale dates / timestamps / causality — RESULT: **3 issues**

### SD-1. Sam's invoice flow has reversed causality (SEVERE for narrative)

`sample-audit.ts` for Sam:

| Order in feed | Verb | Ago |
|---|---|---|
| Most recent | "extracted INV-2024-08741 line items" | 6 m ago |
|             | "flagged duplicate invoice INV-2024-08739" | 38 m ago |
|             | "queued $14,250 for CFO approval (INV-2024-08741)" | 1 h 32 m ago |

The CFO approval was queued **before** the extraction. The `sample-audit-detail.ts` `SAM_001` even has `followUp: { id: "act_sam_002", verb: "queued $14,250 for CFO approval (INV-2024-08741)" }` — which is correct logically, but `act_sam_002` is hours older than `act_sam_001` in the feed. If a forensics-minded viewer notices, the audit chain doesn't make causal sense.

### SD-2. Jamal's ticket SUP-29382 is both resolved and paused

- `sample-audit.ts`: JAMAL "resolved SUP-29382 with KB article SK-413" — succeeded, 4 min ago.
- `sample-audit-detail.ts`: hand-crafted detail says "marked resolved", "Sent, 200 OK · customer notified".
- `sample-worker-skills.ts` (Jamal's learned patterns): "Paused ticket SUP-29382 — customer asked to come back next week."

Same ticket can't be in both states. Internal-consistency check fails.

### SD-3. Sara Lin contradictions

- `sample-audit.ts`: "closed JD-2024-07 with hire (Sara Lin → backup pool)" — 10 days ago.
- `sample-audit.ts` (same array): "drafted rejection email to Sara Lin" — 47 min ago, status `pending`.
- `sample-approvals.ts`: approval `ap_01H9Z1` for Priya "wants to send a rejection email" to Sara Lin — 22 hours ago.

Sara Lin was moved to backup pool 10 days ago but is still pending a rejection email today. Three different views of her, none reconcilable.

### SD-4. Hard-coded calendar dates

- `sample-team.ts` + `sample-workers-detail.ts`: Rio "paused by you on May 7" — but today is **May 20, 2026**.
- `sample-approvals.ts`: Mariah's reason "You said 'always check counter-offers' on May 4" — May 4 is roughly 2 weeks ago, OK but generic.
- `sample-approvals.ts`: Priya "Proposed: Thursday May 14, 10:30 AM NZST" — May 14 is **6 days in the past**. The interview is "proposed" for a past date.

The seed was last hand-edited probably in early May. The narrative dates have not been refreshed.

---

## 5. Personality consistency — RESULT: **major drift**

Mariah's personality on the personality card matches the runbook **exactly** (Patient, Detail-oriented, Picture-perfect listings, Polite). Her overview KPIs match too: 3 listings live (+33%), 11 buyer replies, 2 items flagged below floor — all four runbook callouts are correct.

But across the rest of the seed, **Sam, Mariah, and Nori each have TWO personas** — one used in the team / overview / memory views, and a completely different one used in the audit / skills / replay views:

| Worker | Team / Overview persona | Audit / Skills / Replay persona |
|--------|-------------------------|---------------------------------|
| Sam    | On-call queue ops — PagerDuty, alerts, incidents (`sample-team.ts:78`, `sample-worker-overview.ts:149`, `sample-worker-memory.ts:154`) | AP Specialist — invoice OCR, vendor reconciliation, CFO approval (`sample-audit.ts:60`, `sample-worker-skills.ts:79`, `sample-audit-detail.ts:84`) |
| Mariah | Marketplace listings — re-pricing, photos, buyers (`sample-team.ts:31`, `sample-worker-overview.ts:77`) | Vendor Manager — RFP responses, vendor scoring, renewal pricing (`sample-audit.ts:70`, `sample-worker-skills.ts:103`, `sample-audit-detail.ts:122`) |
| Nori   | Marketing brand comms — announcements, reach, brand voice (`sample-team.ts:64`, `sample-worker-overview.ts:125`, `sample-worker-memory.ts:133`) | RevOps quote-validation — pricing rules, discount flagging, AE commission (`sample-audit.ts:88`, `sample-worker-skills.ts:138`, `sample-audit-detail.ts:196`) |

Mariah's memory file (`sample-worker-memory.ts:82-106`) shows both personas in the same record: facts 4 and 5 are listings/photos (the marketplace persona), facts 1-3 are vendor/strategic-partners (the vendor persona), and the active conversations are split between a vendor and a marketplace buyer.

This is the largest data-integrity issue in the audit. The demo runbook is built around the marketplace personas. The /audit timeline at 07:30, which the CEO is told is the "forensic" climax, will show Mariah drafting RFPs, Sam queuing CFO approvals, and Nori validating QT quotes — none of which the operator just narrated 5 minutes earlier on the team page.

Additionally: the `WorkerPersonalityCard` (which the runbook calls out by name at 02:30) only has hand-crafted personality entries for **Mariah, Priya, and Jamal**. Sam, Nori, and Rio render the `DEFAULT_PERSONALITY` ("Reliable, Curious, Helpful" + 3 generic bullets). The runbook only walks through Mariah, so this isn't a guaranteed demo blocker — but anyone who clicks Sam or Nori during questions will see the bland default.

---

## 6. Demo runbook match — RESULT: **mostly aligned, two stale numbers**

| Runbook claim (line) | Source in seed | Match? |
|---|---|---|
| "These are real AI teammates — Mariah, Priya, Jamal, Nori" (00:25) | `sample-team.ts` has all 4 (+ Sam, Rio) | ✓ |
| "7 decisions need you" (01:15) | `sample-approvals.ts` has 7 entries | ✓ |
| "6 teammates, one team" (02:00) | `sample-team.ts` = 6 | ✓ |
| "1,122 actions this week" (02:00) | `team/page.tsx:145` `workers.length * 187` = 6 × 187 = 1,122 | ✓ |
| "saving you 72 hours" (02:00) | `team/page.tsx:146` `workers.length * 12` = 72 | ✓ |
| "Patient, Detail-oriented, Picture-perfect listings, Polite" (02:30) | `WorkerPersonalityCard.tsx:37` PERSONALITIES.Mariah | ✓ |
| "spotting price gaps. Cropping photos. Writing tight 60-word copy" (02:30) | `WorkerPersonalityCard.tsx:38-41` greatAt | ✓ (slightly paraphrased — runbook says "60-word", seed says "tight 60-word listing copy with real specifics") |
| "escalates to you: price drops below your floor. Edited-looking photos. Multiple competing offers" (02:30) | `WorkerPersonalityCard.tsx:44-48` escalateWhen | ✓ |
| "3 listings live (+33%)" (02:45) | `sample-worker-overview.ts:87` k-live = "3", deltaPct 33 | ✓ |
| "11 buyer replies" (02:45) | `sample-worker-overview.ts:88` k-replies = "11" | ✓ |
| "2 items flagged below floor" (02:45) | `sample-worker-overview.ts:89` k-floor = "2" | ✓ |
| "Catalog with 50+ templates" (04:00) | `workflow-templates.ts` TEMPLATE_COUNT = 50 | ✓ (right at the line; CEO can claim "50+" only by counting the 8 SEED_WORKFLOWS on `/workflows` separately) |
| "Eight of thirteen rows are clean wins" (06:30) | `/compare` page — not audited in this scope; should be a wave-1 / wave-3 finding | not verified |
| "SOC 2, ISO 27001, GDPR, HIPAA, AFSL on day one" (07:30) | `sample-export.ts` EXPORT_FORMATS = soc2-type-ii, gdpr-article-15, hipaa-disclosure-log, generic — no AFSL / ISO 27001 surface | partial (4 of 5 frameworks have a code path) |

The runbook's specific Mariah numbers (3 / +33% / 11 / 2) all match cleanly.

**Stale claim**: the runbook caption "your teammates have batched these for your review. Each one is one tap." (01:30) plus "Mariah is going to send that listing" (01:45) after clicking Approve — works in isolation, but if the CEO clicks the **/audit** drill-in next, they will see a synthetic generic decision chain (per CR-1) instead of the rich provenance the runbook implies forensic depth from.

---

## Top 5 highest-impact issues visible on the demo path

1. **CR-1 / CR-2: audit drill-in IDs don't match the hand-crafted detail records.** Every click from /audit or from a worker overview → 404 OR generic synthetic decision chain. The forensic climax at 07:30 lands flat.
2. **Sam, Mariah, Nori have split personas across views.** The CEO walks the operator through the team page (Mariah = marketplace listings), then later in the same demo /audit shows Mariah drafting RFP responses for "Tomás Industries". The viewer will notice; the audit log is what they're told is "forensic".
3. **Stale `acmeco.com` brand references in `sample-audit-detail.ts`** (3 hand-crafted detail bodies). The project is Skycot. Anyone clicking through to see the AI-drafted output sees `accounts@acmeco.com`, `app.linear.app/acmeco/issue/RFP-184`, `help.acmeco.com/SK-413`.
4. **Sam's invoice flow has reversed causality** (extraction at 6 min ago, CFO approval queued 1 h 32 m ago for the same invoice). Forensic-minded viewer notices immediately.
5. **Cohorts references phantom worker `wkr_01J0MAYA` + Mariah missing from cohort.** /cohorts is currently outside the runbook but is one of the locked design pages — anyone exploring will hit a broken cross-reference.

## Whether the runbook's specific numbers match seed data

**Yes for everything explicitly called out in the runbook voiceover**: 7 / 6 / 1,122 / 72 / 3 / +33% / 11 / 2 all match. The runbook will read accurately on the team page and on Mariah's overview. The numbers gap opens only if the CEO improvises off-script (clicks on a non-Mariah teammate's overview, clicks an audit row, explores cohorts).

## Single highest-impact fix before record

**Rename the 5 hand-crafted IDs in `lib/sample-audit-detail.ts` from `act_priya_001` / `act_sam_001` / `act_mariah_001` / `act_jamal_001` / `act_nori_001` to `act_01J0PRIYA_000` / `act_01J0SAM_010` / `act_01J0MARIAH_018` / `act_01J0JAMAL_025` / `act_01J0NORI_032`** so they match the first row each persona's seed produces in `sample-audit.ts`. Do the same in `lib/sample-replay.ts` so the Replay button on the audit detail page wires to the right outcomes.

This is a ~20-line read-only-time rename. It restores the forensic provenance click-through that the runbook leans on at 07:30, and unblocks every per-row replay verdict in one shot. None of the other findings are runbook-blockers if the demo stays on script.

---

## Appendix — what was NOT audited (scope boundaries)

- `/compare` page seed (rows + check marks) — not in this audit.
- `/landing` page hero numbers / customer logos — not in this audit.
- `/onboarding` flow defaults — not in this audit (sample-team-onboarding mention only).
- `/pricing` plans / claims — not in this audit.
- Server-side gateway responses if the gateway is running — this audit covers only the dev fallback path (`ARGO_GATEWAY_AUTH_BYPASS=1`). Production responses are out of scope.
- The runbook claim "Eight of thirteen rows are clean wins" for /compare — flagged for a separate sweep.
