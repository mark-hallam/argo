# Wave-3 Copy + Voice Audit

**Auditor:** Wave-3 audit agent #1 (read-only)
**Date:** 2026-05-20
**Scope:** Customer-facing copy across SKYCOT marketing pages (`/landing`, `/compare`, `/pricing`, `/customers`, `/changelog`, `/docs`) and in-product surfaces (`/`, `/team`, `/workers/[id]/overview`, `/workflows`, `/workflows/templates`, `/audit`, `/skills`, `/hire`, `/onboarding/*`, `/auth/*`).
**Dimensions:** Dev jargon stripped · Workers vs teammates voice · Headline quality · CTA clarity · Trust + safety over-claim.

---

## Executive summary

| Dimension | Count | Severity |
|---|---|---|
| Dev jargon hits in customer-facing copy | 19 visible-to-user strings | High — operator-locked rule violated |
| `worker(s)` used as customer noun (should be `teammate`) | 12 visible-to-user strings | High — brand-voice rule violated |
| Weakest headline (demo path) | `/onboarding/welcome` "Welcome to SKYCOT." | Medium — generic, not promise-forward |
| Weakest CTA (demo path) | `<a href="#how">Watch a 90-second demo</a>` (landing hero secondary) | Low — vague verb |
| Trust + safety over-claim lines | 8 lines across 5 files | **CRITICAL — legal exposure** |

---

## 1. DEV JARGON STRIPPED

Operator-locked rule: customer-facing copy expands abbreviations on first use, never raw. The audit found **19 instances** of raw dev jargon in user-visible strings (excluding API route handlers, comments, and type definitions, which are not customer-facing). The most damaging are landing-page hero sub-copy and pricing FAQ.

### Top hits to fix (file:line + replacement)

| # | File:line | Current copy | Plain-language replacement |
|---|---|---|---|
| 1 | `web/app/landing/page.tsx:40` | "Describe the change in plain English. Your teammate proposes the new **TOML**, you approve, it ships." | "Describe the change in plain English. Your teammate proposes the new playbook, you approve, it ships." |
| 2 | `web/app/landing/page.tsx:58` | "Forensic-grade audit chain on every action. **Capability-based RBAC**. Encryption-at-rest by default." | "Forensic-grade audit chain on every action. Fine-grained permissions on every action. Encryption-at-rest by default." |
| 3 | `web/app/pricing/page.tsx:105` | (Feature matrix row) "**Capability-based RBAC**" | "Fine-grained permissions (who can do what)" |
| 4 | `web/app/pricing/page.tsx:133` | "Every primitive (audit row, skill execution, **MCP call**, memory key) is keyed by **tenant_id** and CI guards enforce it." | "Every record (audit row, skill run, connector call, memory entry) carries your workspace ID and automated checks enforce it." |
| 5 | `web/app/docs/page.tsx:66` | "Forensic-grade audit chain on every action. **Capability-based RBAC**, **tenant isolation**, SOC 2 evidence, kill-switch ops." | "Forensic-grade audit chain on every action. Fine-grained permissions, workspace separation, audit evidence bundle, one-tap kill-switch." |
| 6 | `web/app/(app)/workflows/page.tsx:66` | (Card description) "**BANT**-style qualification + calendar-link insertion + CRM update." | "Lead qualification + calendar-link insertion + CRM update." (or expand: "BANT (Budget, Authority, Need, Timeline) qualification…") |
| 7 | `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:95,109,111,116` | "**BANT**-style qualification", "Enrich + **BANT** score", "**BANT**-scores 0-100", "Branch on **BANT** score" | Either expand once at top ("BANT — Budget · Authority · Need · Timeline — qualification") or rename to "qualification score". |
| 8 | `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:348` | "**BrowserExecutor** opens FB Marketplace, fills the form…" | "Your teammate's browser opens Facebook Marketplace, fills the form…" |
| 9 | `web/app/(app)/workers/[workerId]/responsibilities/new/page.tsx:76` | "We'll draft a workflow **TOML** for you to approve, edit, or cancel." | "We'll draft a workflow recipe for you to approve, edit, or cancel." (TOML is the file format — irrelevant to the user.) |
| 10 | `web/app/(app)/brand/components/page.tsx:104` | (Enterprise feature chip) "SSO + **SCIM**" | "SSO + auto-provision users (SCIM)" |
| 11 | `web/app/(app)/brand/components/page.tsx:173` | "Core platform — runtime / **RBAC** / audit" | "Core platform — runtime / permissions / audit" |
| 12 | `web/app/(app)/brand/components/page.tsx:177` | "Providers + **MCP** connectors" | "Providers + connectors" (MCP is internal implementation detail). |
| 13 | `web/app/docs/page.tsx:630-631` | "Operator guides, **API (Application Programming Interface — the contract programs use to talk to SKYCOT)** reference…" | Expansion is too long for a hero. Replace with "Operator guides, developer reference, workflow recipes, and integration docs." with "API reference" written out properly in the card grid below where the technical audience expects it. |
| 14 | `web/app/changelog/page.tsx:66` | "**Capability-based RBAC** is now visible end-to-end…" | "Fine-grained permissions are now visible end-to-end…" |
| 15 | `web/app/(setup)/onboarding/connect-provider/page.tsx:145` | "Skipping uses a mock provider. **Workers** will run, but responses are simulated until you add a real key in Settings." | "Skipping uses a mock provider. Your teammates will run, but their replies are simulated until you add a real key in Settings." |

