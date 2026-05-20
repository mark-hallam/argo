# Wave-1 Mission Alignment Audit — 2026-05-20

**Auditor:** Wave-1 audit agent #3 (mission alignment)
**Mission target (per `docs/skycot/MISSION.md` + `CLAUDE.md`):**
> "AI teammates that live in Slack, learn from your feedback, and ask for approval before they act. Audit-first. Single-binary. Enterprise-grade."

**Scoring rubric (per pillar, 1-10):**
- 1-3 = absent or actively contradicts the pillar
- 4-6 = mentioned but weak/decorative; not load-bearing
- 7-8 = clearly delivers the pillar with specifics
- 9-10 = the surface IS the pillar — would convince a sceptical buyer

**Pillars:**
1. TEAMMATES — named colleagues, personality, avatar, voice
2. APPROVAL-FIRST — gates visible, "ask before act", trust ratchet
3. AUDIT-FIRST — every action logged, hash chain, evidence export
4. ENTERPRISE-GRADE — SOC 2 / HIPAA / RBAC / tenant isolation surfaced
5. SINGLE BINARY — self-host / no-Kubernetes / download-and-run
6. VIRAL / EPIC UX — moments you'd screenshot

---

## /landing — `web/app/landing/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 9 | Hero H1 "Don't buy another tool. Hire a teammate. (Seriously.)" (L818-821). Named workers in hero art bubbles — Mariah, Priya, Jamal, Nori, each with a verb-of-the-day (L862-872). Feature card 1: "Named teammates, not tools" (L26). HOW step 02: "SKYCOT spins up a named worker (Priya, Jamal, Mariah, etc.) with the right skills, joins your Slack, and introduces themselves" (L109). Testimonials reference Mariah by name (L79). |
| APPROVAL-FIRST | 8 | Feature card "Asks before it acts. Every risky action waits for your yes. Approval-first by default. Trust ratchets up over time" (L33-35). HOW step 03 "Approve their first decisions" + "trust grade rises" + "earn full autonomy" (L113-115). Testimonial "Approval-first was the deal-breaker" (L86). |
| AUDIT-FIRST | 7 | Feature card "Audit-first, enterprise-grade. Forensic-grade audit chain on every action" (L56-58). HOW step 04 "Watch the audit chain. Every action, every approval, every decision — fully audit-logged with worker, action, cost, latency, and outcome" (L119-122). Testimonial cites "Every action audit-logged" (L86). Could be louder — no visual of the actual chain. |
| ENTERPRISE-GRADE | 9 | Trust strip in hero: "NO CREDIT CARD · SOC 2 TYPE II · ISO 27001 · GDPR · HIPAA · AFSL · 2,847 teams this month" (L838-856). Sub-headline names it: "Audit-first, single-binary, enterprise-grade" (L826). Customer logos (Northwind, Beacon Health, Maple Finance, Harbour Legal) read enterprise (L62-75). |
| SINGLE BINARY | 5 | Sub-headline mentions "single-binary" (L826) but it's buried. No dedicated section explaining what that means or why it matters to a buyer. Not a load-bearing claim on this page. |
| VIRAL / EPIC UX | 9 | Audiowide hero font + purple→pink gradient + animated radial blobs + 4 chat-bubble worker stat cards + animated compare-band shine. "Don't buy another tool. Hire a teammate. (Seriously.)" reads like a billboard. The compare-band ribbon ("8 of 13 rows are clean wins") would be screenshotted. |

**Average:** 7.8
**Verdict:** Best-in-class for mission delivery. Reads exactly like the mission statement. Only weakness: the single-binary differentiator is underplayed against more abstract competitors that *can't* be self-hosted.

---

