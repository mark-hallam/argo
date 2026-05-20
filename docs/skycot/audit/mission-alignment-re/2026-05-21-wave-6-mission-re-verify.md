# Wave-6 Mission-Alignment Re-Verify — 2026-05-21

**Auditor:** Wave-6 audit agent #2 (deep re-score)
**Baseline:** wave-1 audit at `docs/skycot/audit/mission-alignment/2026-05-20-wave-1-mission-alignment.md` (overall 6.4/10)
**Mission target (per `docs/skycot/MISSION.md` + `CLAUDE.md`):**
> "AI teammates that live in Slack, learn from your feedback, ask before they act. Audit-first. Single-binary. Enterprise-grade."

**Scope:** identical 12 surfaces as wave-1, re-scored 1-10 across 6 pillars:
1. TEAMMATES — named colleagues, personality, avatar, voice
2. APPROVAL-FIRST — gates visible, "ask before act", trust ratchet
3. AUDIT-FIRST — every action logged, hash chain, evidence export
4. ENTERPRISE-GRADE — SOC 2 / HIPAA / RBAC / tenant isolation surfaced
5. SINGLE BINARY — self-host / no-Kubernetes / download-and-run
6. VIRAL / EPIC UX — moments you'd screenshot

Rubric:
- 1-3 = absent or actively contradicts the pillar
- 4-6 = mentioned but weak/decorative; not load-bearing
- 7-8 = clearly delivers the pillar with specifics
- 9-10 = the surface IS the pillar — would convince a sceptical buyer

---

## Methodology — what changed since wave-1

Five fix-cycle commits + 37 new TODO briefs landed in the 24 hours between wave-1
(2026-05-20) and wave-6 (2026-05-21). Material changes that move pillar scores:

| Commit | Surface(s) affected | Pillar impact |
|---|---|---|
| `7d6e0967` web-onboarding-f1-wizard-shell merged (#1898) | /onboarding/welcome + all 5 steps | TEAMMATES + AUDIT-FIRST + ENTERPRISE-GRADE + SINGLE-BINARY (all jumped — welcome page now surfaces 3 explicit pillar cards) |
| `f298fb51` web-team-c2-holographic-port (#1900) | /team | TEAMMATES + VIRAL UX (TeammateCard now has trait chips, level number, gradient banner, animated shine) |
| middleware allowlist fix (active) | /compare, /pricing, /customers | All pillars — wave-1's CRITICAL bug (/compare gated) is RESOLVED |
| `1ca5ce06` web-sso-okta-google-azure (#1891) | /settings (out of scope), surrounds enterprise pillar | Surrounds enterprise pillar (real SSO wired, not just marketing claim) |
| `799f1abe` web-customer-case-study-pages (#1895) | /customers | Detail pages now exist (cards link to /customers/[slug]) |
| `0968972f` web-error-boundaries route-segment (#1899) | All in-app surfaces | Reliability foundation; no direct pillar score change |
| `9114c2cd` web-vertical-landing-pages (#1892) | /for/<vertical> | Adjacent funnel surface; out of audit scope but visible from /landing nav |

The wave-1 audit explicitly flagged the `/compare` middleware bug as "the highest-scoring surface in the entire audit — 9.5/10 across all 6 pillars — and it's invisible to anyone considering signing up." That gap is now closed: `web/middleware.ts:16-29` allowlists `/compare`, `/customers`, `/pricing`, `/changelog`, `/docs`, `/for/`.

---

## /landing — `web/app/landing/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 9 | 9 | 0 | Unchanged — same Audiowide hero + 4 named-worker bubbles + HOW step "spins up a named worker (Priya, Jamal, Mariah, etc.)". |
| APPROVAL-FIRST | 8 | 8 | 0 | Unchanged — feature card "Asks before it acts" + HOW step "Approve their first decisions". |
| AUDIT-FIRST | 7 | 7 | 0 | Unchanged — feature card "Forensic-grade audit chain on every action" + HOW step "Watch the audit chain". |
| ENTERPRISE-GRADE | 9 | 9 | 0 | Unchanged — hero trust strip "SOC 2 TYPE II · ISO 27001 · GDPR · HIPAA · AFSL · 2,847 teams this month". |
| SINGLE BINARY | 5 | 5 | 0 | Unchanged — sub-headline mentions "single-binary" once but no dedicated section. |
| VIRAL / EPIC UX | 9 | 9 | 0 | Unchanged — Audiowide hero + gradient + chat-bubble cards. |

**Wave-1 avg:** 7.8 → **Wave-6 avg:** 7.8 (Δ 0.0)
**Verdict:** No regression. Single-binary remains the weakest pillar; could promote it to a dedicated section since it's the only true differentiator vs SaaS competitors.

---

## /compare — `web/app/compare/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 10 | 10 | 0 | Unchanged — "Teammates, not tools" group + "Hire a teammate" tagline. |
| APPROVAL-FIRST | 10 | 10 | 0 | Unchanged — "Safe by default" group with 3 dimensions including freeze-in-one-click. |
| AUDIT-FIRST | 9 | 9 | 0 | Unchanged — "Is every action audit-logged with full lineage?" with caption "SKYCOT ships forensic-grade audit chain by default". |
| ENTERPRISE-GRADE | 10 | 10 | 0 | Unchanged — "Enterprise-ready" group: multi-tenant, RBAC, encryption-at-rest. |
| SINGLE BINARY | 10 | 10 | 0 | Unchanged — "Deploy how you want" group with single-binary + air-gapped questions. |
| VIRAL / EPIC UX | 8 | 8 | 0 | Unchanged — 5-card positioning grid + 13-row scorecard + conversion footer. |

**Wave-1 avg:** 9.5 → **Wave-6 avg:** 9.5 (Δ 0.0)
**Verdict:** Page is unchanged but it is now PUBLIC. Wave-1's critical bug (gated behind auth) is resolved — `middleware.ts:16-29` now allowlists `/compare`. **The page that scored highest in the entire audit is now visible to prospects.** This is the single largest mission-delivery improvement in wave-6 even though the score is identical, because reach × score is what actually matters.

---

## /pricing — `web/app/pricing/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 8 | 8 | 0 | Unchanged — "Hire teammates. Pay like they're real" hero + per-teammate billing. |
| APPROVAL-FIRST | 6 | 6 | 0 | Unchanged — buried in Pro features list. |
| AUDIT-FIRST | 8 | 8 | 0 | Unchanged — matrix "Forensic audit chain" on all tiers + Enterprise "Audit export". |
| ENTERPRISE-GRADE | 10 | 10 | 0 | Unchanged — SOC 2, GDPR, HIPAA, ISO 27001, SQLCipher, RBAC, SSO. |
| SINGLE BINARY | 9 | 9 | 0 | Unchanged — FAQ Q5 explicit "Can I self-host SKYCOT?" with full answer. |
| VIRAL / EPIC UX | 7 | 7 | 0 | Unchanged — 3-tier card grid with featured ribbon + animated CTA band. |

**Wave-1 avg:** 8.0 → **Wave-6 avg:** 8.0 (Δ 0.0)
**Verdict:** Stable. Could louden approval-first messaging but nothing regressed.

---

## /customers — `web/app/customers/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 10 | 9 | -1 | Slight regression — page now reads `SEED_CUSTOMERS` from `@/lib/content/customers` (refactored from inline literals). Customer cards still show named workers (Mariah, Patrick, etc. via `c.cardQuoteAuthor`) but the inline `customerStories` array of wave-1 with prominent worker quotes is now a generic card layout. Worker-anchoring is still present but slightly less prominent. |
| APPROVAL-FIRST | 8 | 7 | -1 | Slight regression — quotes still cite approval-first as buying reason but are now generic per `cardQuote` strings rather than the explicit "Approval-first is the only way" quote from wave-1. |
| AUDIT-FIRST | 8 | 7 | -1 | Slight regression — generic card quotes have less specific audit references than the wave-1 inline quotes. |
| ENTERPRISE-GRADE | 9 | 9 | 0 | Unchanged — industry + size meta + vertical chip on each card. |
| SINGLE BINARY | 1 | 1 | 0 | Unchanged — not mentioned. |
| VIRAL / EPIC UX | 8 | 8 | 0 | Unchanged — gradient logo blocks, italic quotes, animated CTA band. **Plus new detail pages at `/customers/[slug]` per #1895** (out of audit scope but improves the funnel). |

**Wave-1 avg:** 7.3 → **Wave-6 avg:** 6.8 (Δ -0.5)
**Verdict:** Minor regression on the index page from CMS-driven refactor (generic-er quote shape), partially offset by the new per-customer detail pages that ship at `/customers/[slug]` and are linked from each card. Net mission delivery probably unchanged once the detail pages are evaluated; the index now exists as a router, the depth is in the slug pages. **Cards still carry `dataPending: true` indicating placeholder content.**

---

## / (Approvals inbox) — `web/app/(app)/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 7 | 7 | 0 | Unchanged — `BrandLoudHero` + "Your AI teammates need a yes or no" sub. |
| APPROVAL-FIRST | 10 | 10 | 0 | Unchanged — every primitive enforces the pillar (pending count, trust ratchet, bundles, three-action cards, batch undo). |
| AUDIT-FIRST | 6 | 6 | 0 | Unchanged — decisions flow through `submitDecision` which writes audit rows, but the inbox itself doesn't surface "this decision will be hash-logged" inline. |
| ENTERPRISE-GRADE | 7 | 7 | 0 | Unchanged — redaction-by-policy, `RequestElevatedView`, data source chip. |
| SINGLE BINARY | 2 | 2 | 0 | Unchanged — zero mention. |
| VIRAL / EPIC UX | 8 | 8 | 0 | Unchanged — `BrandLoudHero` + 3 KPIs + pulse dot. New "Pulling your inbox…" intermediate state (line 530-535) reduces a wave-1 demo-record bug where "Inbox zero" flashed for 3-4 sec while data loaded. |

**Wave-1 avg:** 6.7 → **Wave-6 avg:** 6.7 (Δ 0.0)
**Verdict:** Stable. The demo-record loading-flash fix is a quality improvement that doesn't move pillar scores but improves the demo recording.

---

## /team — `web/app/(app)/team/page.tsx` + `TeammateCard.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 10 | 10 | 0 | Unchanged at score, but the underlying delivery is significantly stronger. `TeammateCard` (post-#1900) now carries: gradient banner (purple→pink), weight-900 level number, animated shine sweep, large 92px portrait avatar overlapping banner, **trait chips per worker** (`TRAITS_BY_NAME` map at L50-57 covers Mariah/Priya/Jamal/Nori/Sam/Rio with curated traits), **level number** (`LEVEL_BY_NAME` at L59-66), name + role tagline + status pill + Today bullets + Tomorrow. This is the **biggest single-component upgrade in the audit period** — wave-1 had `TeammateCard` showing avatar + name + status + bullets; wave-6 has it as the C2-holographic-showcase trading card. |
| APPROVAL-FIRST | 5 | 5 | 0 | Unchanged — status pill includes "Waiting on you" but no flow surfaced. |
| AUDIT-FIRST | 3 | 3 | 0 | Unchanged — not surfaced on this page. |
| ENTERPRISE-GRADE | 3 | 3 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 9 | 10 | +1 | **Bump.** Wave-1 noted "C2 holographic-showcase design language per the design-lock memory" was the spec but wave-1 source `TeammateCard` was a flat list card. Wave-6 has the full implementation: gradient banner + 56px level number + animated shine + 92px portrait + trait chips. **This is now the viral screenshot moment the design lock promised.** |

**Wave-1 avg:** 5.2 → **Wave-6 avg:** 5.3 (Δ +0.2)
**Verdict:** Teammates + Viral pillars now at maximum (10+10). Approval/Audit/Enterprise/Single-Binary still all under 5 — the team page deliberately stays as a roster and doesn't try to carry the other pillars. The C2 reskin is the biggest visible improvement in the audit period for any single surface.

---

## /workers/[workerId]/overview — `WorkerPersonalityCard` + `WorkerOverviewContent` + `RecentChangesPanel`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 10 | 10 | 0 | Unchanged — `WorkerPersonalityCard` retains the wave-1 C2-personality lock: trait chips, "Great at" / "Escalate when" / "Signature behaviors" per worker. |
| APPROVAL-FIRST | 7 | 7 | 0 | Unchanged — "Escalate when" list is the visible articulation of the approval pillar in worker-voice. |
| AUDIT-FIRST | 4 | 5 | +1 | Slight bump. The page now includes `RecentChangesPanel` (`page.tsx:5` import + `:28` render) which shows last 14 days of retrains with relative timestamps + "Undo" button for entries inside the 14-day TTL. This is audit-adjacent — every retrain is a recorded change with provenance — visible at the worker level. Not the hash chain, but a step toward "every change to a teammate is logged and reversible". |
| ENTERPRISE-GRADE | 4 | 4 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 10 | 10 | 0 | Unchanged — LinkedIn-profile-of-an-AI-colleague register, gradient stripe + Heart icon + colleague-language. |

**Wave-1 avg:** 6.0 → **Wave-6 avg:** 6.2 (Δ +0.2)
**Verdict:** Marginal improvement via the RecentChangesPanel addition. Still the highest-viral-payoff surface in the app.

---

## /workflows — `web/app/(app)/workflows/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 7 | 7 | 0 | Unchanged. |
| APPROVAL-FIRST | 9 | 9 | 0 | Unchanged — "Approval required" / "Autonomous" risk badges on every card. |
| AUDIT-FIRST | 5 | 5 | 0 | Unchanged — version chips visible. |
| ENTERPRISE-GRADE | 4 | 4 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 9 | 9 | 0 | Unchanged. Templates CTA "Browse 50+ templates" linking to `/workflows/templates` (L469-476) is a quiet addition that strengthens the marketplace feel. |

**Wave-1 avg:** 5.8 → **Wave-6 avg:** 5.8 (Δ 0.0)
**Verdict:** No change.

---

## /workflows/.../build — `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 4 | 4 | 0 | Unchanged. |
| APPROVAL-FIRST | 9 | 9 | 0 | Unchanged — approval-gate is a first-class step kind. |
| AUDIT-FIRST | 3 | 3 | 0 | Unchanged. |
| ENTERPRISE-GRADE | 4 | 4 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 10 | 10 | 0 | Unchanged. Stub `handleProposeChange` now uses a sonner `toast.loading` with full prompt echo + "Proposal ready for review" success state (L303-322) — meaningful feedback fix for the demo recording (wave-1 noted the stub "just timeouts at L307-311"). |

**Wave-1 avg:** 5.2 → **Wave-6 avg:** 5.2 (Δ 0.0)
**Verdict:** The toast-feedback fix is a demo-record polish that doesn't move pillar scores but means the prompt-bar interaction works on camera now.

---

## /audit — `web/app/(app)/audit/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 5 | 5 | 0 | Unchanged. |
| APPROVAL-FIRST | 7 | 7 | 0 | Unchanged. |
| AUDIT-FIRST | 10 | 10 | 0 | Unchanged. `ProvenanceBlock` confirmed at `web/components/skycot/audit/ProvenanceBlock.tsx` — "Hashes pinpoint the exact code, prompt, skill, memory, and policy that produced this action" + "Copy hash chain" clipboard button (L54-56) + 5 rows (Model / Prompt template / Skill version / Memory snapshot / Policy version). On the detail page, not the index — confirmed gap from wave-1. |
| ENTERPRISE-GRADE | 9 | 9 | 0 | Unchanged. |
| SINGLE BINARY | 2 | 2 | 0 | Unchanged. |
| VIRAL / EPIC UX | 7 | 7 | 0 | Unchanged. |

**Wave-1 avg:** 6.7 → **Wave-6 avg:** 6.7 (Δ 0.0)
**Verdict:** Stable. Locks the audit pillar at 10/10 via the detail-page `ProvenanceBlock`. The index page itself still doesn't surface "this row's hash chain" — buyers have to drill in.

---

## /skills — `web/app/(app)/skills/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 5 | 5 | 0 | Unchanged. |
| APPROVAL-FIRST | 6 | 6 | 0 | Unchanged. |
| AUDIT-FIRST | 4 | 4 | 0 | Unchanged. |
| ENTERPRISE-GRADE | 4 | 4 | 0 | Unchanged. |
| SINGLE BINARY | 1 | 1 | 0 | Unchanged. |
| VIRAL / EPIC UX | 9 | 9 | 0 | Unchanged. |

**Wave-1 avg:** 4.8 → **Wave-6 avg:** 4.8 (Δ 0.0)
**Verdict:** No change. Still mission-pillar-thin — skills float without owning workers, without explicit approval semantics, without audit lineage. Lowest-scoring in-app surface that hasn't been touched.

---

## /onboarding (Welcome) — `web/app/(setup)/onboarding/welcome/page.tsx`

| Pillar | Wave-1 | Wave-6 | Delta | Evidence |
|---|---|---|---|---|
| TEAMMATES | 8 | 9 | +1 | Bump — welcome page now uses `OnboardingShell` wrapping `WizardEditorialShell` (the locked F1 editorial design from /hire), so the brand-loud Audiowide hero + gradient + warm radial-blob background is shared across all 5 onboarding steps. "Welcome to SKYCOT." hero + "Let's hire your first worker. (90 seconds.)" sub. The pillar cards (next row) reinforce "your AI hires are colleagues" framing. |
| APPROVAL-FIRST | 4 | 4 | 0 | Unchanged — no example approval gate flow during onboarding. |
| AUDIT-FIRST | 1 | 7 | **+6** | **MAJOR JUMP.** Welcome page now has a 3-pillar card row (`testid="onboarding-pillar-audit-first"` at L37): "Audit-first — Every action signed and replayable — SOC 2-ready from day one." The mission promise "Audit-first" surfaced explicitly in step 1 of onboarding. This was the wave-1 finding that scored a 1 — the operator was told "Audit-first" on landing then onboarded with zero mention. **Closed.** |
| ENTERPRISE-GRADE | 2 | 7 | **+5** | **MAJOR JUMP.** `testid="onboarding-pillar-enterprise-grade"` card (L56): "Enterprise-grade — Multi-tenant, capability-based RBAC, GDPR Article 30 trails." Explicit enterprise pillar surfaced in onboarding step 1. |
| SINGLE BINARY | 1 | 7 | **+6** | **MAJOR JUMP.** `testid="onboarding-pillar-single-binary"` card (L47): "Single-binary — One Rust process. No Kubernetes, no sidecars, no surprises." The promise from landing/pricing is now reinforced at the moment the operator commits to onboarding. |
| VIRAL / EPIC UX | 7 | 9 | +2 | Bump — the F1 editorial-center shell unifies the 5 onboarding pages with the locked /hire design language (Audiowide hero + purple→pink gradient + warm radial-blob bg + brand-trust footer + SAVE & EXIT). Per the memory: "all 13 in-product files" got the D3 brand-consistency pass; onboarding is now in that consistent register. |

**Wave-1 avg:** 3.8 → **Wave-6 avg:** 7.2 (Δ **+3.4**)
**Verdict:** **The biggest mission-delivery improvement in the entire audit.** Wave-1's "worst-scoring surface in the audit by a clear margin" is now mid-pack. The wave-1 critical gap — "operator is told 'Audit-first. Single-binary. Enterprise-grade.' on landing, then onboarded with zero mention" — is fully closed. Three explicit pillar cards with copy that ties back to the mission statement, surfaced as the second thing the new operator sees after the hero.

---

## Wave-6 summary table

| Surface | Wave-1 | Wave-6 | Δ | Worst pillar (wave-6) |
|---|---|---|---|---|
| /compare | 9.5 | 9.5 | 0.0 | (none below 8) |
| /pricing | 8.0 | 8.0 | 0.0 | Approval-first (6) |
| /landing | 7.8 | 7.8 | 0.0 | Single binary (5) |
| /onboarding | 3.8 | **7.2** | **+3.4** | Approval-first (4) |
| /customers | 7.3 | 6.8 | -0.5 | Single binary (1) |
| /audit | 6.7 | 6.7 | 0.0 | Single binary (2) |
| / (approvals) | 6.7 | 6.7 | 0.0 | Single binary (2) |
| /workers/[id]/overview | 6.0 | 6.2 | +0.2 | Single binary (1) |
| /workflows | 5.8 | 5.8 | 0.0 | Single binary (1) |
| /team | 5.2 | 5.3 | +0.2 | Single binary (1) |
| /workflows/.../build | 5.2 | 5.2 | 0.0 | Single binary (1) |
| /skills | 4.8 | 4.8 | 0.0 | Single binary (1) |

**Overall mission-delivery score (12-surface mean):**
- Wave-1: 6.40 / 10
- Wave-6: **6.67 / 10**
- **Δ +0.27** (rounded; the unweighted mean across 12 surfaces)

**Plus** the structural fix that doesn't show in the mean: `/compare` (9.5/10) is now publicly visible, which materially changes the funnel.

---

## Top 3 surfaces that improved most

1. **/onboarding** — wave-1 3.8 → wave-6 **7.2** (Δ **+3.4**). Three mission pillars (Audit-first, Enterprise-grade, Single-binary) jumped from 1-2 to 7 each. The wave-1 critical funnel gap is closed.
2. **/workers/[id]/overview** — wave-1 6.0 → wave-6 6.2 (Δ +0.2). RecentChangesPanel adds an audit-adjacent surface; trait-chip layer underneath was already in wave-1 spec.
3. **/team** — wave-1 5.2 → wave-6 5.3 (Δ +0.2). Viral pillar bumped 9 → 10 via the full C2-holographic-showcase port (gradient banner + level number + trait chips + animated shine).

(Honourable mention: **/compare** — score unchanged but is now PUBLIC. The middleware fix in `web/middleware.ts:16-29` is the biggest funnel-reach improvement in the audit period.)

---

## Top 3 surfaces still scoring poorly

1. **/skills (4.8)** — single-binary 1, audit-first 4, enterprise-grade 4, approval-first 6. Beautiful marketplace UX but mission-pillar-thin. Skills are abstract artifacts floating without owning workers or surfaced approval/audit semantics.
2. **/workflows/.../build (5.2)** — single-binary 1, audit-first 3, teammates 4. The class-leading visual builder is workflow-shaped not teammate-shaped; great viral hook (10), weak on 4 of 6 pillars.
3. **/team (5.3)** — single-binary 1, audit-first 3, enterprise-grade 3. Owns teammates + viral at 10/10 but leaves 4 pillars on the table. The "asks before acts" safety mechanism is not surfaced at the roster level.

---

## Single biggest mission-gap still open

**Single-binary is invisible in 9 of 12 surfaces.** Pillar scores for "single binary":

| Surface | Score |
|---|---|
| /compare | 10 |
| /pricing | 9 |
| /onboarding | 7 (closed in wave-6) |
| /landing | 5 |
| /audit | 2 |
| / (approvals) | 2 |
| /customers | 1 |
| /workflows | 1 |
| /workflows-build | 1 |
| /team | 1 |
| /skills | 1 |
| /workers/[id]/overview | 1 |

Only 3 surfaces score 7+ on single-binary; 8 surfaces score 1-2. This is the only true differentiator vs every SaaS competitor (Zapier / Lindy / Crew.ai are SaaS-only), and the in-app experience never mentions it. A buyer in trial mode who never visits `/compare` or `/pricing` would never learn that SKYCOT is self-hostable.

**Recommended fix (one paragraph for a brief):** Add a single bottom-of-page footer chip across all in-app surfaces — `"Self-hosted on your hardware · skycot.com/security"` — visible on /audit, /team, /workflows, /skills, / (approvals), /workers/<id>. Cost ~30 LOC across a shared `AppShellFooter` component. Score impact: +3 to +5 single-binary across 6 surfaces → 6-surface average lift of +0.5 to +0.8 per surface → overall mission-delivery score moves from 6.67 toward 7.0+.

Secondary gap: **approval-first surfaces inconsistently.** The / (approvals) inbox owns it at 10, /compare + /workflows surface it at 9, but /skills (6), /pricing (6), /onboarding (4), and /workers/[id]/overview (7) only carry it as decoration. Onboarding does not show an example approval-gate-in-action before commit; /pricing buries it in a feature bullet.

---

## Recommendation — is the mission promise demo-ready?

**Yes, with one caveat.** The mission promise — "AI teammates that live in Slack, learn from your feedback, ask before they act. Audit-first. Single-binary. Enterprise-grade." — is now demo-ready on every surface a demo path crosses:

1. **Marketing path (/landing → /compare → /pricing → /customers):** 7.8 / 9.5 / 8.0 / 6.8. The wave-1 critical funnel bug (gated /compare) is fixed. /compare scores 10 on 4 of 6 pillars and is now visible.
2. **Sign-up funnel (/auth/signup → /onboarding/*):** /onboarding bumped from 3.8 to 7.2. The wave-1 "marketing promised X, product delivered Y" gap is closed — operator sees the three mission pillars as cards in step 1.
3. **In-product demo (/ → /team → /workers/<id> → /workflows → /audit):** Approval-first owned by / (10), teammates by /team (10) + /workers (10), audit by /audit (10). Each pillar has a "star" surface that locks it.
4. **Differentiation pitch (/compare):** 9.5 / 10 — the page that wins deals.

**The caveat:** single-binary deserves one more pass. It is the only true SaaS-vs-self-host differentiator and 8 of 12 surfaces ignore it. A 30-LOC footer chip across in-app shells closes this gap and would push the overall score from 6.67 toward 7.0+.

**Demo recording posture:** ship the recording. The wave-1 P0 blockers (gated /compare, naked onboarding) are resolved. Single-binary is a P1 cleanup that doesn't block the demo but should land before V1 launch.

---

## Appendix — wave-6 source-of-truth files

- `web/middleware.ts:16-29` — public-route allowlist (`/landing`, `/marketing`, `/pricing`, `/customers`, `/compare`, `/changelog`, `/docs`, `/for/`)
- `web/app/landing/page.tsx` (1168 LOC)
- `web/app/compare/page.tsx` (480 LOC)
- `web/app/pricing/page.tsx` (811 LOC)
- `web/app/customers/page.tsx` (489 LOC) — now reads from `web/lib/content/customers.ts`
- `web/app/(app)/page.tsx` (564 LOC) — approvals inbox with new loading-flash fix
- `web/app/(app)/team/page.tsx` (463 LOC) + `web/components/skycot/TeammateCard.tsx` (580 LOC, C2-port)
- `web/app/(app)/workers/[workerId]/overview/page.tsx` (31 LOC) — adds `RecentChangesPanel`
- `web/components/skycot/worker/WorkerPersonalityCard.tsx` (265 LOC)
- `web/components/skycot/worker/RecentChangesPanel.tsx` (162 LOC)
- `web/app/(app)/workflows/page.tsx` (540 LOC) — `Browse 50+ templates` CTA added
- `web/app/(app)/workflows/[verticalId]/[name]/build/page.tsx` (726 LOC) — sonner-toast feedback in propose stub
- `web/app/(app)/audit/page.tsx` (426 LOC) + `web/components/skycot/audit/ProvenanceBlock.tsx` (hash chain)
- `web/app/(app)/skills/page.tsx` (413 LOC)
- `web/app/(setup)/onboarding/welcome/page.tsx` (81 LOC) — **3 mission-pillar cards added**
- `web/app/(setup)/onboarding/_OnboardingShell.tsx` (58 LOC) — F1 editorial shell wrap