**Acceptable as-is** (technical audience or already plain): API route handler `JSON` references (developer-facing), `regex` in a comment, `manifest.json` in the data-export page (the file users download is literally `manifest.json`), and the `dh-hero-sub` "RSS (Really Simple Syndication — …)" expansion which follows the operator rule.

**Single fix that covers ~half the hits:** Replace "Capability-based RBAC" with "Fine-grained permissions" everywhere it appears in customer copy (occurrences: landing, pricing matrix, docs blurb, changelog entry, compare table row).

---

## 2. WORKERS vs TEAMMATES VOICE

Operator-locked positioning: workers are **teammates**. The audit found **12 customer-visible instances** of `worker(s)` where `teammate(s)` is the brand-correct noun. Technical code (testids, query selectors, `useMemo`'d filter arrays, `Worker` type imports) is unaffected and was filtered out.

### Top hits to fix

| # | File:line | Current copy | Fix |
|---|---|---|---|
| 1 | `web/app/landing/page.tsx:805,830,1037` | (Header pill + 3 hero/final CTAs) "Hire your first **worker**" | "Hire your first **teammate**" — repeat 3× |
| 2 | `web/app/landing/page.tsx:824` | "SKYCOT compiles plain-English job descriptions into AI **workers** that live in Slack…" | "…into AI **teammates** that live in Slack…" |
| 3 | `web/app/landing/page.tsx:28` | (Feature card) "Each AI **worker** has a name, a personality, and a job." | "Each AI **teammate** has a name, a personality, and a job." |
| 4 | `web/app/landing/page.tsx:109` | "SKYCOT spins up a named **worker** (Priya, Jamal, Mariah, etc.)…" | "SKYCOT spins up a named **teammate**…" |
| 5 | `web/app/landing/page.tsx:121` | "Every action, every approval, every decision — fully audit-logged with **worker**, action, cost, latency, and outcome." | "…with **teammate**, action, cost, latency, and outcome." |
| 6 | `web/app/landing/layout.tsx:7` | (Page metadata) "AI **workers** that live in Slack, ask before they act…" | "AI **teammates** that live in Slack…" (also fixes OG description) |
| 7 | `web/app/pricing/page.tsx:693` | (Header CTA pill) "Hire your first **worker**" | "Hire your first **teammate**" |
| 8 | `web/app/customers/page.tsx:562` | (Header CTA pill) "Hire your first **worker**" | "Hire your first **teammate**" |
| 9 | `web/app/docs/page.tsx:617` | (Header CTA pill) "Hire your first **worker**" | "Hire your first **teammate**" |
| 10 | `web/app/docs/page.tsx:107` | (Featured guide title) "How the trust ratchet promotes **workers**" | "How the trust ratchet promotes **teammates**" |
| 11 | `web/app/changelog/page.tsx:58,72,128,136,152` | 5 entry bodies mixing "AI worker's playbook" / "personality cards on worker profiles" / "with worker, tenant, capability…" / "worker proposes" / "freeze any worker" | All → **teammate** (entry on personality cards even says "personality cards on worker profiles" then "AI teammate" in the body — internally inconsistent) |
| 12 | `web/app/(app)/billing/page.tsx:46,55,64` | (Tier feature chips) "1 worker", "5 workers", "Unlimited workers" | "1 teammate", "5 teammates", "Unlimited teammates" (pricing page already says "teammates" — billing is now divergent) |
| 13 | `web/app/(setup)/onboarding/welcome/page.tsx:26` + `first-hire/page.tsx:66` | "Let's hire your first **worker**." / "Describe your first **worker**" | "Let's hire your first **teammate**." / "Describe your first **teammate**" |

**The most damaging:** landing page hero sub-head, layout metadata, and 4 CTA pills all say "worker". This is the very first 10 seconds of any prospect's experience, and it directly contradicts the H1 ("Hire a teammate"). The contradiction is visible inside a single fold.

**Acceptable as-is:** technical worker filter labels (`/audit` filter chip `worker`, `/orchestration` `Worker` filter column header) — these are operator-debug surfaces, the `worker=` URL parameter is a stable API. Also acceptable: `useMemo`s, type imports, internal field names.

---

## 3. HEADLINE QUALITY

Scored 1-10 across (Specific · Promise-forward · Brand-on · Under 12 words · Backed by a sub-head). Pages on the demo path scored:

| Page | H1 | Score | Notes |
|---|---|---|---|
| `/landing` | "Don't buy another tool. Hire a teammate. (Seriously.)" | **9/10** | Specific, promise-forward, brand-on, 9 words. The "(Seriously.)" parenthetical is the brand-signature moment. Sub-head spoiled by "workers" + "single-binary" jargon. |
| `/pricing` | "Hire teammates. Pay like they're real." | **8/10** | Punchy, brand-on, 7 words. Sub-head is correct. |
| `/compare` | "A teammate isn't a workflow. It's the thing the workflow serves." | **8/10** | Conceptual, specific, 11 words. Bit philosophical for first-touch; works for the comparison context. |
| `/customers` | "Teams shipping with SKYCOT." | **6/10** | Generic. "Shipping" is brand-on. Doesn't promise what the visitor gets. Try: "300+ teams. 14 verticals. One audit chain." |
| `/changelog` | "What we shipped this week." | **7/10** | Specific + temporal hook is good. Brand-restrained register matches the page. |
| `/docs` | "Everything you need to ship." | **6/10** | Generic — could be any product. Try: "Operator guides. API reference. Recipes you can clone." |
| `/skills` | "Every skill your team will ever need." | **7/10** | Promise-forward but slightly grandiose. "Every skill" overpromises. |
| `/workflows` | "How your teammates actually think." | **8/10** | Brand-on, specific, intriguing. |
| `/workflows/templates` | "Don't start from a blank canvas." | **8/10** | Promise-forward (you skip the hard work), brand-on. |
| `/team` | (in-app) "Team" | **N/A** | Functional label, not a marketing headline. Brand-loud hero ("Inbox zero." / "N decisions need you.") is dynamic and brand-on. |
| `/audit` | "Audit" + sub "Forensic + compliance view" | **5/10** | Functional. Acceptable as in-app, but sub-head says "Forensic + compliance view" which is operator-jargon ("forensic" is a tone-on-brand word, "compliance view" is fine, but the sub doesn't say what the user gets to do here). |
| `/auth/signup` | "Hire your first AI teammate." | **9/10** | Specific, promise-forward, brand-on. Sub-head also clean. |
| `/auth/signin` | "Sign in to SKYCOT." | **6/10** | Functional. Acceptable for return-user flow. |
| `/onboarding/welcome` | "Welcome to SKYCOT." | **4/10** | **Weakest headline on the demo path.** Generic, promises nothing, says only that you arrived. Sub-head is the actual hook ("Let's hire your first worker. (90 seconds.)") — the H1 and sub should swap roles. Try: "Hire your first teammate." with sub "It takes 90 seconds." |
| `/onboarding/workspace` | "Name your workspace" | **6/10** | Functional. Acceptable mid-flow. |
| `/onboarding/pick-vertical` | "Pick a starting template" | **6/10** | Functional. Acceptable. |
| `/onboarding/connect-provider` | "Connect Anthropic" | **6/10** | Functional. Acceptable. |
| `/onboarding/first-hire` | "Describe your first worker" | **5/10** | Functional + breaks workers-voice rule. Should be "Describe your first teammate." |
| `/hire` (wizard step 1) | "Build your AI workforce." | **8/10** | Brand-loud, Audiowide hero, promise-forward. |
| `/hire/step/2..5` | "A few questions." / "Crafting your teammate." / "Meet your first teammate." / "Connect a channel." | **8/10 avg** | Consistent F1 editorial-center voice. |
| `/hire/done` | "You just hired your first teammate." | **10/10** | Brand peak — earned celebration moment. |

**Worst headlines on the demo path:**
1. `/onboarding/welcome` "Welcome to SKYCOT." (4/10) — landing here mid-flow should reuse the marketing promise, not state a fact.
2. `/onboarding/first-hire` "Describe your first worker" (5/10) — wrong noun + functional tone breaks the F1 hero family.
3. `/audit` sub-head "Forensic + compliance view" (5/10) — operator-jargon adjacent.

---

## 4. CTA CLARITY

The audit found one literal "Get started" CTA on the brand-storybook page (not user-facing), zero "Learn more" / "Click here" / "Read more" instances, and **zero vague CTAs on the customer-facing demo path**. Most CTAs are operator-locked to "Hire your first teammate" or task-specific ("Email support", "Validate & continue", "Hire them", "Continue").

### Demo-path CTA scorecard

| Page | Primary CTA | Score | Notes |
|---|---|---|---|
| `/landing` hero | "Hire your first worker" → noun fix to "teammate" | 9/10 | Strong verb, specific outcome. |
| `/landing` hero secondary | "Watch a 90-second demo" | **6/10** | **Weakest on the demo path.** The verb "watch" is fine, but the destination is `href="#how"` — an anchor to the "How it works" text section, not an actual demo video. CTA promises a video, link delivers a 4-step text block. Either: (a) wire up a real demo video, or (b) rename to "See how it works" / "Read how it works" — matches the actual destination. |
| `/landing` compare-band | "See the comparison →" | 8/10 | Specific. |
| `/landing` final | "Hire your first worker →" | 9/10 (after noun fix) | Strong. |
| `/pricing` tier-Free | "Start free" | 8/10 | Specific outcome. |
| `/pricing` tier-Pro | "Upgrade to Pro" | 8/10 | Specific. |
| `/pricing` tier-Enterprise | "Contact sales" | 7/10 | Functional; could be "Talk to a human" or "Get a custom quote" for brand-warmth, but acceptable. |
| `/pricing` final | "Start free →" | 8/10 | Specific. |
| `/customers` final | "Email hello@skycot.com →" | 7/10 | Functional; the link target IS the action. |
| `/changelog` header | "Get notified →" | 7/10 | Specific outcome. |
| `/docs` help band | "Email support" + "See what we shipped →" | 8/10 | Both specific. |
| `/compare` footer-row | "Hire your first teammate →" | 10/10 | Best CTA in the codebase — context-specific, brand-on, promise-forward. |
| `/onboarding/welcome` | "Continue →" | 6/10 | Functional. Could be "Name my workspace →" to preview what's next. |
| `/onboarding/first-hire` | "Hire them" | 9/10 | Brand peak — emotionally loaded, specific. |
| `/hire/step/[step]` | "Continue" | 6/10 | Functional. Acceptable mid-wizard. |
| `/hire` (step 1) | "Continue" | 6/10 | Acceptable. Could be "Meet my teammate →" but the wizard's pace works. |
| `/auth/signup` | (form submit) | 8/10 | Email-magic-link flow, button copy not audited but `Validate & continue` style elsewhere is acceptable. |

**The one CTA worth changing today:** `/landing` line 832 `<a href="#how" className="land-btn-secondary">Watch a 90-second demo</a>` — there is no demo video at `#how`. Either ship the video or rename. Recommended rename: `See how it works →`.

---

## 5. TRUST + SAFETY OVER-CLAIM

**This is the highest-severity finding in this audit.** SKYCOT advertises 4 compliance attestations (SOC 2 Type II, ISO 27001, HIPAA, AFSL) as held certifications across landing, pricing, customers, and shared wizard/auth shells. The operator-locked truth (per `2026-05-14 v1 strategic decisions` and the trust+safety audit referenced in the prompt) is:

- **SOC 2 Type II:** not yet attested. SKYCOT has a SOC 2-ready audit chain, but no Type II report.
- **ISO 27001:** not yet certified. Controls are supported, evidence bundle exists.
- **HIPAA:** not a software certification (it's a contractual / workflow posture). SKYCOT is HIPAA-workflow-ready, not "HIPAA certified".
- **AFSL (Australian Financial Services Licence):** an *operator* licence, not a *software* certification. SKYCOT cannot hold an AFSL — only the customer can. Mentioning it as a SKYCOT capability is a category error.

### Every line to soften (file:line + recommended new copy)

| # | File:line | Current line | Recommended copy |
|---|---|---|---|
| 1 | `web/app/landing/page.tsx:843` | `SOC 2 TYPE II` (trust strip pill) | `SOC 2-READY AUDIT TRAIL` |
| 2 | `web/app/landing/page.tsx:847` | `ISO 27001` | `ISO 27001 CONTROLS SUPPORTED` |
| 3 | `web/app/landing/page.tsx:851` | `GDPR · HIPAA · AFSL` | `GDPR · HIPAA WORKFLOW-READY` (drop AFSL entirely) |
| 4 | `web/app/pricing/page.tsx:99` | `{ label: "SOC 2 Type II", free: true, pro: true, enterprise: true },` | `{ label: "SOC 2-ready audit trail", free: true, pro: true, enterprise: true },` |
| 5 | `web/app/pricing/page.tsx:101` | `{ label: "HIPAA", free: false, pro: true, enterprise: true },` | `{ label: "HIPAA workflow-ready", free: false, pro: true, enterprise: true },` |
| 6 | `web/app/pricing/page.tsx:102` | `{ label: "ISO 27001", free: false, pro: true, enterprise: true },` | `{ label: "ISO 27001 controls supported", free: false, pro: true, enterprise: true },` |
| 7 | `web/app/pricing/page.tsx:103` | `{ label: "AFSL (Australian Financial Services)", free: false, pro: false, enterprise: true },` | **DELETE THIS ROW.** AFSL is an operator-licensed permission, not a software feature. Replace with something SKYCOT can legitimately deliver: `{ label: "AFSL-compatible audit evidence", free: false, pro: false, enterprise: true }` — but only if the audit evidence has actually been reviewed against ASIC RG 105/146 by a compliance counsel. Safest action: delete the row. |
| 8 | `web/app/customers/page.tsx:91` | (Beacon Health Ops testimonial) `"HIPAA-compliant out of the box."` | `"HIPAA-workflow-ready out of the box."` (a customer can still say their *workflow* is compliant; SKYCOT-the-software is the workflow scaffold, not the certification holder) |
| 9 | `web/app/customers/page.tsx:103-104` | (Maple Finance testimonial) `"85 employees · AFSL holder"` … `"AFSL compliance was the deal-breaker."` | The customer can legitimately be an AFSL holder — keep `"AFSL holder"`. The quote is the issue: `"AFSL-compatible audit evidence was the deal-breaker. SKYCOT was the only platform that shipped with the audit chain regulators actually want."` (the company holds the AFSL; SKYCOT supplies the evidence trail) |
| 10 | `web/app/docs/page.tsx:66` | `"SOC 2 evidence, kill-switch ops."` | `"SOC 2-ready evidence bundle, kill-switch ops."` (already softer than "SOC 2 certified" but should be explicit about "ready" vs "attested") |
| 11 | `web/components/auth/AuthEditorialShell.tsx:396` | (Trust foot strip) `<span>SOC 2 TYPE II</span>` | `<span>SOC 2-READY AUDIT TRAIL</span>` — this surface shows on signup/verify/signin |
| 12 | `web/components/auth/WizardEditorialShell.tsx:418` | (Wizard foot) `SOC 2 TYPE II` | `SOC 2-READY AUDIT TRAIL` — this surface shows during all hire-wizard steps |
| 13 | `web/components/skycot/audit/ExportFormatSelector.tsx:13` | (component doc comment) `"SOC2 Type II default…"` | Acceptable — this is an internal code comment, not user-visible string. **No change needed.** |

**Customer-facing string count to soften:** 11 lines (12 if you delete the AFSL feature row, 10 if you only soften it).

**Customer-facing string count to keep as-is:** GDPR mentions (it's a regulation users comply with, not a certification SKYCOT must hold — SKYCOT can be "GDPR-compliant" because GDPR governs data handling, which is verifiable behavior). Also keep: any "audit chain that regulators want" / "evidence bundle" / "controls supported" framing.

---

## Suggested batch fix

If the operator wants a single PR to clear the top-3 risks:

1. **Trust over-claim sweep (11 strings across 5 files).** Highest legal risk.
2. **Workers → teammates noun swap (12 customer-visible strings across 6 files).** Brand-voice rule violation, fixed in 20 minutes with find-and-replace bracketed by context-checks.
3. **Capability-based RBAC → Fine-grained permissions (5 strings).** Single jargon term, customer-visible everywhere.

Estimated total fix-scope: ~30 strings, single PR, no functional code touched.

---

## Out-of-scope notes (flagged but not actioned)

- The `/landing` "single-binary, enterprise-grade" jargon trio in the sub-head — Mark CEO is not a developer; "single-binary" means nothing to the buyer. Defer to a brand pass.
- The `/audit` page title "Audit" is functional. Brand-loud hero treatment exists elsewhere — the audit page chose the brand-restrained register intentionally per operator memory `project_audit_page_design_locked.md`. Leave as-is.
- The Audiowide-vs-Inter conflict noted in the operator's memory (`project_brand_consistency_pass.md` says D3 brand-consistency pass applied Audiowide hero typography to all 13 in-product files, but several locked design notes say "Inter only / no Audiowide") is a brand-typography question, not a copy question. Out of scope.
- The pricing FAQ entry "Is data shared between tenants?" expansion (line 133) is the longest dev-jargon hit and might warrant a complete rewrite rather than a word-swap. Out of scope for this audit; flagged for the brand/voice round.

---

**End of report.**