## / (Approvals inbox) — `web/app/(app)/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 7 | Hero copy: "Inbox zero / 1 decision needs you / N decisions need you" + "Your AI teammates have batched these for your review" (L520, L524-527). Subtitle "Your AI teammates need a yes or no" (L399). Each `ApprovalCard` shows worker name + initials/avatar (per `ApprovalCard.tsx:30`). But the inbox surface itself is task-shaped, not person-shaped — workers appear as a header on each card, not as the primary unit. |
| APPROVAL-FIRST | 10 | This IS the approval gate. Every primitive on the page enforces the pillar: pending count pill (L394), "N waiting", trust ratchet section (L258, `<TrustRatchetSection>`), bundle cards for batched approvals (L279-291), three-action approval card (per `ApprovalCard.tsx:21-23` Approve · Refuse · Why?), 30-second undo per `onUndo` callback (L161-164), batch undo (L213-225). The whole surface IS "worker proposes → you approve". |
| AUDIT-FIRST | 6 | Decisions flow through `submitDecision` mutation, which presumably writes audit rows — but the inbox itself doesn't surface the audit chain visibly. Redaction policy is enforced via `useRedactionPolicy()` (L80) + `redactPII` (L148) + `RequestElevatedView` (L253). The elevated-view CTA is the closest the inbox comes to "audit-first visible". Could explicitly show "this decision will be hash-logged" inline on the card. |
| ENTERPRISE-GRADE | 7 | Redaction-by-policy means PII never leaks to viewers without RBAC (L142-154). `RequestElevatedView` (L23) is the load-bearing enterprise primitive. `DataSourceChip` (L18) + `LiveIndicator` (L21) signal production-grade infrastructure. Filters cover worker/channel/status — but RBAC chip, tenant context, and SSO surface are not shown. |
| SINGLE BINARY | 2 | Zero mention. The page assumes the binary; never explains it. |
| VIRAL / EPIC UX | 8 | `<BrandLoudHero>` with eyebrow dot pulse + "DECISIONS NEED YOU NOW" + animated shine (L529-545). 3 KPIs (Waiting / Resolved/wk / Avg response). "Your AI teammates need a yes or no" is a quotable line. The inbox-zero empty state ("Inbox zero. Your AI teammates have it handled.") is the screenshot moment. |

**Average:** 6.7
**Verdict:** The pillar this surface OWNS (approval-first) scores a 10. But teammates/audit/enterprise-grade are present as decoration not as the load-bearing surface they should be. Audit chain visibility on the approval card itself is a gap.

---

## /team — `web/app/(app)/team/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 10 | The entire surface is the teammates pillar made visible. Hero "Hire your first teammate / Your first teammate is working / N teammates, one team" (L402-405). KPI strip: Working / Actions/wk / Hours saved (L421-424). Sub: "X working right now · Y actions this week · saving you ~Z hours" (L408-409). `<TeammateCard>` per worker, each with avatar + status + tagline + today's bullets + tomorrow (per `TeammateCard.tsx:161-209`). "teammate(s) working" copy throughout (L225). Sticky `Hire` CTA. |
| APPROVAL-FIRST | 5 | `WORKER_STATUSES` includes "waiting" (per `team/page.tsx:29`) with label "Waiting on you" (per `TeammateCard.tsx:32`). Status pill on cards is the surface signal that workers ask. But it's a chip, not a flow — the "ask before it acts" idea is not surfaced explicitly on this page. |
| AUDIT-FIRST | 3 | Not surfaced. Cards link to `/workers/<id>` for profile drill-in (`TeammateCard.tsx:60`), where audit lives separately. The team page itself does not mention or hint at the audit chain. |
| ENTERPRISE-GRADE | 3 | No SOC 2 / RBAC / tenant chips on this surface. Status filter + search are the only chrome. The "Hire" button assumes single-tenant context. |
| SINGLE BINARY | 1 | Not surfaced. |
| VIRAL / EPIC UX | 9 | `BrandLoudHero` with "YOUR AI WORKFORCE · LIVE" eyebrow + pulse dot + animated shine (L411-429). C2 holographic-showcase design language per the design-lock memory. The "X teammates, one team" hero is poster copy. Trading-card teammate grid would absolutely be screenshotted. |

**Average:** 5.2
**Verdict:** Owns the teammates pillar (10/10) and viral UX (9/10) but leaves the other 4 pillars on the table. Critical gap: no visible "asks before acts" pattern at the team-roster level — a buyer scrolling here sees a roster of names but not the safety mechanism that makes them trustworthy.

---

