# Wave-10 Mission Alignment — Final Audit — 2026-05-21

**Auditor:** Wave-10 audit agent #1 (post-9-waves re-score)
**Baselines:**
- Wave-1 (`docs/skycot/audit/mission-alignment/2026-05-20-wave-1-mission-alignment.md`) — 6.40/10
- Wave-6 (`docs/skycot/audit/mission-alignment-re/2026-05-21-wave-6-mission-re-verify.md`) — 6.67/10

**Mission target (per `docs/skycot/MISSION.md` + `CLAUDE.md`):**
> "AI teammates that live in Slack, learn from your feedback, ask before they act. Audit-first. Single-binary. Enterprise-grade."

**Scope:** identical 12 surfaces re-scored 1-10 across 6 pillars:
1. TEAMMATES — named colleagues, personality, avatar, voice
2. APPROVAL-FIRST — gates visible, "ask before act", trust ratchet
3. AUDIT-FIRST — every action logged, hash chain, evidence export
4. ENTERPRISE-GRADE — SOC 2 / HIPAA / RBAC / tenant isolation surfaced
5. SINGLE BINARY — self-host / no-Kubernetes / download-and-run
6. VIRAL / EPIC UX — moments you'd screenshot

Rubric: 1-3 = absent or contradicts; 4-6 = mentioned, decorative; 7-8 = clearly delivers with specifics; 9-10 = the surface IS the pillar.

---

## Methodology — what landed between Wave-6 and Wave-10

Across waves 6 → 9 a steady cadence of fix-cycle commits + 42+ TODO briefs landed. Material code changes that move pillar scores (Wave-6 → Wave-10):

