# SKYCOT Trust + Safety + Audit-Chain Audit (Wave 1, 2026-05-20)

**Auditor:** Wave-1 audit agent #4 (read-only)
**Scope:** Frontend trust-promise delivery on the surfaces a Diego-persona
(B2B SaaS Sales VP) or a procurement/security reviewer would inspect
during a 30-minute platform walk-through.

**Method:** Inspected every file under `web/app/(app)/` that the trust
promise touches plus the three marketing surfaces (`landing`, `pricing`,
`compare`, `customers`). Cross-checked claims against the operative
mission (`docs/skycot/MISSION.md`) and the design-lock memories
(approvals page = "E ops-dashboard" with 5-KPI strip, audit page =
"Etherscan-style"). Backend audit-chain crates were spot-checked to
confirm marketing claims have substrate behind them.

---

## Headline score

**Overall trust-promise delivery: 6.4 / 10.**

The substrate is genuinely strong (audit chain, provenance, replay,
freeze, RBAC redaction, evidence-bundle export, GDPR export, hash
verifier slot reserved). The drill-in surface (`/audit/action/<id>`)
will impress a security reviewer. The trust-grade per-worker page is
read-only and grown-up. **But the headline surfaces a CFO or procurement
team will see first — the Approvals inbox and the Audit-feed top — fail
to deliver the marketing promise.** Marketing copy on `/landing` and
`/pricing` advertises a 5-KPI ops dashboard, $-at-risk visibility,
confidence donuts, SLA bars, severity tiers, AFSL/HIPAA compliance
badges; none of those are visible on the surfaces a buyer would land
on. The gap is "we built the engine and the engine room is beautiful;
we forgot to put the dashboard on the dashboard."

---

## Surface-by-surface findings

### 1. Approvals Inbox (`/`) — score **5 / 10**

**File:** `~/code/argo/web/app/(app)/page.tsx`
**Card:** `~/code/argo/web/components/skycot/ApprovalCard.tsx`
**Bundle:** `~/code/argo/web/components/skycot/BundleCard.tsx`
**Hero:** `~/code/argo/web/components/skycot/BrandLoudHero.tsx`

| Checklist item                          | Status                  | Evidence |
|-----------------------------------------|-------------------------|----------|
| YES/NO branch shown on each card        | YES                     | `ApprovalCard.tsx:235-248` ("If YES →" / "If NO →") |
| Worker name + verb visible              | YES                     | `ApprovalCard.tsx:182-200` |
| Worker avatar visible                   | YES                     | `ApprovalCard.tsx:183` |
| Channel + relative time chip            | YES                     | `ApprovalCard.tsx:189-198` |
| Bundles (batched approvals) supported   | YES                     | `BundleCard.tsx:49-196` rendered at `page.tsx:280-289`; stack-illusion + count badge + atomic decide |
| Risk score / risk band on card          | **NO**                  | `ApprovalCardProps` lacks any `risk` field (`ApprovalCard.tsx:28-71`); the locked design called for severity-banded sections |
| SLA / time-to-expire shown              | **NO**                  | No `expiresAt` / `slaMinutes` field on the card; only `receivedAt` relative time |
| $ at risk shown                         | **NO**                  | Not present anywhere in `ApprovalCard.tsx`; `BatchGroupBlock.tsx` doesn't carry $ either |
| Confidence shown                        | **NO**                  | No `confidence` field; no donut |
| 5-KPI strip                             | **NO — 3 KPIs, all placeholders** | `page.tsx:512-545` `ApprovalsHero`. KPIs are: `Waiting`, `Resolved/wk` (hardcoded `42`), `Avg response` (hardcoded `"8m"`). The design-lock memory `project_approvals_page_design_locked.md` specifies 5: Over SLA / Oldest pending / $ at risk / Decisions today / Avg time-to-decide |
| 3-tier severity hierarchy (Urgent/Elevated/Normal) | **NO** | Day-grouped (`Today/Yesterday/Earlier`), not severity-banded |
| Bottom stats band (7-day approval rate ring + distribution chart) | **NO** | Absent |
| Trust-ratchet promotion offer in-line   | YES                     | `page.tsx:257-259` mounts `<TrustRatchetSection>` over the cards |
| Reasoning visible (`Why?` button)       | YES                     | `ApprovalCard.tsx:276-292` expands the worker's reason |
| Draft body preview                      | YES                     | `ApprovalCard.tsx:206-233` collapsible draft inspector |
| 30-second undo window after decide      | YES                     | `ApprovalCard.tsx:317-360` post-decision bar with countdown |
| Idempotent batch decisions              | YES                     | `BundleCard.tsx:64-73` single-decision-applies-to-all |
| PII redaction (defense-in-depth)        | YES                     | `redactPII` + `RedactedText` woven through every text field |
| "Request elevated view" affordance      | YES                     | `RequestElevatedView` mounted at `page.tsx:253` |