## /workers/[workerId]/overview — `web/app/(app)/workers/[workerId]/overview/page.tsx` + `WorkerPersonalityCard.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 10 | `WorkerPersonalityCard` is the surface that proves the mission. Trait chips ("Patient", "Detail-oriented", "Picture-perfect listings") per worker (`WorkerPersonalityCard.tsx:36-45`). Per-worker "Great at" / "Escalate when" / "Signature behaviors" lists (L38-54). Worker-name-keyed personality map (Mariah, Priya, Jamal). "HOW [NAME] WORKS" eyebrow with Heart icon (L220). Gradient stripe at top of card (L142-144). The page reads "this teammate is a person", verbatim per the C2-personality lock. |
| APPROVAL-FIRST | 7 | "Escalate when" list is the visible articulation of the approval pillar in worker-voice (L43-49). E.g. Mariah escalates when "Multiple competing offers on the same item" (L49). The pattern is on every personality card. But the actual approval *flow* sits in `/` (inbox) — this page describes intent, doesn't enforce it. |
| AUDIT-FIRST | 4 | The overview page renders `<WorkerOverviewContent>` (`page.tsx:27`) which includes `RecentActivityCard` (`WorkerOverviewContent.tsx:14-17`) and links into `/audit` for forensic drill-in. The personality card itself does not mention audit. |
| ENTERPRISE-GRADE | 4 | Not surfaced on this page. RBAC + tenant context lives in surrounding chrome (AppShell), not on the personality card. |
| SINGLE BINARY | 1 | Not surfaced. |
| VIRAL / EPIC UX | 10 | This is the page Mark would tweet. "Trait chips + great-at + escalate-when + signature behaviors" reads like a LinkedIn profile of an AI colleague — the exact viral hook ("look at my AI hire's profile"). The gradient stripe top + heart icon + structured colleague-language is brand-loud-warm done right. Per the design-lock memory: "your AI hires are colleagues" framing — distinctive personality language. |

**Average:** 6.0
**Verdict:** Carries the teammates + viral pillars at 10/10 — best individual surface for those. But it's a profile, not a control panel, so approval/audit/enterprise pillars sit elsewhere. The personality data is hardcoded for 3 workers (Mariah, Priya, Jamal); defaults degrade gracefully but lose the personality oomph for any other name. **Highest viral payoff in the whole app.**

---

## /workflows — `web/app/(app)/workflows/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 7 | Hero "How your teammates actually think" (L427-430). Each workflow card shows `lastEditor` as a worker name (Mariah, Priya, Jamal, Nori, Rio, Sam) (L29-99). Sub "Workflows are the plain-English step-by-step playbooks your AI teammates follow" (L433-435). Workers tie into the workflow ownership. |
| APPROVAL-FIRST | 9 | Risk badges on every card — "Approval required" (amber) vs "Autonomous" (teal) (L106-114, L487-490). 6 of 8 seed workflows are approval-required (L21-101). This is the visible commitment that workflows themselves carry their approval semantics, not just runtime gates. |
| AUDIT-FIRST | 5 | Versioned workflows visible (v1, v2, v3 chips on cards, L504). "Edit them in chat, version them like code" (L435). Versioning is sibling to audit but not the same. No explicit hash/log mention on this page. |
| ENTERPRISE-GRADE | 4 | "WORKFLOWS · LIVE" eyebrow + step pill grid + monospace metadata creates an enterprise-feel aesthetic, but no actual enterprise signals (SOC 2, RBAC chips, tenant context) on this surface. |
| SINGLE BINARY | 1 | Not surfaced. |
| VIRAL / EPIC UX | 9 | Brand-loud-marketplace recipe: hero search bar with gradient icon + ⌘K + count pill + try chips + step pill animation per card (L289-419). Audiowide "How your teammates actually think" + purple→pink "think" gradient (L426-430). Risk color-coding (amber/teal) gives the grid visual rhythm. |

**Average:** 5.8
**Verdict:** Strong on approval (the workflow IS the approval flow) and viral UX. The "Approval required" / "Autonomous" badge across the catalog is a quiet win — it surfaces the pillar where buyers will be browsing. Could surface audit lineage / hash chain on each card to lock the audit pillar.

---

