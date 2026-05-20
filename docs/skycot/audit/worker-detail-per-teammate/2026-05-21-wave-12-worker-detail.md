# Wave-12 Audit — Worker-Detail Per-Teammate

**Date:** 2026-05-21
**Auditor:** Wave-12 agent #2
**Scope:** `/workers/[id]/{overview,activity,memory,skills,trust,escalation,settings}` for all 6 sample personas (Priya, Mariah, Jamal, Nori, Sam, Rio).
**Mode:** Read-only. No code changes.

## Audit method

Read every per-worker sample dataset (`web/lib/sample-worker-*.ts` + `sample-workers-detail.ts` + `sample-audit.ts`) and every page under `web/app/(app)/workers/[workerId]/`. Scored each persona on:

1. Personality card distinct (4 unique traits) — `WorkerPersonalityCard.tsx`
2. Responsibilities listed (3+) — `sample-worker-overview.ts.duties`
3. KPIs visible (3+) — `sample-worker-overview.ts.kpis`
4. Recent activity (5+ rows) — `sample-worker-activity.ts` + `sample-audit.ts`
5. Memory facts (5+ facts) — `sample-worker-memory.ts.facts`
6. Skills active/library/learned (mix) — `sample-worker-skills.ts`
7. Worker-scoped audit rows — `/audit?worker=<id>` (Activity tab "Open in Audit timeline →" link)
8. Escalation tab (post #1904) — `escalation/page.tsx` SEEDS
9. Policies present — folded into Settings tab `ApprovalPolicySection` (no standalone `/policies` route, by design)

There is NO standalone `/workers/[id]/audit` route; per-worker audit is the Activity tab, which deep-links into `/audit?worker=<id>`. There is NO standalone `/workers/[id]/policies` route; policies live inside Settings under `ApprovalPolicySection`. Both are correct per the layout — but the audit-brief checklist phrased these as separate routes which would mis-read at face value.

## Worker-by-worker findings

### Priya — Recruiting (wkr_01J0PRIYA) — Score 10/10

- Personality card: 4 distinct traits (Thorough / Respectful / Fast / Calibrated). 4 great-at + 4 escalate-when + 3 signature. **PASS**
- Responsibilities: 4 duties incl. one disabled. **PASS**
- KPIs: 4 (screened today, interviews booked, pending drafts, time-to-first-response). **PASS**
- Recent activity: 9 sample-worker-activity rows + 12 sample-audit rows. **PASS**
- Memory facts: 6 enterprise-flavored facts (diversity goals, comp cap, Friday rejection-queue, Workato referrals). **PASS**
- Skills: 3 active + 2 library + 1 learned pattern. **PASS**
- Worker-scoped audit: 12 rows in sample-audit.ts. **PASS**
- Escalation tab: 3 seeded rules + 2 history rows. **PASS**
- Settings/policies: standard rows. **PASS**

Verdict: Priya is the only persona where **every surface tells the same story** (recruiting, JD-2024-08, candidate #4821, Diego Alvarez, Sara Lin, Marta). She is unambiguously demo-ready.

### Mariah — Procurement/Marketplace (wkr_01J0MARIAH) — Score 6/10

- Personality card: 4 distinct traits (Patient / Detail-oriented / Picture-perfect listings / Polite). **PASS**
- Responsibilities: 4 duties (listings, repricing, buyer-chat, floor flag). **PASS**
- KPIs: 4 KPIs. **PASS**
- Recent activity: 3 sample-worker-activity rows (BELOW the 5+ threshold) + 8 sample-audit rows. **PARTIAL — activity feed thin**
- Memory facts: 5 facts. **PASS**
- Skills: 2 active + 1 library + 0 learned. **PASS**
- Worker-scoped audit: 8 rows. **PASS**
- Escalation tab: 4 seeded rules + 3 history rows. **PASS**

**CRITICAL CROSS-FILE INCONSISTENCY:** Mariah is a marketplace listings worker in `sample-workers-detail.ts` + `sample-worker-overview.ts` + `sample-worker-memory.ts` + `WorkerPersonalityCard.ts` + `escalation/page.tsx`, but in `sample-worker-activity.ts` (3 of 3 rows) and `sample-audit.ts` (`Mariah — Vendor Manager`) she is a **vendor-management / RFP worker**. The skills mock has both: rfp-response-drafter (vendor) + renewal-checklist-runner (vendor) — neither matches the "listings" identity. Operator demo'ing the page will see two contradicting backstories.

### Jamal — Customer Support (wkr_01J0JAMAL) — Score 8/10

- Personality card: 4 distinct traits (Calm under pressure / Empathic / Quick / Honest). **PASS**
- Responsibilities: 4 duties (tier-1, triage, escalate, CSAT). **PASS**
- KPIs: 4 KPIs. **PASS**
- Recent activity: 2 sample-worker-activity rows (BELOW threshold) + 9 sample-audit rows. **PARTIAL**
- Memory facts: 5 facts. **PASS**
- Skills: 2 active + 2 library + 1 learned. **PASS**
- Worker-scoped audit: 9 rows. **PASS**
- Escalation tab: 3 seeded rules + 0 history rows. **PARTIAL — empty history while other personas have history**

Identity is consistent (support story holds across all files). Activity-feed thinness and empty escalation history are the only gaps.

### Nori — Brand/Marketing (wkr_01J0NORI) — Score 4/10

- Personality card: **NO seed — falls through to DEFAULT_PERSONALITY** ("Reliable / Curious / Helpful"). **FAIL** — 4-trait requirement defaulted out, identity collapsed.
- Responsibilities: 4 duties (announcements / voice / scheduling / reach roundup). **PASS**
- KPIs: 4 KPIs (reach, drafts, engagement, draft-to-approve). **PASS**
- Recent activity: 2 sample-worker-activity rows (BELOW threshold). **FAIL**
- Memory facts: 5 facts. **PASS**
- Skills: 2 active + 1 library + 0 learned. **PASS**
- Worker-scoped audit: 10 rows. **PASS**
- Escalation tab: **NO seed — falls through to DEFAULT_SEED** (3 generic rules, 0 history). **FAIL**

**CRITICAL CROSS-FILE INCONSISTENCY:** Detail says brand/marketing comms. Skills mock says quote-pricing/deal-desk-router (sales). Activity mock says "validated quote QT-2024-7283 / flagged discount for Deal Desk" (RevOps). Audit mock says `Nori — RevOps`. Four files, two roles — at least two of them must change.

### Sam — Reliability/On-call (wkr_01J0SAM) — Score 3/10

- Personality card: **NO seed — DEFAULT_PERSONALITY**. **FAIL**
- Responsibilities: 4 duties (triage / incident / page / post-mortem) — one disabled. **PASS**
- KPIs: 4 KPIs, but 3 of 4 values are "—" because Sam is `blocked` (PagerDuty disconnect). **PARTIAL — empty-numeric values feel hollow**
- Recent activity: 4 sample-worker-activity rows (BELOW threshold; deliberately invoice-themed not on-call themed). **FAIL — wrong domain**
- Memory facts: 4 facts (BELOW the 5+ threshold). **FAIL**
- Skills: 2 active + 2 library + 1 learned — all invoice/AP themed (`invoice-line-item-extractor`, `duplicate-invoice-detector`, `purchase-order-matcher`, `flag-Q4-rush-invoices`). **WRONG DOMAIN.**
- Worker-scoped audit: 10 rows — all AP-themed (`extracted INV-2024-08741`, `CFO approval`, `reconciled vendor statement`). **WRONG DOMAIN.**
- Escalation tab: **NO seed — DEFAULT_SEED**. **FAIL**

**CRITICAL CROSS-FILE INCONSISTENCY:** Sam in `sample-workers-detail.ts` is the on-call SRE ("watches the on-call queue", "PagerDuty disconnected"). Sam in `sample-worker-memory.ts` is the on-call SRE (pages secondary, post-mortems, PagerDuty alias). Sam in `sample-worker-skills.ts`, `sample-worker-activity.ts`, and `sample-audit.ts` is an **Accounts Payable specialist** (invoices, POs, CFO, reconciliation). Two completely different roles bolted together. Worst persona by drift.

### Rio — Real Estate (wkr_01J0RIO) — Score 4/10

- Personality card: **NO seed — DEFAULT_PERSONALITY**. **FAIL**
- Responsibilities: 3 duties — all disabled because paused. **PASS**
- KPIs: 4 KPIs — all "—" because paused. **PARTIAL — by design but hollow**
- Recent activity: 0 sample-worker-activity rows (not in `SAMPLE_WORKER_ACTIVITY`) + 5 sample-audit rows. **PARTIAL — activity tab will show empty-state**
- Memory facts: 3 facts (BELOW 5+ threshold). **FAIL**
- Skills: 0 active + 1 library + 0 learned. **PASS for paused worker**
- Worker-scoped audit: 5 rows. **PASS for paused worker**
- Escalation tab: **NO seed — DEFAULT_SEED**, no trust history seed either. **FAIL**

The "paused" status excuses the hollowness on duties/KPIs/activity, but personality + memory + escalation gaps are not excused by status. Rio also intentionally has no `department / team / kpiToday` per the file's "tests the missing-fields fallback" comment — that's a hero-card edge case, but it does mean Rio looks the most stub-like.

## Score summary

| Worker | Score | Verdict |
|--------|------:|---------|
| Priya  | 10/10 | Demo-ready. Story holds across every file. |
| Jamal  | 8/10  | Story holds; minor thinness. |
| Mariah | 6/10  | Two contradicting backstories (listings vs vendor mgmt). |
| Nori   | 4/10  | Two contradicting backstories (brand vs RevOps) + DEFAULT personality + DEFAULT escalation. |
| Rio    | 4/10  | Paused excuses thinness, but personality + memory + escalation still default. |
| Sam    | 3/10  | Two roles in one worker. Skills/activity/audit are AP; detail/memory/personality slot is on-call (DEFAULT). |

Mean: 5.8/10. Median: 5/10.

## Top 3 inconsistencies between workers

1. **Personality card seed: only 3 of 6 workers seeded.** Priya / Mariah / Jamal have real 4-trait personality + great-at + escalate-when + signature lists. Nori / Sam / Rio fall through to a single shared DEFAULT_PERSONALITY (`Reliable / Curious / Helpful`). Three of six workers therefore feel like NPCs — Priya feels like a colleague, Sam feels like a placeholder. (`WorkerPersonalityCard.tsx:35-111`)

2. **Escalation tab seed: only 3 of 6 workers seeded.** Same split — Mariah / Priya / Jamal have specific rules + history; Nori / Sam / Rio fall through to DEFAULT_SEED (3 generic rules, empty history). Trust-history is similar — Rio has no seed and uses the FALLBACK_ARC flat line. (`escalation/page.tsx:63-173`, `sample-worker-trust-history.ts:135-145`)

3. **Cross-file role drift for 3 of 6 workers.** Three personas have **two different roles in two different files**:
   - **Sam:** on-call SRE in overview/memory/detail; AP specialist in skills/activity/audit
   - **Mariah:** marketplace lister in overview/memory/detail/personality/escalation; vendor manager in skills/activity/audit
   - **Nori:** brand/marketing in overview/memory/detail; RevOps/sales in skills/activity/audit

   The skills/activity/audit set came from one authoring pass (probably copying from an older "enterprise" persona pack); the overview/memory/detail/personality/escalation set is a newer "consumer-friendly" framing. They were never reconciled. A user landing on Sam's overview ("watches the on-call queue, PagerDuty re-link needed") and then clicking Skills ("invoice-line-item-extractor v5.2.0, 312 uses last 7d") will lose trust immediately.

## Single best persona for demo

**Priya.** She is the only worker whose seven surfaces tell the same coherent story — recruiting, JD-2024-08, the candidates Diego Alvarez and Sara Lin appear in overview activity, memory conversations, skills sample-outputs, and audit-row verbs. Her personality card has 4 distinct traits and 11 bullets across great-at/escalate/signature. Her escalation tab has 3 seeded rules + 2 history rows. Her audit feed has 12 rows including cohort-scope, escalation, refusal, memory-write, and learn events — the widest cross-section of audit action types in the dataset.

If the operator can only show one persona in the v1 demo, show Priya. If they can show two, pair her with Jamal (the second-most-consistent). Avoid demoing Sam.

## Suggested follow-ups (not in scope for this audit)

- Seed `WorkerPersonalityCard.PERSONALITIES` for Nori, Sam, Rio (3 entries × ~14 lines each).
- Seed `escalation/page.tsx SEEDS` for Nori, Sam, Rio (3 entries × ~10 lines each).
- Seed `sample-worker-trust-history.SAMPLE_WORKER_TRUST_HISTORY` for Rio (1 entry).
- Reconcile cross-file role drift for Sam, Mariah, Nori. Either rewrite skills/activity/audit to match the consumer framing, or rewrite overview/memory/detail to match the enterprise framing. The enterprise framing reads more demo-worthy (CFO sign-off, INV/PO numbers) but contradicts the operator-locked "your AI hires are colleagues" worker-detail register.
- Expand `sample-worker-activity` to 5+ rows per worker (currently Mariah=3, Jamal=2, Nori=2, Rio=0).
- Sam memory: add at least 1 more fact to reach the 5+ threshold (currently 4).
- Rio memory: add 2 more facts to reach 5+ (currently 3).