**Gap summary:** the card and bundle quality is solid for an
operator demo. But the **header strip + the meta-row on each card** are
short by every variable a CFO/procurement team needs to scan a queue
quickly: how many are over SLA, how much money is at risk, how
confident the worker is in its draft. Those gaps would not fail a
security review, but they would fail a Diego-persona "show me how
this scales when my team is processing 200 approvals/day" walkthrough.

**Over-claim risk:** marketing in `customers/page.tsx:91` reads "every
action audit-logged, every approval traced. Our compliance team finally
relaxed." The approval card surfaces no audit-row id, no chain hash,
no `View in audit log →` link from the card. A reviewer who clicks
"approve" and asks "where's the receipt?" gets nothing.

---

### 2. Audit Log (`/audit`) — score **7 / 10** (top page) / **9 / 10** (drill-in)

**Files:**
- `~/code/argo/web/app/(app)/audit/page.tsx`
- `~/code/argo/web/components/skycot/audit/AuditPageHeader.tsx`
- `~/code/argo/web/components/skycot/audit/AuditDayGroup.tsx`

| Checklist item                          | Status | Evidence |
|-----------------------------------------|--------|----------|
| Etherscan-style hash chain (visible per row) | **PARTIAL** | Rows show timestamp / worker / verb / status / cost / copy-id (`ActivityRow.tsx:46-86`). The 8-char hash chain that defines Etherscan style is rendered on **drill-in** (`ProvenanceBlock.tsx`), not on the row itself. A buyer scanning a 50-row feed cannot see the chain integrity at a glance. |
| Row has timestamp                       | YES    | `ActivityRow.tsx:46-53` |
| Row has worker                          | YES    | `ActivityRow.tsx:55-63` |
| Row has skill/action                    | PARTIAL — only `verb` (free-form prose); `actionType` is dropped at `AuditDayGroup.tsx:74-86` (stubbed `"message_sent"`) | `AuditDayGroup.tsx:80-83` |
| Row has status                          | YES    | `ActivityRow.tsx:72-75` |
| Row has cost                            | YES (if non-zero) | `ActivityRow.tsx:76-78` |
| Row has hash                            | **NO** | The hash lives on `AuditDetail`, not `AuditRow` (`types/audit.ts:97-112` lacks any hash field) |
| Row link to detail                      | YES    | `ActivityRow.tsx:29` `href = /audit/action/<id>` |
| Search / filter                         | YES    | `AuditPageHeader.tsx:108-117` search + `AuditFilterPopover` with 7 multi-select dimensions |
| Time-range presets                      | YES (8 presets + Custom stub) | `AuditPageHeader.tsx:124-155` |
| Export button                           | YES    | `EvidenceBundleExporter` mounted at `audit/page.tsx:254-263`; SOC2 Type II default, GDPR Article 15, HIPAA disclosure log formats per `ExportFormatSelector.tsx` |
| Compliance badges (SOC2/ISO 27001/GDPR/HIPAA/AFSL) on audit page | **NO** | Searched `audit/page.tsx`, all components under `audit/` — only "Forensic + compliance view" subtitle. No badge row. |
| RBAC-aware redaction with elevation request | YES    | `RedactionBanner.tsx:1-105` |
| Pagination / cursor                     | YES    | `audit/page.tsx:237-251` load-more with `totalPages` |
| Mock fallback in dev when gateway absent | YES (404 → mocks) | `lib/api/audit.ts:1-78` |

**Drill-in surface (`/audit/action/<id>`)** is the strongest trust
artifact in the entire frontend. It renders:

1. Summary hero with worker + verb + approver + channel + mono action
   id with copy-on-click + Replay + Export (`ActionSummaryHero.tsx`)