## /workflows/.../build — `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 4 | The builder is workflow-shaped, not teammate-shaped. No worker name/avatar/voice on the canvas. The header shows the workflow name and vertical tag only. Indirectly via `lastEditor` if the data binds back. |
| APPROVAL-FIRST | 9 | Approval-gate is a first-class step kind: `ShieldCheck` icon, amber color, "GATE · Slack" inline ribbon on the node (L116, L189-205). Step node treats approval as its own visual class. Side panel shows the approval gate's channel + reason explicitly (L644-660). The AI-prompt suggested chip "Convert this to fully autonomous (no approval needed)" (L579-580) treats removing-the-gate as a deliberate, prompted-action. |
| AUDIT-FIRST | 3 | Not surfaced on the build canvas. Versioning visible (`workflow.version`, L539) but no audit chain reference. |
| ENTERPRISE-GRADE | 4 | "Save + publish" implies promotion gates but the page doesn't surface RBAC / approver / change-audit. The mini-map + brand chrome read pro, not specifically enterprise. |
| SINGLE BINARY | 1 | Not surfaced. |
| VIRAL / EPIC UX | 10 | This is the operator's competitive moat-shot. React Flow canvas + per-step-kind gradient icons + animated edges + the AI prompt bar at the top with "Sparkles" icon + "Propose" button + suggested prompt chips ("Add a step that emails the seller before posting") — this is the n8n-killer screenshot. Mini-map + draggable nodes + side panel + add-step floating button. Per the file header docstring (L33-58): "the SKYCOT-class-leading differentiator". |

**Average:** 5.2
**Verdict:** Best UX moment in the app for differentiation against Zapier/n8n; second-best viral payoff after the worker personality card. But the surface is so workflow-centric that the teammate pillar nearly vanishes. The AI-prompt-edit chip is a 10/10 viral moment that needs more polish on the response side (the stub just timeouts at L307-311).

---

## /audit — `web/app/(app)/audit/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 5 | Every audit row carries `row.worker.name` + initials avatar (`AuditDayGroup.tsx:63-68`). Worker is a filter dimension (`audit/page.tsx:171`). But the surface is forensic-first, person-second — workers are columns in a log, not colleagues. |
| APPROVAL-FIRST | 7 | Approver is a filter dimension (`audit/page.tsx:172`). Status filter includes "succeeded / refused / pending / failed / autonomous" (per `audit/page.tsx:5-6`, `audit-types`). The audit shows the *outcome* of approval flows, including refused and autonomous decisions. Strong but indirect. |
| AUDIT-FIRST | 10 | This IS the audit pillar. Cross-worker forensic timeline (L52). 7 filter dimensions: worker, channel, actionType, status, approver, costBand, classification (`filters/audit.ts`). Range presets (`AUDIT_RANGE_PRESETS`). Search by free-text. Evidence-bundle export dialog (`EvidenceBundleExporter`, L15, L254-263). Redaction-by-RBAC via `useViewerPolicy` + `redactAuditRow` + `RedactionBanner` (L19-21, L154-162, L202-206). Provenance & hash chain visible on the detail page (`ProvenanceBlock` mentioned in audit-detail). Etherscan-style is locked design per memory. |
| ENTERPRISE-GRADE | 9 | Capability-based RBAC redaction (`categoriesLackedBy`, L167-170). Evidence-bundle exporter — the auditor-friendly artifact. Tenant-scoped (`tenantId="tenant_default"`, L258). Pagination + `Load more page X of Y` for forensic completeness. Filter chip rail surfaces every dimension a compliance officer wants. |
| SINGLE BINARY | 2 | Not surfaced. |
| VIRAL / EPIC UX | 7 | Per the locked design memory ("B3-enterprise: Etherscan-style + Inter + pop-out modal"): brand-restrained register, gradient sparingly, Inter throughout. Less viral than `/team` or `/workflows/build`, deliberately. The export-evidence button is the compliance screenshot moment. |

**Average:** 6.7
**Verdict:** Locks the audit pillar at 10/10 and enterprise pillar at 9/10. Deliberately restrained on viral by design (the buyer here is the auditor, not the marketing tweet). The audit is genuinely auditor-grade — this is the surface that wins enterprise procurement.

---

## /compare — `web/app/(app)/compare/page.tsx`

