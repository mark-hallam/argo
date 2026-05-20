# SKYCOT 12-persona journey audit — 2026-05-20

Status: complete (analytical pass; no live-environment scoring since SKYCOT's
production-data is mocked in dev). Where "score" appears it's a SKYCOT-readiness
score, 0-100, weighted: brand voice (20) · UX clarity (25) · feature coverage
for the persona's job-to-be-done (35) · trust signals (20). Scored against
current `origin/main` (post demo-audit, commit `9dc08c16`).

---

## 1. SOLO E-COMMERCE RESELLER (Anna, 32, Auckland)

**Industry**: marketplace reseller (Facebook Marketplace + TradeMe).
**Team size**: 1 (her).
**Annual rev**: $40k side-business.
**Job to be done**: list 30+ items/week, reply to buyers, post pickups.

**Journey**: `/landing` → "Hire your first worker" → `/auth/signup` →
`/auth/check-email` → `/auth/verify` → `/hire` → meets Mariah → `/team`.

**Pass criteria**:
- "Solo" framing visible in marketing
- Mariah personality reads as a human
- Pricing fair for a 1-person business

**Findings**:
- ✅ Landing hero "Hire a teammate" speaks to her — she's been looking for help.
- ✅ Mariah's personality card on /workers/[id] (Patient · Detail-oriented ·
  Picture-perfect listings · Polite) hits exactly. "Always confirms the seller
  before posting publicly" is the trust signal she needs.
- ⚠ /billing shows Free / Pro / Enterprise — Free is "1 worker · 100 actions/mo"
  which is too tight for 30 items/week. Pro at $29 is reasonable.
- ❌ /hire wizard renders a minimal Q&A (textarea) — should be the locked F1
  editorial-center per brief `web-hire-wizard-f1-polish` (P1 follow-up).

**Score: 82/100.** Top gap: hire wizard polish.

---

## 2. SALES VP (Diego, 42, mid-market SaaS, 50-person team)

**Industry**: B2B SaaS (developer tools).
**Job to be done**: qualify 200+ inbound leads/week, book demos, hand off to AEs.

**Journey**: comes from a tweet → `/landing` → `/compare` → `/auth/signup`.