| Commit / PR | Surface(s) | Pillar movement |
|---|---|---|
| `cbf81b01` Wave-6 pre-record fixes | `/workflows/.../build`, `/landing`, `/workflows` | All three: builder toast lies softened (3 lies → 3 honest copy lines), landing hero bubbles `min-width: 220px`, PWA banner env-var-gated for clean demo recording, workflows TRY-chip pop search, "Propose new" rewired to `/workflows/templates` |
| `2489e57f` Wave-7 fix-cycle | `/workflows`, `/workflows/.../build` | `/workflows` zero-results state with "Clear search" + "Browse templates →" CTAs; `onNodeKeyDown` on builder canvas (WCAG 2.1.1 keyboard a11y) |
| `8b0a43b8` (#1890) trigger config UX | `/workflows/.../build` | Per-trigger-kind tailored config: WebhookConfigForm (URL+secret+test-ping+curl), ScheduleConfigForm (cron picker + next-5-runs), EmailConfigForm (IMAP creds + filters), FormConfigForm (field builder). Lifts approval-first/enterprise-grade pillars in the builder by making triggers feel production-grade. |
| `e23a4b09` Wave-9 fix-cycle | `/landing` | Intermediate-viewport (700-890px) hero bubble fix — 4-corner grid + below-540px hide-art breakpoint. Pure responsive polish; no pillar move. |
| `799f1abe` (#1895) /customers detail pages | `/customers` | Per-customer slugs with rich approval-first + audit-first + named-worker copy in setupParagraphs (Northwind: "deal-closer was approval-first posture", "audit trail" 5× across 4 customers). Recovers the Wave-6 -0.5 regression on the index. |
| `7d6e0967` (#1898) onboarding F1 shell | `/onboarding` (Wave-6 already credited) | No new movement past Wave-6 — three pillar cards (audit-first / single-binary / enterprise-grade) still in place. |
| `f298fb51` (#1900) team C2 reskin | `/team` (Wave-6 already credited) | No new movement past Wave-6. |
| **NOT MERGED** (in-review only): `#1902` audit-feed-per-row-hash, `#1904` worker-detail-escalation+trust-chart, `#1905` approvals-inbox-ops-dashboard-completion | `/audit`, `/workers/[id]`, `/` | Would lift audit-first on `/audit` index and approval-first on `/` if merged. Wave-10 scores them as in-flight, not landed. |
| **NOT MERGED**: single-binary footer chip | All in-app surfaces | Wave-6 recommended a 30-LOC `AppShellFooter` chip ("Self-hosted on your hardware · skycot.com/security"). Deferred to a post-record brief; not landed. Single-binary remains invisible on 8 of 12 surfaces. |

The bulk of Wave-7 + Wave-9 work was a11y / responsive / honesty polish — quality lifts that don't move pillar scores but reduce buyer-catchable demo lies. The wave-7 keyboard-nav fix lifts viral UX by making the builder demo recordable end-to-end with the keyboard.

---

## /landing — `web/app/landing/page.tsx` (1196 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 9 | 9 | 9 | 0 | Unchanged — Mariah/Priya/Jamal/Nori bubbles + HOW step "spins up a named worker". |
| APPROVAL-FIRST | 8 | 8 | 8 | 0 | Unchanged — feature card "Asks before it acts" + HOW step "Approve their first decisions". |
| AUDIT-FIRST | 7 | 7 | 7 | 0 | Unchanged — feature card + HOW step + L949 trust strip "SOC 2-READY AUDIT TRAIL". |
| ENTERPRISE-GRADE | 9 | 9 | 9 | 0 | Unchanged — trust strip "SOC 2 · ISO 27001 · GDPR · HIPAA". |
| SINGLE BINARY | 5 | 5 | 5 | 0 | Unchanged — sub-headline (L932) mentions "single-binary" once; no dedicated section. |
| VIRAL / EPIC UX | 9 | 9 | 9 | 0 | Unchanged — Audiowide hero + gradient + chat bubbles. Intermediate-viewport bubble fix (L505-524) is responsive polish — recordable on more screen sizes — but doesn't change the screenshot moment. |

**W-10 avg:** 7.8 (Δ 0.0 vs W-6) **Verdict:** Stable. Single-binary is still the weakest pillar; the Wave-6 promotion recommendation has not landed.

---

## /compare — `web/app/compare/page.tsx` (479 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 10 | 10 | 10 | 0 | Unchanged. |
| APPROVAL-FIRST | 10 | 10 | 10 | 0 | Unchanged. |
| AUDIT-FIRST | 9 | 9 | 9 | 0 | Unchanged. |
| ENTERPRISE-GRADE | 10 | 10 | 10 | 0 | Unchanged. |
| SINGLE BINARY | 10 | 10 | 10 | 0 | Unchanged. |
| VIRAL / EPIC UX | 8 | 8 | 8 | 0 | Unchanged. |

**W-10 avg:** 9.5 (Δ 0.0) **Verdict:** Still the highest-scoring surface in the audit and still public (middleware allowlist holding from W-6). No regression.

---

## /pricing — `web/app/pricing/page.tsx` (810 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 8 | 8 | 8 | 0 | Unchanged. |
| APPROVAL-FIRST | 6 | 6 | 6 | 0 | Unchanged — still buried in Pro features list. |
| AUDIT-FIRST | 8 | 8 | 8 | 0 | Unchanged. |
| ENTERPRISE-GRADE | 10 | 10 | 10 | 0 | Unchanged. |
| SINGLE BINARY | 9 | 9 | 9 | 0 | Unchanged. |
| VIRAL / EPIC UX | 7 | 7 | 7 | 0 | Unchanged. |

**W-10 avg:** 8.0 (Δ 0.0) **Verdict:** Stable.

---

## /customers — `web/app/customers/page.tsx` (488 LOC) + `/customers/[slug]/page.tsx` (270 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 10 | 9 | 10 | **+1** | **Recovery.** With the slug detail pages live (#1895, evaluated in this wave), the named-worker thread reappears: Mariah at Northwind, Jamal at Beacon, Priya at Apex, Tobias at Maple — each with role + tagline + signature behavior in setupParagraphs. Card-level worker references are also stronger (cardQuoteAuthor + named worker in cardQuote: "Mariah replied to 12,400 buyers…", "Priya screens 200 candidates a week…"). |
| APPROVAL-FIRST | 8 | 7 | 9 | **+2** | **Recovery + lift.** Detail-page evidence is dense: Northwind setupParagraphs[2] "deal-closer wasn't accuracy — it was the approval-first posture: every escalation point is configurable"; Beacon setupParagraphs[1] explicit "audit-first actually means in practice. Every action Jamal takes is gated, logged, and traceable"; Apex setupParagraphs "asks for human review on any score within the close-call band". The cardQuote layer references "trust ratchet" twice (Apex, Northwind). |
| AUDIT-FIRST | 8 | 7 | 9 | **+2** | **Recovery + lift.** Detail pages carry specific audit evidence: Beacon "Every Jamal action ships with: policy version, tenant, actor, inputs, redaction set, content hash"; Maple "Tobias collects evidence on a schedule, every collection ties to a policy version"; Northwind "audit chain — every reply traceable to a specific KB article, every decision logged against intent classification". Beacon second quote: "SOC 2 auditor asked for a sample with full evidence. We pulled it in three minutes." This is now the strongest audit-pillar marketing surface in the funnel. |
| ENTERPRISE-GRADE | 9 | 9 | 9 | 0 | Unchanged — industry + size + vertical chip + ASIC/AFSL/HIPAA in copy. |
| SINGLE BINARY | 1 | 1 | 1 | 0 | Unchanged — still not mentioned in customer copy. |
| VIRAL / EPIC UX | 8 | 8 | 8 | 0 | Unchanged — gradient blocks + italic quotes + animated CTA band. Detail pages add a step-list workflow showcase pulled from `web/lib/workflow-templates.ts` (no parallel SOT) — meaningful but not "screenshot moment" register. |

**W-10 avg:** 7.7 (Δ +0.9 vs W-6 6.8) **Verdict:** **Biggest single-surface recovery in Wave-10.** The W-6 -0.5 regression (CMS-driven generic cards) is reversed and overshot — the new detail pages carry the pillar weight the index alone couldn't, and the index now reads as "card + link to depth" exactly as the W-6 audit predicted. **Notable: every card still carries `dataPending: true` indicating placeholder content** — when real design-partner stories replace placeholders the surface will hold the same shape.

---

## / (Approvals inbox) — `web/app/(app)/page.tsx` (563 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 7 | 7 | 7 | 0 | Unchanged — `BrandLoudHero` + "Your AI teammates need a yes or no". |
| APPROVAL-FIRST | 10 | 10 | 10 | 0 | Unchanged — every primitive still enforces it. |
| AUDIT-FIRST | 6 | 6 | 6 | 0 | Unchanged. `#1905` approvals-inbox-ops-dashboard-completion in-review may move this when it lands. |
| ENTERPRISE-GRADE | 7 | 7 | 7 | 0 | Unchanged. |
| SINGLE BINARY | 2 | 2 | 2 | 0 | Unchanged — footer-chip brief not landed. |
| VIRAL / EPIC UX | 8 | 8 | 8 | 0 | Unchanged — loading-flash fix still present (L530-535). |

**W-10 avg:** 6.7 (Δ 0.0) **Verdict:** Stable. Approval-first remains locked at 10.

---

## /team — `web/app/(app)/team/page.tsx` (462 LOC) + `TeammateCard.tsx`

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 10 | 10 | 10 | 0 | Unchanged — C2-holographic TeammateCard with trait chips + level number + gradient banner + animated shine still in place. |
| APPROVAL-FIRST | 5 | 5 | 5 | 0 | Unchanged. |
| AUDIT-FIRST | 3 | 3 | 3 | 0 | Unchanged. |
| ENTERPRISE-GRADE | 3 | 3 | 3 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 9 | 10 | 10 | 0 | Unchanged — full C2 port still in place. |

**W-10 avg:** 5.3 (Δ 0.0) **Verdict:** Stable. Pillar shape unchanged — owns teammates+viral at 10, leaves 4 pillars on the table.

---

## /workers/[workerId]/overview

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 10 | 10 | 10 | 0 | Unchanged — `WorkerPersonalityCard` lock holding. |
| APPROVAL-FIRST | 7 | 7 | 7 | 0 | Unchanged. |
| AUDIT-FIRST | 4 | 5 | 5 | 0 | Unchanged from Wave-6 — `RecentChangesPanel` still in place. `#1904` worker-detail-escalation+trust-chart in-review may bump this when it lands. |
| ENTERPRISE-GRADE | 4 | 4 | 4 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 10 | 10 | 10 | 0 | Unchanged. |

**W-10 avg:** 6.2 (Δ 0.0) **Verdict:** Stable.

---

## /workflows — `web/app/(app)/workflows/page.tsx` (602 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 7 | 7 | 7 | 0 | Unchanged. |
| APPROVAL-FIRST | 9 | 9 | 9 | 0 | Unchanged — `risk: "approval-required"` badges on every card. |
| AUDIT-FIRST | 5 | 5 | 5 | 0 | Unchanged — version chips visible. |
| ENTERPRISE-GRADE | 4 | 4 | 4 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 9 | 9 | 9 | 0 | Unchanged — the zero-results state (L507-558) is a quality polish: "No workflows match '<q>'" + "Clear search" + "Browse templates →" gradient CTA. Improves the demo recording but does not move the pillar score. The TRY-chips (`approval flows`, `data extraction`, etc.) now actually populate `setQuery` (L495). |

**W-10 avg:** 5.8 (Δ 0.0) **Verdict:** Stable. The Wave-7 polish makes search recordable without a blank-grid flash but the pillar shape is unchanged.

---

## /workflows/.../build — `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 4 | 4 | 4 | 0 | Unchanged. |
| APPROVAL-FIRST | 9 | 9 | 9 | 0 | Unchanged — approval-gate first-class step kind. |
| AUDIT-FIRST | 3 | 3 | 3 | 0 | Unchanged. |
| ENTERPRISE-GRADE | 4 | 4 | 5 | **+1** | **Bump.** Per-trigger-kind tailored config forms (#1890) landed: WebhookConfigForm (read-only URL + clipboard + confirm-protected secret regenerate + test-ping + curl snippet + optional header auth + allowed-methods chips), ScheduleConfigForm (cron mode picker + timezone + plain-English summary + next-5-runs preview), EmailConfigForm (IMAP creds + TLS toggle + filter rules + Test connection), FormConfigForm (field builder). Production-grade trigger UX is a load-bearing signal of enterprise readiness on the surface a buyer evaluates the "extensibility" claim. |
| SINGLE BINARY | 1 | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 10 | 10 | 10 | 0 | Unchanged. Three honest stubs now (toast at L341-351, L359-361, L369-371): "Drafting:… Live wire ships in PR #1882", "Test run — preview build", "Saved as draft — preview build". Each describes the action without claiming completion. `onNodeKeyDown` callback (L309-315) lets keyboard users open the side panel by tabbing + Enter/Space — WCAG 2.1.1 closure on the central differentiator. |

**W-10 avg:** 5.3 (Δ +0.2) **Verdict:** Marginal bump. Enterprise-grade lifts from "decorative" to "specific" because the trigger config forms make the builder feel like a production tool, not a demo toy. Keyboard-a11y fix makes the surface end-to-end recordable; honest toasts mean no buyer-catchable lies during the demo.

---

## /audit — `web/app/(app)/audit/page.tsx` (425 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 5 | 5 | 5 | 0 | Unchanged. |
| APPROVAL-FIRST | 7 | 7 | 7 | 0 | Unchanged. |
| AUDIT-FIRST | 10 | 10 | 10 | 0 | Unchanged. `ProvenanceBlock` on detail page still load-bearing. `#1902` audit-feed-per-row-hash in-review will surface hash badges on the index — but until it merges, buyers still drill in. |
| ENTERPRISE-GRADE | 9 | 9 | 9 | 0 | Unchanged. |
| SINGLE BINARY | 2 | 2 | 2 | 0 | Unchanged. |
| VIRAL / EPIC UX | 7 | 7 | 7 | 0 | Unchanged. |

**W-10 avg:** 6.7 (Δ 0.0) **Verdict:** Stable. Audit-first locked at 10 via detail page; index page still doesn't surface per-row hashes until #1902 merges.

---

## /skills — `web/app/(app)/skills/page.tsx` (412 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 5 | 5 | 5 | 0 | Unchanged. |
| APPROVAL-FIRST | 6 | 6 | 6 | 0 | Unchanged. |
| AUDIT-FIRST | 4 | 4 | 4 | 0 | Unchanged. |
| ENTERPRISE-GRADE | 4 | 4 | 4 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 9 | 9 | 9 | 0 | Unchanged. |

**W-10 avg:** 4.8 (Δ 0.0) **Verdict:** No change. Still the lowest-scoring surface and still pillar-thin.

---

## /onboarding (Welcome) — `web/app/(setup)/onboarding/welcome/page.tsx` (81 LOC)

| Pillar | W-1 | W-6 | W-10 | Δ vs W-6 | Evidence |
|---|---|---|---|---|---|
| TEAMMATES | 8 | 9 | 9 | 0 | Unchanged — F1 shell + "Let's hire your first worker. (90 seconds.)". |
| APPROVAL-FIRST | 4 | 4 | 4 | 0 | Unchanged — still no example approval-gate flow during onboarding. |
| AUDIT-FIRST | 1 | 7 | 7 | 0 | Unchanged — `data-testid="onboarding-pillar-audit-first"` card holds. |
| ENTERPRISE-GRADE | 2 | 7 | 7 | 0 | Unchanged — enterprise-grade pillar card holds. |
| SINGLE BINARY | 1 | 7 | 7 | 0 | Unchanged — single-binary pillar card holds. |
| VIRAL / EPIC UX | 7 | 9 | 9 | 0 | Unchanged. |

**W-10 avg:** 7.2 (Δ 0.0) **Verdict:** Stable. Wave-6's +3.4 jump holds.

---

## Wave-10 summary table

| Surface | W-1 | W-6 | W-10 | Δ W-6→W-10 | Worst pillar (W-10) |
|---|---|---|---|---|---|
| /compare | 9.5 | 9.5 | 9.5 | 0.0 | (none below 8) |
| /pricing | 8.0 | 8.0 | 8.0 | 0.0 | Approval-first (6) |
| /landing | 7.8 | 7.8 | 7.8 | 0.0 | Single binary (5) |
| /customers | 7.3 | 6.8 | **7.7** | **+0.9** | Single binary (1) |
| /onboarding | 3.8 | 7.2 | 7.2 | 0.0 | Approval-first (4) |
| /audit | 6.7 | 6.7 | 6.7 | 0.0 | Single binary (2) |
| / (approvals) | 6.7 | 6.7 | 6.7 | 0.0 | Single binary (2) |
| /workers/[id]/overview | 6.0 | 6.2 | 6.2 | 0.0 | Single binary (1) |
| /workflows | 5.8 | 5.8 | 5.8 | 0.0 | Single binary (1) |
| /team | 5.2 | 5.3 | 5.3 | 0.0 | Single binary (1) |
| /workflows/.../build | 5.2 | 5.2 | 5.3 | +0.2 | Single binary (1) |
| /skills | 4.8 | 4.8 | 4.8 | 0.0 | Single binary (1) |

**Overall mission-delivery score (12-surface mean):**
- Wave-1: 6.40 / 10
- Wave-6: 6.67 / 10 (Δ +0.27 over W-1)
- **Wave-10: 6.75 / 10** (Δ **+0.08** over W-6, **+0.35** over W-1)

The Wave-6 → Wave-10 lift (+0.08) is entirely carried by:
1. `/customers` recovery + overshoot (+0.9 on a single surface = +0.075 overall)
2. `/workflows/.../build` enterprise-grade bump (+0.2 on a single surface = +0.017 overall)

…minus a near-zero loss on 9 other surfaces. The mean is stable — Wave-7+ work was reliability/honesty polish (toast lies, keyboard a11y, search empty state, hero responsive) rather than pillar-shifting.

---

## Top 3 improved surfaces (Wave-6 → Wave-10)

1. **/customers — +0.9 (6.8 → 7.7)**. The single biggest surface-level improvement of the period. Detail pages at `/customers/[slug]` (shipped #1895, evaluated in Wave-10 with rich content review) carry approval-first +2 and audit-first +2 lifts via specific evidence in setupParagraphs (Northwind "deal-closer was approval-first posture", Beacon "every action gated, logged, traceable to policy version", Maple "every collection ties to a policy version + content hashes"). Teammates recovers to 10 with named workers (Mariah, Jamal, Priya, Tobias). **Index + detail together is now the strongest marketing surface for the audit-first pillar.**
2. **/workflows/.../build — +0.2 (5.2 → 5.3)**. Enterprise-grade lifts from 4 → 5 via per-trigger-kind tailored config drawers (#1890 — Webhook with test-ping + curl, Schedule with cron+next-5-runs, Email with IMAP creds, Form with field builder). The builder no longer feels like a demo toy. Wave-7 `onNodeKeyDown` keyboard fix + Wave-6 honest toasts (3 lies → 3 honest copy lines) reduce buyer-catchable demo failures without moving pillar scores.
3. **Honourable mention: /workflows — Δ 0.0**. Zero-results state polish + TRY-chips that populate search + "Browse templates" CTA improve demo-record quality without moving pillars.

---

## Top 3 still gapping (lowest Wave-10 scores)

1. **/skills (4.8)** — unchanged across 4 audits. Single-binary 1, audit-first 4, enterprise-grade 4, approval-first 6. Beautiful A1 marketplace UX, pillar-thin. Skills float as abstract artifacts without owning workers or surfacing approval/audit semantics. **Highest-priority next-pass target.**
2. **/team (5.3)** — owns teammates+viral at 10/10 but leaves approval-first (5), audit-first (3), enterprise-grade (3), single-binary (1) on the table. The "asks before acts" safety mechanism is not surfaced at the roster level.
3. **/workflows/.../build (5.3)** — and **/workflows (5.8)**. The visual builder is class-leading on viral UX (10) but weak on 4 of 6 pillars. Workflow-shaped, not teammate-shaped.

---

## Single biggest mission gap still open (unchanged from Wave-6)

**Single-binary is invisible in 8 of 12 surfaces.** Pillar scores for "single binary":

| Surface | W-10 score |
|---|---|
| /compare | 10 |
| /pricing | 9 |
| /onboarding | 7 |
| /landing | 5 |
| /audit | 2 |
| / (approvals) | 2 |
| /customers | 1 |
| /workflows | 1 |
| /workflows-build | 1 |
| /team | 1 |
| /skills | 1 |
| /workers/[id]/overview | 1 |

The Wave-6 fix recommendation — a 30-LOC shared `AppShellFooter` footer chip ("Self-hosted on your hardware · skycot.com/security") on all in-app surfaces — was deferred to a post-record brief (`web-single-binary-footer-chip`) and has not landed. **A buyer in trial mode who never visits /compare or /pricing still has no in-app way to learn that SKYCOT is self-hostable.** Single-binary is the only true SaaS-vs-self-host differentiator vs Zapier / Lindy / Crew.ai.

If the footer chip lands: 6 in-app surfaces lift +3 to +5 on single-binary → 12-surface overall mean moves from 6.75 to ~7.10. This is the highest-leverage single change still available.

Secondary gap: **approval-first surfaces inconsistently** in the marketing funnel. /pricing buries it in a feature bullet (6), /onboarding doesn't show a working example gate (4), /skills mentions it but doesn't enforce it (6). The in-product layer (/ at 10, /compare at 10, /workflows at 9) carries the pillar; the funnel does not.

---

## Final demo verdict

**SHIP IT.** The mission promise — "AI teammates that live in Slack, learn from your feedback, ask before they act. Audit-first. Single-binary. Enterprise-grade." — is now demo-ready and substantially stronger than Wave-1.

**The demo path scoring:**

1. **Marketing path (/landing → /compare → /pricing → /customers):** 7.8 / 9.5 / 8.0 / **7.7** (avg 8.25). Wave-6's `/customers` -0.5 regression is reversed and overshot. /compare is the strongest single surface in the entire app and is publicly visible. Marketing → demo conversion path is well-armed.
2. **Sign-up funnel (/auth/signup → /onboarding/*):** /onboarding holds Wave-6's +3.4 jump (3.8 → 7.2). The "marketing promised X, product delivered Y" gap stays closed.
3. **In-product demo (/ → /team → /workers/<id> → /workflows → /workflows/.../build → /audit):** Each pillar has a star surface — Approval-first owned by / (10), Teammates by /team + /workers (10), Audit by /audit (10), Viral by /workers + /workflows-build (10). /workflows-build is marginally stronger thanks to trigger config drawers + honest toasts.
4. **Differentiation pitch (/compare):** 9.5 / 10 — the page that wins deals.

**Caveats / known gaps the demo recording can navigate around:**
- Single-binary is invisible in-app (8 of 12 surfaces). Demo it from /compare or /pricing, not from in-product.
- 3 in-flight PRs (#1902 audit per-row hash, #1904 worker trust chart, #1905 ops dashboard completion) would lift 3 surfaces by ~0.3 each on merge. They do not block the recording.
- All customer cards still carry `dataPending: true`. Brand and pillar shape work; copy is placeholder until design partners ship real stories.

**The overall mission-delivery score has moved from 6.40 (Wave-1) → 6.67 (Wave-6) → 6.75 (Wave-10).** The Wave-6 → Wave-10 lift is modest because the pillar shape was already well-defined; the work in that period was honesty / a11y / responsive polish that made the demo recordable end-to-end without lies. Mission-pillar shape is now stable enough to ship.

**One final recommendation before V1 launch:** land the single-binary footer chip (~30 LOC, ~1 dev-hour). Moves the overall score from 6.75 to ~7.10 by surfacing the only true SaaS differentiator on every in-app surface a buyer visits.

---

## Appendix — Wave-10 source-of-truth files (line counts confirmed 2026-05-21)

- `web/middleware.ts:16-29` — public-route allowlist holding
- `web/app/landing/page.tsx` (1196 LOC) — Wave-9 intermediate-viewport bubble fix at L505-524
- `web/app/compare/page.tsx` (479 LOC)
- `web/app/pricing/page.tsx` (810 LOC)
- `web/app/customers/page.tsx` (488 LOC) + `web/app/customers/[slug]/page.tsx` (270 LOC) — detail pages live
- `web/lib/content/customers.ts` — 4 stories with rich approval-first + audit-first + named-worker copy
- `web/app/(app)/page.tsx` (563 LOC) — approvals inbox, loading-flash fix at L530-535
- `web/app/(app)/team/page.tsx` (462 LOC) + `web/components/skycot/TeammateCard.tsx` — C2 holographic port holding
- `web/app/(app)/workers/[workerId]/overview/page.tsx` (31 LOC) — `WorkerPersonalityCard` + `WorkerOverviewContent` + `RecentChangesPanel`
- `web/app/(app)/workflows/page.tsx` (602 LOC) — Wave-7 zero-results state + TRY-chip search
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` — `onNodeKeyDown` at L309-315 + honest toasts at L341-371 + `TriggerConfigDrawer` at L754
- `web/components/workflows/triggers/TriggerConfigDrawer.tsx` + 4 per-kind forms (Webhook/Schedule/Email/Form)
- `web/app/(app)/audit/page.tsx` (425 LOC) + `web/components/skycot/audit/ProvenanceBlock.tsx`
- `web/app/(app)/skills/page.tsx` (412 LOC) — unchanged
- `web/app/(setup)/onboarding/welcome/page.tsx` (81 LOC) — 3 pillar cards holding

**Wave-10 audit complete. Final overall: 6.75 / 10.**