**NOTE: route lives under `(app)`, requires auth (per `middleware.ts:16-29`), but `/landing` links to it as if public (L796, L1027). This is a bug — discovered during audit.**

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 10 | Entire page is the teammates pillar argument vs competitors. Hero "A teammate isn't a workflow. It's the thing the workflow serves" (L375-378). First group is "Teammates, not tools" (L41-62). Caption: "Zapier and n8n call them 'zaps' / 'workflows'. SKYCOT teammates have names, traits, and signature behaviors" (L50). Tagline grid: SKYCOT = "Hire a teammate" vs Zapier = "Connect apps" (L124-130). |
| APPROVAL-FIRST | 10 | Whole second group "Safe by default" — 3 dimensions: ask-for-approval, audit-log, freeze-in-one-click (L64-78). SKYCOT scores "yes" on all 3, competitors mostly "no" or "partial". Caption: "SKYCOT ships forensic-grade audit chain by default. Most competitors offer audit as an enterprise add-on" (L72). |
| AUDIT-FIRST | 9 | "Is every action audit-logged with full lineage?" row — SKYCOT "yes", everyone else "partial" or "no" (L71). Audit is explicitly framed as ship-day default. |
| ENTERPRISE-GRADE | 10 | Whole fourth group "Enterprise-ready" — multi-tenant, RBAC, encryption-at-rest, with SKYCOT at "yes" on all 3 (L90-103). |
| SINGLE BINARY | 10 | Whole fifth group "Deploy how you want" — "Single binary you can self-host?" and "Air-gapped deploys for regulated environments?" (L105-113). SKYCOT vs the SaaS-only competitors. This is the ONLY surface in the audit that surfaces the single-binary pillar at full force. |
| VIRAL / EPIC UX | 8 | Brand-loud-comparison table layout. Positioning row (5 cards, SKYCOT highlighted with gradient) + 13-row scorecard with green-check / red-x / amber-dash icons. The footer band "If 8+ of those rows landed for you, this is your team. Hire your first teammate →" (L414-417) is the conversion-money-shot. |

**Average:** 9.5
**Verdict:** Highest-scoring surface in the audit. All 6 pillars at 8+ and 4 at 10. This is the page that wins the deal. **Critical bug: it's behind the auth gate but linked from the public landing page.** Either move it public or change the landing link. The /compare page IS the marketing page — gating it kills the funnel.

---

## /pricing — `web/app/pricing/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 8 | Hero "Hire teammates. Pay like they're real" (L694-696). Per-teammate billing model: "$29 per teammate / month" (L52-55). FAQ: "On Pro, every named teammate you hire (Mariah, Priya, Jamal, etc.) is $29/month" (L121). |
| APPROVAL-FIRST | 6 | Pro features list "Approval flows + trust ratchet" (L62). Otherwise pricing focuses on what you get per tier, not the approval primitive specifically. |
| AUDIT-FIRST | 8 | Matrix row "Forensic audit chain" — Free/Pro/Enterprise all "yes" (L106). Enterprise features list "Audit export (S3 / GCS / Azure)" (L81). Audit is named explicitly as ship-day-1 in every tier. |
| ENTERPRISE-GRADE | 10 | Feature matrix is the enterprise pillar made shoppable. SOC 2 Type II, GDPR, HIPAA, ISO 27001, AFSL, Encryption-at-rest (SQLCipher), Capability-based RBAC, SSO/SAML (L99-111). FAQ on tenant isolation (L132-134). Custom verticals + private marketplace for Enterprise (L82). |
| SINGLE BINARY | 9 | Enterprise tier explicitly "On-prem / single-binary deploy" (L84). FAQ Q5 "Can I self-host SKYCOT?" with full answer: "SKYCOT ships as a single binary with SQLite + SQLCipher, runs on a Mac mini or any Linux server" (L136-138). Strong commercial surfacing of the single-binary differentiator. |
| VIRAL / EPIC UX | 7 | Brand-loud-marketing — same Audiowide gradient hero + 3-tier card grid (featured ribbon on Pro) + feature matrix table + FAQ accordion + animated gradient final CTA band. Pro tier card with gradient price + "Most popular" ribbon is the screenshot moment. Not as viral as /landing but solid commercial polish. |

**Average:** 8.0
**Verdict:** Excellent enterprise + single-binary delivery. The matrix and FAQ surface every enterprise pillar including SOC 2/HIPAA. The per-teammate price model itself ($29 / named-teammate / month) reinforces "they're real". Could louden the approval-first messaging — currently buried in the Pro features list.