**Pass criteria**:
- Quantifiable wins vs Outreach.io / Salesloft (his current stack)
- Approval-gated outreach (he doesn't want hallucinated emails sent to enterprise prospects)
- Multi-AE handoff workflow

**Findings**:
- ✅ /landing "Hire a teammate" + "Asks before it acts" copy resonates.
- ✅ /compare row "Multi-channel native" + "Approval-first by default" =
  exactly his concerns.
- ✅ /workflows shows `qualify_lead` skill (Sales vertical, autonomous, 7 steps).
- ⚠ /compare missing direct Outreach.io / Salesloft head-to-head — only Zapier
  / n8n / Crew.ai / Lindy. Sales VPs don't compare against Crew.ai. **GAP**.
- ❌ No "Estimated $$ saved" calculator on /landing or /compare — would land hard
  for a Sales VP doing CFO math.

**Score: 73/100.** Top gaps: sales-vertical competitor coverage, ROI math.

---

## 3. RECRUITING MANAGER (Priya, 38, 200-person eng org)

**Industry**: Engineering recruiting at a mid-size B2B company.
**Job to be done**: screen 500 candidates/week against 12 open reqs, book screens, kick off background checks.

**Journey**: hears about SKYCOT via Greenhouse community → `/landing` →
`/workflows/people-ops/screen_candidate` → `/auth/signup`.

**Pass criteria**:
- See exact candidate-screening workflow
- ATS integration mentioned (Greenhouse, Lever, etc.)
- Anti-bias guardrails visible

**Findings**:
- ✅ Visual workflow viewer at `/workflows/people-ops/screen_candidate` shows
  8 clear steps including "Compose response" + "Update hiring manager"
  + "Append to ATS". This is genuinely impressive.
- ✅ Priya worker exists in the demo team — same name as the persona, awkward
  but happens.
- ⚠ No anti-bias / EEO disclosure visible. Recruiting managers will ASK.
- ⚠ ATS integration only mentioned in workflow step ("Greenhouse webhook or
  email") — should be a featured connector on /landing.
- ❌ No template gallery showing 5 common recruiting workflows out-of-box.

**Score: 76/100.** Top gaps: bias disclosure, ATS positioning,
template gallery.

---

## 4. SUPPORT LEAD (Jamal, 35, 500-person consumer brand)

**Industry**: D2C consumer brand (skincare).
**Job to be done**: triage 1,500 tickets/week, defract with KB, escalate VIPs.

**Journey**: searches "AI for Zendesk" → external article mentions SKYCOT →
`/landing` → `/workflows/support/triage_ticket`.

**Pass criteria**:
- Direct Zendesk / Intercom integration
- VIP-tier handling visible
- "Median reply time" KPI visible

**Findings**:
- ✅ Jamal worker exists with personality "Calm under pressure · Empathic · Quick · Honest". Perfect for support persona.
- ✅ Jamal's signature behaviors: "Always names the human owner when escalating",
  "Sends a 'still investigating' note at the 4h mark", "Never closes a ticket
  without a one-line outcome" — this is REAL support-lead expertise encoded.
- ✅ "12 tickets resolved · median reply 4m" visible on Team page card.
- ⚠ `/workflows/support/triage_ticket` doesn't exist as seed data (only `screen_candidate`
  + `post_listing` have seeded steps). Page would show "Workflow not found".
  **GAP** — need at least 2-3 more seeded workflows for the demo.
- ❌ No Zendesk / Intercom connector visible.

**Score: 71/100.** Top gaps: more seeded workflows, connector logos.

---

## 5. FINANCE OPS (Sara, 41, 100-person agency)

**Industry**: Marketing agency.
**Job to be done**: process 300 invoices/month, reconcile vendor data, flag
discrepancies.

**Journey**: chief-of-staff sends her the SKYCOT landing →
`/landing` → `/workflows/finance/extract_invoice`.

**Pass criteria**:
- SOC 2 / PCI compliance visible
- QuickBooks / Xero / NetSuite integration
- Per-vendor audit trail

**Findings**:
- ✅ Trust strip on /landing mentions SOC 2 TYPE II + GDPR ARTICLE 30.
- ✅ Sam worker handles "extracted INV-2024-08741 line items" — visible on
  Audit page.
- ⚠ No QuickBooks / Xero / NetSuite logo / connector mention. Finance ops people
  need this concrete.
- ⚠ Per-vendor audit trail isn't surfaced — workflow shows "Pull line items..."
  but doesn't show audit lineage to vendor.
- ❌ No "$ saved per invoice" KPI placeholder.

**Score: 68/100.** Top gaps: finance ERP connector positioning, vendor-level audit visibility.

---

## 6. MARKETING CMO (Tomás, 39, 30-person agency)

**Industry**: full-service marketing agency.
**Job to be done**: campaign briefs, content calendar drafts, client status reports.

**Journey**: comes from a Twitter thread → /landing → /skills → /compare.

**Pass criteria**:
- Brand voice consistency (his clients pay for this)
- "Tone" controls visible
- Email + Slack templating

**Findings**:
- ✅ /skills shows "Outreach.compose" + brand-loud register — clearly mentions
  voice/style ("writes the candidate-facing message in your voice").
- ✅ Nori worker (Marketing cohort) is on the Team page — "drafts product
  comms and last campaign 12,400 reached".
- ⚠ No "Tone settings" or "Brand voice training" visible.
- ⚠ No content-calendar workflow seeded.
- ❌ Marketing cohort shows "Pre-trust · Grade 0 · 1 member · 1 skill" —
  feels thin compared to Support / Sales.

**Score: 65/100.** Top gaps: brand voice training feature, thicker marketing seed.

---

## 7. CUSTOMER SUCCESS DIRECTOR (Naomi, 44, B2B SaaS Series B)

**Industry**: workflow automation SaaS.
**Job to be done**: QBR prep, churn alerts, success-plan generation.

**Journey**: Salesforce admin shares /landing → /workflows.

**Pass criteria**:
- Salesforce integration explicit
- Health-score signal handling
- Renewal-conversation drafting

**Findings**:
- ✅ /workflows catalog brand-loud + visual.
- ⚠ No CS-specific workflow seeded.
- ⚠ No Salesforce integration mentioned outside the connector hints.
- ❌ /landing copy is heavy on hiring use-cases — CS director might not see herself.

**Score: 64/100.** Top gaps: vertical-specific copy on landing, CS workflow seeds.

---

## 8. PROCUREMENT MANAGER (Aroha, 47, manufacturing)

**Industry**: industrial parts manufacturer (NZ-based).
**Job to be done**: 80-supplier price-watch, RFQ generation, contract renewal alerts.

**Journey**: ERP vendor mentions SKYCOT → /landing → /compare.

**Pass criteria**:
- Supplier-data privacy guarantees
- On-prem / self-host option visible
- Procurement-specific workflow

**Findings**:
- ✅ /compare row "Single binary you can self-host" = green check for SKYCOT
  vs Zapier red X.
- ✅ /compare row "Air-gapped deploys for regulated environments" = green.
- ❌ /landing doesn't mention manufacturing or industrial use cases.
- ❌ No procurement workflow seeded.

**Score: 68/100.** Top gaps: industrial vertical copy, procurement workflow seed.

---

## 9. HR BUSINESS PARTNER (Olivia, 36, 800-person enterprise)

**Industry**: Fortune 500 division.
**Job to be done**: onboarding automation, performance-review prep, exit-interview synthesis.

**Journey**: comes from a People Ops Slack community → /landing → /compare → /workflows.

**Pass criteria**:
- Workday / SAP SuccessFactors integration
- SOC 2 / HIPAA disclosure
- BYO-model (their enterprise GPT) supported

**Findings**:
- ✅ Trust strip mentions SOC 2 + GDPR.
- ✅ /compare green checks on RBAC + encryption-at-rest.
- ⚠ No Workday / SF connector visible.
- ⚠ No BYO-model copy — but settings/providers tile suggests it ("Anthropic,
  OpenAI, custom providers").
- ❌ HIPAA NOT visible (only SOC 2 + GDPR on the trust strip). Enterprise
  HR needs this for medical-leave / disability cases.

**Score: 62/100.** Top gaps: HIPAA disclosure, enterprise HRIS connectors.

---

## 10. COMPLIANCE OFFICER (Daniel, 50, regulated fintech)

**Industry**: regulated financial services (Australia AFSL).
**Job to be done**: monthly AML reporting, regulatory drift monitoring, audit prep.

**Journey**: comes from a compliance vendor referral → /landing → /audit → /compare.

**Pass criteria**:
- Forensic audit chain
- Per-action lineage (worker, skill, prompt, cost)
- Regulatory frameworks named explicitly

**Findings**:
- ✅ /audit page IS the locked B3-enterprise Etherscan-style log. Truly
  forensic-feeling.
- ✅ /compare green check on "audit-logged with full lineage".
- ⚠ No specific regulatory framework named (ISO 27001, SOC 2 Type II, AFSL).
  Compliance officers want bullet-point lists.
- ⚠ No "export to evidence package" CTA on /audit.

**Score: 78/100.** Top gaps: compliance-framework name-drops, evidence-pack export.

---

## 11. ENG MANAGER (Marcus, 33, deep-tech startup, 12 engineers)

**Industry**: AI/ML startup.
**Job to be done**: shave 10-15 hours/wk of "ops glue" off the team.

**Journey**: discovers via HN → /landing → /compare → GitHub.

**Pass criteria**:
- API + CLI exist
- Self-host option
- LLM-routing / cost-aware

**Findings**:
- ✅ /landing copy "single-binary, enterprise-grade" speaks to him.
- ✅ /compare green on single-binary + air-gapped.
- ✅ /audit shows "$0.004 / action" cost data — eng managers love this.
- ⚠ No GitHub link visible on /landing (would close the trust gap for HN
  readers immediately).
- ⚠ No public docs link.

**Score: 80/100.** Top gaps: GitHub badge on /landing, docs link.

---

## 12. CEO OF SERVICES BIZ (Mark, the operator)

**Industry**: small services / professional consulting.
**Job to be done**: replace "hiring assistant for $80k/yr" with AI.

**Journey**: the operator's own journey through the product.

**Pass criteria**:
- Mission alignment ("teammates not tools")
- "Will my customers buy this?" gut check
- "Does it look launch-ready?" emotional check

**Findings**:
- ✅ Brand-loud language is consistent + viral-feeling.
- ✅ /landing → /compare → /workflows → /workers gives a complete story
  arc.
- ✅ "How your teammates actually think." (Audiowide on /workflows) is the
  kind of headline you remember.
- ⚠ The journey assumes the user knows SKYCOT exists — no SEO landing pages
  for "AI alternative to Zapier" / "Hire AI sales assistant" / etc.
- ⚠ /landing footer says "MADE IN MELBOURNE · skycot.com" — needs proper
  legal footer (privacy / terms / contact / GitHub link).

**Score: 88/100.** Top gaps: SEO landings, proper footer.

---

# AGGREGATE FINDINGS

## Score distribution (out of 100)

```
12 personas, avg: 73/100, median: 72
High: 88 (CEO), 82 (solo reseller)
Low:  62 (HR BP), 64 (CS director)
```

## Top 10 cross-persona gaps (ranked by frequency × severity)

1. **More seeded demo workflows** — only `post_listing` + `screen_candidate` exist.
   Need: triage_ticket, qualify_lead, extract_invoice, approve_refund, draft_QBR,
   draft_RFQ, generate_status_report, BI dashboard digest. Recommend 8 total.
   **Touched personas: 4, 5, 6, 7, 8, 9.**
2. **Connector / integration logos missing on /landing** — Slack is the only
   one mentioned. Add: Gmail, Outlook, Zendesk, Intercom, QuickBooks, Xero,
   NetSuite, Salesforce, HubSpot, Workday, Greenhouse, Lever, Notion, Linear,
   GitHub. **Touched personas: 2, 3, 4, 5, 7, 9.**
3. **HIPAA + ISO 27001 + AFSL on trust strip** — only SOC 2 + GDPR shown.
   **Touched personas: 9, 10.**
4. **ROI / cost calculator on /landing** — "estimate hours saved" widget.
   **Touched personas: 2, 11.**
5. **Sales / CRM head-to-head on /compare** — currently only generic Zapier/n8n.
   Need Outreach.io / Salesloft / Apollo on /compare. **Touched personas: 2.**
6. **Brand-voice / tone-training UX** — agencies need this exposed.
   **Touched personas: 6.**
7. **Public GitHub link** on /landing — closes trust gap for technical buyers.
   **Touched personas: 11.**
8. **Proper /landing footer** — privacy, terms, contact, GitHub.
   **Touched personas: 12.**
9. **Audit page evidence-pack export CTA** for compliance teams.
   **Touched personas: 10.**
10. **Vertical-specific marketing pages** — /for/sales, /for/recruiting,
    /for/support, /for/finance, /for/marketing, /for/cs. **Touched personas: 2-9.**

## "Are we able to compete yet?" — honest verdict

**YES** for:
- Solo founders / SMB ops folks (avg score 80+)
- Engineering managers + technical buyers (avg 80)
- Compliance + ops roles where audit trail matters (78+)

**MARGINAL** for:
- Vertical-specific operators (sales, recruiting, support) — they want their
  workflow as a featured demo (60-75)

**NOT YET** for:
- Enterprise HR/finance buyers expecting concrete integration logos +
  framework disclosures (62-68)

The brand-voice + UX baseline is LEADING (we're ahead of Zapier/n8n on AI-native
language + ahead of Crew.ai on operator surface). The vertical-specific +
connector-logo gaps are the lift-from-leading-edge-back-to-mainstream work.