2. **Decision-chain card with the locked 8-step trace** (memory loaded
   → skills considered → plan generated → plan selected → policy check
   → approval → execution → outcome) — `DecisionChainCard.tsx:42-47`
3. **Provenance card** with 5 hash rows (model · prompt template ·
   skill version · memory snapshot · policy version), each with copy
   button, plus a "Copy hash chain" button that emits a canonical JSON
   payload (`ProvenanceBlock.tsx:31-58`)
4. Action inputs card + output card (computer-use screenshot lightbox
   slot reserved)
5. **Sandboxed replay** at `/audit/action/<id>/replay` with four
   outcome verdicts including `leak-detected` for side-effect-bearing
   replays. (`replay/page.tsx:91-159`)

That drill-in is **demo-grade for a security review**. A CISO who
opens it sees forensic-grade transparency. The gap is that nothing on
the audit *feed* hints at how rich the drill-in is — buyers must click
through to discover the engine room.

**Over-claim risk:** `landing/page.tsx:128` and `compare/page.tsx:72`
claim "forensic-grade audit chain by default … hash-chained." The
feed page does not render any hash. A skeptic clicking `/audit` first
might bounce before they reach a row's drill-in.

---

### 3. Worker Trust Pages — score **6 / 10**

**Files surveyed:**
- `~/code/argo/web/app/(app)/workers/[workerId]/trust/page.tsx` (LIVE)
- `~/code/argo/web/components/skycot/worker/TrustGradeCard.tsx`
- `~/code/argo/web/components/skycot/worker/TrustGradeProgress.tsx`
- `~/code/argo/web/components/skycot/WorkerTabs.tsx`

| Sub-tab planned by the brief        | Status | Evidence |
|-------------------------------------|--------|----------|
| `/workers/[id]/audit` (worker-scoped audit) | **NOT a tab — exists as a deep-link from Activity tab** | `workers/[workerId]/activity/page.tsx:122-128` renders an `Open in Audit timeline →` link to `/audit?worker=<id>` |
| `/workers/[id]/policies`            | **NOT a route** | Policies are tucked under Settings (`workers/[workerId]/settings/page.tsx:86-91` → `ApprovalPolicySection`). They are not first-class navigation. |
| `/workers/[id]/trust` (trust ratchet visualization) | YES | `trust/page.tsx:25-95` — `TrustGradeCard` with current grade pill + 3 signals + 5-step ladder |
| `/workers/[id]/escalation`          | **NOT a route — no component anywhere** | `find -name "*scalat*"` returns nothing in `web/`. The MISSION.md "escalation path" is unrepresented in UI. |
| Trust-score-over-time graph         | **NO** | `TrustGradeCard.tsx:53-112` shows current grade + 3 signal values; the ladder shows criteria for the next rung. No time-series visualisation. |
| Read-only enforcement               | YES    | `trust/page.tsx:53-60` disclaimer "not operator-tunable" |
| Pause / Freeze worker               | YES    | `FreezeButton.tsx` + `FreezeBanner.tsx` always rendered above the hero (`WorkerLayoutShell.tsx:71-76`) — kill-switch is visible on every tab |

**Strengths:** the trust grade page is grown-up and reads honestly
("not operator-tunable, derived from observed behaviour"). The freeze
banner on every worker tab is exactly what a CISO wants — a panic
button visible at all times. Approval-policy editing with required
operator-note (`ApprovalPolicyEditModal.tsx`) is the right shape.

**Gaps that would fail a security review:**

- No `/workers/[id]/escalation` page despite MISSION.md item #2 calling
  out "escalation path." A buyer asking "what happens if Mariah is
  about to do something we said was forbidden — who does she
  escalate to?" has no UI to show.
- No trust-score time-series. The claim "trust ratchet earns trust
  over time" lacks a visualisation of the over-time arc.
- No per-worker dedicated audit tab — currently a link to the
  global `/audit?worker=X` page. That works, but it pulls the buyer
  out of the worker context.
- Policies live three clicks deep (Worker → Settings → table). Should
  arguably be a first-class tab.

---

### 4. Trust Ratchet UI — score **7 / 10**

**Files:**
- `~/code/argo/web/components/skycot/TrustRatchetCard.tsx`
- `~/code/argo/web/components/skycot/worker/TrustGradeCard.tsx`
- `~/code/argo/web/components/skycot/worker/TrustGradeProgress.tsx`