---

## /customers — `web/app/customers/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 10 | 10 customer cards, each anchored on a named worker (Mariah, Patrick, Nori, Priya, Jamal, Tobias, Peta) with hours-saved + actions/mo + vertical (L32-150). Quote-driven, e.g. "Mariah replied to 12,400 buyers our human team didn't have to. She knows when to escalate" (L39). Hero "Teams shipping with SKYCOT" + "300+ teams. From solo founders to Fortune 500. Hear how they're using their teammates" (L571-577). |
| APPROVAL-FIRST | 8 | Multiple customer quotes name approval-first as the reason they bought. "Approval-first is the only way we'd put an AI on client-facing work. Every action gates through a partner" (Harbour Legal, L143). "AFSL compliance was the deal-breaker. SKYCOT was the only platform that shipped with the audit chain regulators actually want" (Maple Finance, L104). |
| AUDIT-FIRST | 8 | Multiple quotes anchor on audit: "every action audit-logged" (Beacon Health, L91), "depositioned-ready" (Harbour Legal, L143), audit chain (Maple Finance, L104). Audit shows as a buying reason, which is more credible than self-promotion. |
| ENTERPRISE-GRADE | 9 | Customer mix is enterprise-shaped: AEC/Construction, Financial Services with AFSL, Healthcare (HIPAA mention), Legal, B2B SaaS, Recruiting. Sizes: 28 partners → 1,200 employees. Industry/size meta on every card (L42-44 etc). HIPAA + AFSL + audit chain called out by name in quotes. |
| SINGLE BINARY | 1 | Not mentioned. No customer talks about self-hosting / on-prem on this page. Missed opportunity for an AFSL or healthcare customer to mention regulated-environment deployment. |
| VIRAL / EPIC UX | 8 | Per-customer gradient logo block (initials, gradient bg unique per card), italic quote with gradient left-border, 3-KPI strip (hrs saved / actions / since), vertical chip footer (L580-611). Customer grid + final shine-animated "Want to be featured?" CTA (L477-498). |

**Average:** 7.3
**Verdict:** Strong delivery on teammates + enterprise + approval, all anchored in customer voice not vendor copy. Single-binary is missed — at least one regulated customer (AFSL or healthcare) should mention on-prem deployment for full pillar coverage.

---

## /skills — `web/app/(app)/skills/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 5 | Sub "Drop in a pre-built skill — approval routing, invoice extraction, candidate screening, and dozens more — and your AI teammates start using it the same shift" (L321-323). Otherwise no teammate names on this surface — skills are presented as the unit, workers consume them off-page. |
| APPROVAL-FIRST | 6 | TRY chip "approval routing" is first in the list (L20). Otherwise the skill metadata (name, description, version, layer) doesn't surface approval semantics per-skill. |
| AUDIT-FIRST | 4 | Skill version chip + layer chip on each card (L401-402). Versioning is sibling to audit. No hash/log surface here. |
| ENTERPRISE-GRADE | 4 | "Marketplace" eyebrow + clean tech-feel chrome. Skills load from `useSkills()` → gateway registry (L51). But no SOC 2/RBAC/tenant signals on this surface. |
| SINGLE BINARY | 1 | Not surfaced. |
| VIRAL / EPIC UX | 9 | Locked A1 marketplace design language per memory. Hero search bar with 28px gradient icon + 18px input + ⌘K + count pill + try-pills (L326-358). Featured skill grid with gradient icon per card (L386-407). Skill ID monospace footer (L404). This is signature SKYCOT chrome. |

**Average:** 4.8
**Verdict:** Beautiful marketplace UX, but mission-pillar-thin. Skills are abstract artifacts — without an owning worker (Mariah uses this skill / Priya's signature skill) the teammates pillar slips. Surface the workers consuming each skill, surface the approval semantics each skill carries, surface the audit lineage, and this jumps to a 7+ average.

---

## /onboarding (Welcome + first-hire) — `web/app/(setup)/onboarding/welcome/page.tsx` + `first-hire/page.tsx` + `pick-vertical/page.tsx`

| Pillar | Score | Evidence |
|---|---|---|
| TEAMMATES | 8 | Welcome: "Let's hire your first worker. (90 seconds.)" (welcome L26). Pick-vertical lists 5 verticals as roles: GTM Engineer, Marketing Intern, Sales SDR, Customer Support, Accounts Receivable (pick-vertical L17-53). First-hire example: "A marketing intern named Maya who drafts blog posts and runs weekly competitive scans" (first-hire L10-11). The wizard literally walks the operator through hiring a named teammate. |
| APPROVAL-FIRST | 4 | Pick-vertical hints at "skills, memory schema, and approval gates" (pick-vertical L106-107). The approval pillar is mentioned but not made visible during onboarding — the operator does not see an example approval gate flow before they finish setup. |
| AUDIT-FIRST | 1 | Not surfaced during onboarding. The new operator never sees the audit chain promise before they hire. |
| ENTERPRISE-GRADE | 2 | Not surfaced. No SOC 2 / SSO / tenant context during onboarding. |
| SINGLE BINARY | 1 | Not surfaced. |
| VIRAL / EPIC UX | 7 | The `/hire` flow (alternative entry at `web/app/(hire)/hire/page.tsx`) uses the locked F1 editorial-center design with Audiowide hero "Build your AI workforce." + 4-minute promise (hire/page.tsx L77-86). Wizard editorial shell with sub-headline "In four minutes, you'll have hired your first AI teammate — and they'll start work as soon as you finish" (hire/page.tsx L82-86). But the bare `/onboarding/welcome` is minimal (38 lines) — does not match the polish of `/hire`. |

**Average:** 3.8
**Verdict:** Worst-scoring surface in the audit by a clear margin. Onboarding misses three of four "Audit-first / Enterprise-grade / Single-binary" pillars entirely. The 4-minute promise lands but the audit-first promise from the landing page is forgotten by the time the operator is picking a vertical. This is the surface most at risk for "marketing promised X, product delivered Y" — the operator is told "Audit-first. Single-binary. Enterprise-grade." on landing, then onboarded with zero mention of any of it. Note: there are two onboarding flows (`/onboarding/*` and `/hire/*`) which is itself confusing.

---

## Summary

| Surface | Avg | Worst pillar |
|---|---|---|
| /compare | 9.5 | (none below 8) |
| /pricing | 8.0 | Approval-first (6) |
| /landing | 7.8 | Single binary (5) |
| /customers | 7.3 | Single binary (1) |
| /audit | 6.7 | Single binary (2) |
| / (approvals) | 6.7 | Single binary (2) |
| /workers/[id]/overview | 6.0 | Single binary (1) |
| /workflows | 5.8 | Single binary (1) |
| /workflows/.../build | 5.2 | Single binary (1) |
| /team | 5.2 | Single binary (1) |
| /skills | 4.8 | Single binary (1) |
| /onboarding | 3.8 | Audit-first (1) |

**Overall mission-delivery score: 6.4 / 10**

**Three best surfaces:**
1. /compare (9.5) — the page that wins deals; all 6 pillars at 8+
2. /pricing (8.0) — enterprise + single-binary pillars land hard
3. /landing (7.8) — public-facing mission delivery

**Three worst surfaces:**
1. /onboarding (3.8) — audit pillar absent
2. /skills (4.8) — single binary absent, approval/audit thin
3. /team (5.2) — single binary absent
   (tied with /workflows-build 5.2 — single binary absent)

**Biggest single mission-gap (close before demo):**

The `/compare` page is gated behind authentication but linked from the public landing page (`landing/page.tsx:796, 1027` + `middleware.ts:16-29` does not include `/compare` in the public allow-list). This is the highest-scoring surface in the entire audit — 9.5/10 across all 6 pillars — and it's invisible to anyone considering signing up. **One-line middleware fix unlocks the strongest mission-delivery surface in the app.**

Secondary gap: the onboarding flow loses 3 of the 4 pillars mid-funnel (audit / enterprise / single-binary). An operator who reads the landing page is promised "Audit-first. Single-binary. Enterprise-grade.", then steps into an onboarding flow that never mentions any of them again. Adding a single "How we keep you safe: audit-logged + approval-gated + tenant-isolated" panel between step 2 (vertical) and step 5 (first hire) would close the trust gap that drives buyer drop-off at the "is this a real product?" decision point.