The trust ratchet has **two surfaces**:

1. **Promotion offer card** inline on Approvals inbox — `TrustRatchetCard.tsx`
   renders when a streak crosses the threshold ("You've approved
   Mariah's send-listing 7 times in a row. Auto-approve going
   forward?"). Honest copy, dismissable for 7 days, promotion gated
   on operator click. (`TrustRatchetCard.tsx:94-137`)
2. **Trust grade card** on `/workers/[id]/trust` — current grade pill
   + 3 signals + 5-step ladder showing what unlocks the next rung.

Both are demo-grade. The grade reads honestly off backend signals
(`useWorkerTrustGrade` consumes `TrustGradeSnapshot`).

**Gaps:**
- No time-series chart of trust over time.
- No "trust score" badge on the global Workers list or Cohort tiles.
  A buyer scrolling the team page can't tell at a glance which
  workers are "junior" vs "tenured."
- The mission `compare/page.tsx:61` says SKYCOT "promotes workers
  from supervised → autonomous." The UI exposes the 5 grades and
  current rung, but never explicitly labels "supervised vs
  autonomous." A buyer expecting that vocabulary has to translate.

---

### 5. Approval Detail — score **(See section 2's drill-in)**

The approval inbox card itself does NOT route to a `/approvals/<id>`
page. The card *is* the detail in the inbox. The "approval detail" a
security reviewer would expect — full audit trail, worker reasoning,
cost, hash, provenance — lives on `/audit/action/<id>` *after*
the action runs.

**This is a real UX seam:** before the action, the operator sees the
ApprovalCard (verb + draft + reason). After the operator clicks
approve, the action runs and an audit row appears. The buyer asking
"can I see what Mariah is about to do, the full chain, the cost
prediction, the confidence?" gets a card with verb + draft + reason —
not a forensic preview.

**Recommendation:** either add the audit-detail-style provenance block
to the approval card (defense-in-depth for procurement), or wire a
`/approvals/<id>` deep-link that shows the same info the post-action
audit row would carry, framed as "this is what's about to happen."

---

### 6. Safety Posture Copy — score **5 / 10** (over-claim risk: HIGH)

**Files surveyed:**
- `~/code/argo/web/app/landing/page.tsx`
- `~/code/argo/web/app/pricing/page.tsx`
- `~/code/argo/web/app/customers/page.tsx`
- `~/code/argo/web/app/(app)/compare/page.tsx`
- `~/code/argo/web/app/changelog/page.tsx`

**Claims advertised in marketing:**

| Claim                                  | File:line              | Substrate verdict |
|----------------------------------------|------------------------|-------------------|
| "SOC 2 Type II"                        | `landing/page.tsx:843`, `pricing/page.tsx:99` | **PARTIAL** — `argo-cli/src/handlers/compliance_soc2.rs` and `ExportFormatSelector` ship a SOC2-format audit export. A SOC2 Type II *attestation* is an external auditor activity that takes 6-12 months of observed operating effectiveness. The product cannot claim "SOC 2 Type II" until the auditor's report lands. **Over-claim risk = HIGH** if no audit report has actually been issued. |
| "ISO 27001"                            | `landing/page.tsx:847`, `pricing/page.tsx:102` | **OVER-CLAIM** — no evidence of ISO 27001 management-system documentation in this audit; same caveat as SOC 2: it's a 12-month certifiable management system, not a code feature. |
| "GDPR"                                 | `landing/page.tsx:851`, `pricing/page.tsx:100`, `customers/page.tsx:91` | **STRONG** — `/settings/data-export` ships a GDPR Article 15 data-subject export (`settings/data-export/page.tsx:46`). Tenant isolation invariant + RBAC redaction on audit are real. GDPR compliance is structural in this codebase. |
| "HIPAA"                                | `landing/page.tsx:851`, `pricing/page.tsx:101`, `customers/page.tsx:91` | **OVER-CLAIM** — HIPAA disclosure log format is wired in `ExportFormatSelector`, but a BAA-eligible implementation requires PHI handling controls, encryption-at-rest with operator-controlled keys (SQLCipher is on the roadmap per CLAUDE.md, "v1.0 scope per operator 2026-05-18"), audit log of PHI access, etc. The substrate is partially there; the legal claim is not. |
| "AFSL"                                 | `landing/page.tsx:851`, `pricing/page.tsx:103`, `customers/page.tsx:103-104` | **OVER-CLAIM** — AFSL (Australian Financial Services Licence) is a regulatory license you hold, not a software feature. The customer quote at `customers/page.tsx:104` ("SKYCOT was the only platform that shipped with the audit chain regulators actually want") cannot be substantiated by the codebase. |
| "Forensic-grade audit chain"           | `landing/page.tsx:58`, `compare/page.tsx:72`, `changelog/page.tsx:128` | **REAL** — `argo-audit` crate, hash chain fuzzing (`argo-audit/fuzz/fuzz_targets/audit_chain_hash.rs`), drill-in provenance card with 5-hash chain, `Copy hash chain` clipboard JSON, sandboxed replay with leak detection. **This claim is defendable.** |
| "Capability-based RBAC"                | `landing/page.tsx:58`, `pricing/page.tsx:105`, `compare/page.tsx:97` | **REAL** — `RbacPermission` types + `ViewerPolicy` + `ElevatedViewRequestDialog` + redaction-banner flow. Substrate matches the claim. |
| "Encryption-at-rest"                   | `landing/page.tsx:58`, `pricing/page.tsx:104` | **PARTIAL** — `pricing` page calls it "Encryption-at-rest (SQLCipher)" but CLAUDE.md status (2026-05-18) shows "SQLCipher 4-PR sequence" as **remaining for launch**. The advertised "SQLCipher" is not yet on `main`. |
| "Approval-first / approval before they act" | `landing/page.tsx:825`, multiple | **REAL** — approval gate is the load-bearing primitive (MISSION.md). |
| "One-click freeze"                     | `changelog/page.tsx:152`, `compare/page.tsx:77` | **REAL** — `FreezeButton.tsx` ships and is visible on every worker page. |

**Under-claim opportunities** (things the product delivers that
marketing is silent about):

1. **Sandboxed replay with side-effect-leak detection.**
   `/audit/action/<id>/replay` is *unique in the industry* — runs the
   exact action in a sealed sandbox, diffs the output, screams if a
   side-effect leaks. Marketing never mentions it. (`replay/page.tsx`)
2. **Provenance hash-chain with copy-to-clipboard canonical JSON
   payload.** Auditors can paste this into their own verifier.
   Marketing only says "audit chain." It should say "audit chain you
   can verify yourself with a Python one-liner."
3. **RBAC-aware redaction with operator-driven elevation requests.**
   This is enterprise-grade. Marketing copy on landing/pricing says
   nothing about it.
4. **Per-worker freeze button visible on every tab.** The CISO panic
   button is a real, top-class feature. Marketing buries it in
   changelog and a row on the compare table.
5. **30-second undo window on every approval decision.** Forgiving by
   default. Not mentioned in marketing.
6. **Per-action approval-policy editing with required operator note
   captured in audit.** This is exactly the change-management story
   procurement wants. Not mentioned.

---

## Top 3 trust gaps before demo

1. **Approval inbox is missing every "ops-dashboard" signal that the
   design lock prescribes and the marketing implies.** No 5-KPI strip;
   no severity tiers; no SLA bars; no $-at-risk per row; no confidence
   donut. The current inbox is a friendly approval list. The product
   was *designed* to be an ops-dashboard a manager could survey at a
   glance. **Until this lands, a Diego-persona walkthrough is asking
   "where's the rest of it?"** — `~/code/argo/web/app/(app)/page.tsx:512-545`.

2. **Audit feed renders no hash on rows.** The product's strongest
   trust claim ("forensic-grade hash chain") is invisible until the
   buyer clicks into one row. Etherscan, the metaphor SKYCOT chose,
   renders the hash on every row of the feed. Adding even a short
   8-char hash chip per row would deliver the chain-of-custody promise
   on first scroll. — `~/code/argo/web/components/skycot/ActivityRow.tsx:33-86`.

3. **Marketing over-claims SOC 2 Type II, ISO 27001, HIPAA, and AFSL.**
   These are not software features. They are external certifications
   and regulatory licenses. The codebase ships supporting machinery
   (SOC2-format audit export, HIPAA disclosure log format, GDPR
   data-subject export) — that's defendable. But naming the standards
   on `/landing` and `/pricing` as if SKYCOT *holds* them creates real
   procurement exposure. A buyer who asks "send me your SOC 2 Type II
   report" cannot be answered. Recommend reframing: "SOC 2-ready audit
   export," "HIPAA disclosure-log format," "GDPR Article 15 export
   built in." — `landing/page.tsx:843-851`, `pricing/page.tsx:99-103`,
   `customers/page.tsx:103-104`.

---

## Over-claim risks (in priority order)

1. **SOC 2 Type II / ISO 27001 / HIPAA / AFSL named as held
   certifications.** Procurement-fatal if challenged. Reframe to
   "audit export aligned with…" or move to a clearly-labelled "Roadmap"
   panel until certifications actually issue.
2. **"AFSL (Australian Financial Services)" as a feature row in
   pricing.** AFSL is a license you hold, not a SKU. `pricing/page.tsx:103`.
3. **Encryption-at-rest (SQLCipher) advertised as shipped.** Per
   CLAUDE.md the SQLCipher 4-PR sequence is *remaining for launch*.
   `pricing/page.tsx:104`.
4. **`customers/page.tsx` testimonial copy** ("HIPAA-compliant out of
   the box. Our compliance team finally relaxed.") quotes a fictional
   customer. If used in published collateral, advise framing as
   "illustrative" or pull entirely.
5. **`changelog/page.tsx:128` claim** "Forensic-grade audit chain is
   now wired through ten of the eleven primitives — every action,
   every approval, every escalation lands in the chain." This is
   defendable in substrate (`argo-audit` crate + fuzzing) but the
   `/audit` page itself does not yet render the hash on rows, which a
   skeptic will treat as a contradiction.

---

## Under-claim opportunities to brag about

1. **Sandboxed replay with leak detection** — unique. Lead with it.
2. **Self-verifiable hash chain** (clipboard-canonical JSON) — let
   buyers paste into their own verifier.
3. **RBAC-aware redaction + elevation request flow** — exactly what
   regulated buyers ask about.
4. **Worker freeze on every tab + freeze writes to the chain** — the
   panic button. Procurement loves panic buttons.
5. **30-second undo on every approval** — forgiving by default.
6. **Operator-note required on every policy change** — change
   management for free.
7. **Tenant isolation as an architectural invariant (not a setting)**
   — `compare/page.tsx:91` mentions it. Promote it to the landing hero
   trust strip.

---

## What would survive a security review TODAY

- Per-action drill-in surface (`/audit/action/<id>`) — **excellent**
- Sandboxed replay surface (`/audit/action/<id>/replay`) — **excellent**
- Provenance hash chain with copy-to-clipboard — **excellent**
- Worker freeze with audit-chain write — **excellent**
- GDPR data-subject export (`/settings/data-export`) — **excellent**
- Evidence-bundle exporter with SOC2/GDPR/HIPAA formats — **good** (formats real, certifications not)
- RBAC redaction + elevation request flow — **excellent**

## What would NOT survive a security review TODAY

- Marketing claim "SOC 2 Type II / ISO 27001 / HIPAA / AFSL" as held
  certifications — **fix copy**
- Approval card showing zero hash, zero audit-row id, zero cost
  prediction — **fix UI**
- Audit feed row showing zero hash chip — **fix UI**
- Missing `/workers/[id]/escalation` route despite MISSION.md
  escalation-path requirement — **build it or remove the claim**
- No trust-score time-series ("ratchet over time" is core to the
  pitch) — **build a sparkline**
- SQLCipher claim shipped to marketing before SQLCipher PRs merge —
  **delay the claim or accelerate the PRs**

---

## Closing posture

The substrate of SKYCOT's trust + safety + audit chain is genuinely
demo-grade. The drill-in surfaces would impress a CISO. The promotion
gates, freeze button, RBAC redaction, and replay are real engineering
delivered honestly. **The problem is shopfront, not substrate.** A
buyer arrives at `/landing` expecting SOC 2, lands on `/` expecting the
ops-dashboard described in the design lock, and lands on `/audit`
expecting Etherscan-style hash rows. All three first impressions
under-deliver against marketing. Closing those three gaps moves the
overall score from 6.4 → 8+ without writing a single line of new
backend code. The engine room is fine; the windows are dirty.
