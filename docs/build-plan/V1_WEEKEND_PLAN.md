# V1.0 Weekend Plan — 48-hour ship sprint to viral-ready

**Authored**: 2026-05-16. **Window**: 48 hours from this commit. **Mode**: tokenmaxx (2 Claude Code + codex swarm; maximum parallelism; quality > speed).

**Goal:** transform SKYCOT from "real but not consumable" to "viral-ready" in one weekend, hitting all Wave 1–4 deliverables identified in the 2026-05-16 SWOT + gap analysis. **No shortcuts. Every fix at best practice. Every choice aligned to the 6 mission claims and the Jonny-Ive design discipline.**

When this plan completes:
1. All 14 audit findings (F-01 through F-14) MERGED.
2. Self-serve signup → tenant provisioning → onboarding wizard → first worker hire works end-to-end at `app.skycot.com`.
3. Marketing site live at `skycot.com` with brand identity that earns "design magic" comparison.
4. Hosted instance at `try.skycot.com` and `api.skycot.com`.
5. Stripe subscription + free/pro tiers operational (test mode minimum; production-ready submitted).
6. v1.0 demo recorded against the real, validated flow.
7. Soft-launch design-partner program ready for 3 customers.

---

## §1 — Findings (verbatim from 2026-05-16 SWOT + gap analysis)

### Strengths

- Real Rust + real SPA + real macOS-CU = ~6 months of moat over GPT wrappers.
- "Teammate, not tool" mission narrative is durable + resonant.
- 489 merged PRs + 14-finding audit closed in 1–2 days = velocity proof.
- Differentiated wedge: macOS-first autonomous CU + enterprise positioning (most competitors are E2B/Linux-only).
- Skycot.com + skycot-packs repo owned.
- SPA tech stack already at premium level: Next.js 15 + React 19 + Tailwind v4 token-based theming + Radix primitives + Inter/JetBrains Mono + Framer Motion + Zustand + React Query.
- 5 vertical packs shipped (gtm-engineer, marketing-intern, sales-sdr, customer-support, finance-ar).

### Weaknesses

- **No self-serve onboarding** — viral traffic has nowhere to land. `argo setup` is CLI-only.
- **No Stripe billing** — only events-receiver crate exists; no subscription creation, no metering.
- **No hosted instance** — every customer would need to self-deploy.
- **8 audit findings still in flight** (F-03 through F-08, F-11 through F-14). Per F-06, quiet-hours + signatures + audit-chain-required are paper.
- **Demo flow unvalidated** by a non-operator human.
- **Yesterday's smoke pass was paper** (F-10) until the F-10 brief lands.
- One-person ops team.
- No marketing site distinct from the product SPA.

### Opportunities

- Anthropic's Claude Code is IDE-bound + developer-only; SKYCOT's "non-developer hires workers" is a different lane.
- Enterprise AI agent procurement is heating up; substrate quality differentiates against wrapper-shops.
- CU-first positioning before competitors catch up.
- Vertical packs are a sticky distribution mechanism.

### Threats

- **Viral moment with no signup = permanent brand damage.**
- Investor diligence finds the audit findings — F-01 alone (CI disabled until 6hrs ago + BACKLOG WRONGLY MARKED IT SHIPPED) tanks credibility.
- Anthropic + OpenAI ship adjacent products faster than we can.
- Pen-test surfaces F-03/F-05/F-06/F-07 — any is an enterprise procurement no-go.

### Gap analysis (priority-ordered)

| # | Gap | Time | Cost of ignoring |
|---:|---|---|---|
| 1 | Land all 14 audit findings (8 still TODO/CLAIMED) | 3-5d | Paper demo, pen-test fail |
| 2 | Validate demo script E2E on fresh install | 2-4h | Recording fails mid-take |
| 3 | Self-serve signup flow | 2-4w | Viral 95% bounce |
| 4 | Hosted instance + deployment infra | 1-3w | Can't try before buying |
| 5 | Stripe subscription + metering | 1-2w | Can't monetize |
| 6 | Marketing site (skycot.com) | 1-2w | 404 at moment-of-truth |
| 7 | Doc refresh for SPA-first flow | 1w | Confusion at activation |
| 8 | Support story (Discord/Slack + ticket queue) | 1w | Post-viral support drowns |

**Original estimate**: 6–10 weeks. **Compressed to**: 48 hours via maximum parallelism + 2 Claude Code + codex swarm.

---

## §2 — Execution timeline (48-hour wall clock)

Each wave runs **in parallel where dependencies allow**. The agent swarm executes; the conductor reviews + merges; the operator validates at checkpoints.

### Hour 0–2 — Foundation (BLOCKING for all UI work)

**Briefs that ship NOW (single agent each, design-tier):**

- `claude-brand-identity-system` (~400 LOC) — wordmark + color tokens + typography scale + voice & tone guide + iconography commitments. Foundation for every visible surface.
- `claude-design-system-component-extension` (~600 LOC) — extend existing `web/components/ui/` with the 15 missing components (LandingHero, SignupForm, OnboardingStep, PricingCard, FeatureSection, TestimonialCard, etc.) using shadcn-style discipline. All consume brand tokens from `claude-brand-identity-system`.

**Conductor actions (hour 0–2):**

- Author this plan + all 25 briefs (in progress).
- Push everything to main as one mega-commit.
- Brief operator on agent-account allocation.

### Hour 2–14 — Wave 1 (audit closure) + Wave 2 start (product surface)

**Wave 1 audit closure — codex swarm (in flight, +4 still TODO):**

- F-02 codex-migration-monotonic-renumber (CLAIMED)
- F-03 codex-memory-tool-namespace-wiring (CLAIMED)
- F-04 codex-per-worker-learning-cron-runtime-spawn (CLAIMED)
- F-05 codex-notify-fallback-risk-filter (CLAIMED)
- F-09 codex-fn-loc-gate-script (CLAIMED)
- F-10 codex-macos-smoke-real-action (CLAIMED)
- F-06 codex-policy-as-code-enforcement (TODO)
- F-07 codex-channel-dispatcher-voice-completion (TODO)
- F-08 codex-type-canonicalization-wave-3 (TODO)
- F-11 codex-skill-library-prompt-selector (TODO)
- F-12 codex-code-executor-sandbox-failclose (TODO)
- F-13 codex-smoke-mission-assertions (TODO, deps F-03/04/05/06/10)
- F-14 codex-unsafe-shim-explicit-marking (TODO, deps F-01 — F-01 MERGED ✅)

**Plus: post-F-01 redness fixes (NEW briefs to be authored):**

- `codex-rustfmt-broadside` — fix workspace-wide rustfmt drift surfaced by re-activated `ci.yml`.
- `codex-span-naming-cleanup` — fix raw span-literal violations.

**Wave 2 product surface — Claude swarm (lane A):**

- `claude-spa-signup-flow` (~800 LOC) — Magic-link signup at `app.skycot.com/signup` → JWT-issued session.
- `claude-tenant-provisioning-endpoint` (~600 LOC) — Backend: receive signup → create tenant → seed default workspace + first vertical pack.
- `claude-onboarding-wizard` (~1200 LOC) — 5-step SPA wizard: welcome → provider keys → vertical pick → first hire → activation.
- `claude-magic-link-auth-backend` (~400 LOC) — Magic-link email + token exchange + bcrypt-equivalent + rate limit + expiry.
- `claude-pricing-plans-config` (~250 LOC) — Free/Pro/Enterprise tier definitions + feature gates + per-tier limits.

### Hour 8–24 — Wave 2 continues + Wave 3 starts (brand + marketing)

**Wave 2 hosting + billing — Claude swarm (lane A) + codex (where applicable):**

- `codex-hosted-deployment-flyio` (~400 LOC) — Fly.io Dockerfile + fly.toml + secrets + multi-region config for argo gateway.
- `codex-spa-vercel-deployment` (~250 LOC) — Vercel config + env vars + custom domain mapping for SPA.
- `codex-dns-cloudflare-setup` (~150 LOC) — DNS records (skycot.com, app.skycot.com, api.skycot.com, try.skycot.com) + Cloudflare proxy + SSL.
- `claude-stripe-subscription-integration` (~1500 LOC) — Stripe Checkout + webhook → tenant activation; production-mode submission deferred (test mode at minimum).
- `claude-stripe-webhook-signature-verification` (~300 LOC) — Hardened webhook handler with idempotency keys + signature verification.

**Wave 3 brand + marketing — Claude swarm (lane B):**

- `claude-marketing-site-skycot-com` (~2000 LOC) — Separate Next.js marketing site at `skycot.com`. Hero + features + pricing + use cases + testimonials + sign-up CTAs. Built on the design system from `claude-brand-identity-system`.
- `claude-doc-refresh-spa-first-getting-started` (~800 LOC) — Rewrite `docs/skycot/getting_started.md` as SPA-first onboarding. CLI path becomes a developer addendum.
- `claude-changelog-and-release-notes-page` (~250 LOC) — `changelog.skycot.com` route with v1.0 release notes + roadmap.
- `claude-product-screenshots-and-imagery` (~150 LOC) — Real product screenshots (no stock photos); image-optimization pipeline.

### Hour 24–40 — Wave 3 finishes + Wave 4 starts (launch readiness)

**Wave 4 launch readiness — claude swarm + ops:**

- `claude-discord-community-setup` (~150 LOC of code; operator-side setup) — Discord server config + bot + onboarding channel + moderation rules.
- `codex-observability-prometheus-grafana` (~800 LOC) — Prometheus exporter + Grafana dashboards + critical alerts.
- `claude-status-page-skycot` (~400 LOC) — `status.skycot.com` — uptime + incident timeline.
- `claude-support-ticket-queue-integration` (~300 LOC) — Plain/Linear/Intercom integration; inbound from `support@skycot.com`.
- `claude-design-partner-program-onboarding` (~250 LOC) — 3-customer onboarding playbook + dedicated Slack channels.
- `claude-launch-coordination-playbook` (~150 LOC) — ProductHunt + HackerNews + Tech Twitter timing; ready for soft-launch decision.

### Hour 40–48 — Integration + dry-run + demo recording

**Integration phase:**

- All briefs merged or queued for final review.
- Operator runs full end-to-end on a fresh install (non-operator perspective): visit `skycot.com` → click "Hire your first worker" → complete signup → onboarding wizard → first worker hire → first duty fires → approval round-trip → audit trail visible.
- Any breaks → rapid-fix briefs authored + shipped.
- Final smoke run (`scripts/smoke-skycot.sh` with mission assertions from F-13) — must report 29/29 PASS with explicit-SKIP-vs-PASS distinction.

**Demo recording phase (hour 44–48):**

- Record `docs/skycot/demos/v1-mission-mvp.mp4` per the recording instructions, but against the **validated** SPA flow (not the prior aspirational script).
- Sanitize per the key-safety playbook (throwaway keys, do-not-disturb, region capture, post-recording validation, key rotation).
- Commit + push.

**v1.0 SHIPPED.**

---

## §3 — Brief catalog (everything queued for the swarm)

### Wave 0 — Foundation (authored hour 0–2)

| Brief | Owner | LOC | Dependencies |
|---|---|---:|---|
| claude-brand-identity-system | claude | 400 | — |
| claude-design-system-component-extension | claude | 600 | brand-identity-system |

### Wave 1 — Audit closure (codex swarm; 14 briefs)

Already authored + queued; codex burning through (see `BUILD_STATUS.md` for live state). Plus post-F-01 redness:

| Brief | Owner | LOC | Dependencies |
|---|---|---:|---|
| codex-rustfmt-broadside | codex | ~200 | F-01 MERGED |
| codex-span-naming-cleanup | codex | ~250 | F-01 MERGED |

### Wave 2 — Product surface (Claude lane A)

| Brief | Owner | LOC | Dependencies |
|---|---|---:|---|
| claude-spa-signup-flow | claude | 800 | brand-identity-system |
| claude-tenant-provisioning-endpoint | claude | 600 | F-04 toolcontext-tenant MERGED |
| claude-onboarding-wizard | claude | 1200 | spa-signup + tenant-provisioning |
| claude-magic-link-auth-backend | claude | 400 | tenant-provisioning |
| claude-pricing-plans-config | claude | 250 | — |
| claude-stripe-subscription-integration | claude | 1500 | pricing-plans + tenant-provisioning |
| claude-stripe-webhook-signature-verification | claude | 300 | stripe-subscription |
| codex-hosted-deployment-flyio | codex | 400 | — |
| codex-spa-vercel-deployment | codex | 250 | brand-identity-system |
| codex-dns-cloudflare-setup | codex | 150 | hosted-deployment, vercel-deployment |

### Wave 3 — Brand + marketing (Claude lane B)

| Brief | Owner | LOC | Dependencies |
|---|---|---:|---|
| claude-marketing-site-skycot-com | claude | 2000 | brand-identity-system + design-system-component-extension |
| claude-doc-refresh-spa-first-getting-started | claude | 800 | onboarding-wizard SHIPPED |
| claude-changelog-and-release-notes-page | claude | 250 | marketing-site |
| claude-product-screenshots-and-imagery | claude | 150 | onboarding-wizard SHIPPED |

### Wave 4 — Launch readiness

| Brief | Owner | LOC | Dependencies |
|---|---|---:|---|
| claude-discord-community-setup | claude | 150 | brand-identity-system |
| codex-observability-prometheus-grafana | codex | 800 | F-01 MERGED |
| claude-status-page-skycot | claude | 400 | brand-identity-system + hosted-deployment |
| claude-support-ticket-queue-integration | claude | 300 | tenant-provisioning |
| claude-design-partner-program-onboarding | claude | 250 | onboarding-wizard |
| claude-launch-coordination-playbook | claude | 150 | — (operator-side artifact) |

### Wave 5 — Integration + final fixes (hour 40–48)

Authored on-demand as breaks surface during operator E2E validation. Reserved budget: ~5 briefs × 200 LOC each = ~1000 LOC.

**Total budget across all waves: ~14,000 LOC.** Distributed across ~32 briefs. Comfortably within Claude Code + codex swarm capacity for 48 hours.

---

## §4 — Brand design system (Jonny-Ive discipline)

**Core principle:** "Materials honesty. Reduction. Care in details. Form follows function — but joyfully."

### Typography

- **Display:** Inter Display (Inter with display optical sizing). For hero text, page titles, marketing headlines.
- **Body:** Inter, 16px base, 1.5 line height. Generous reading width: max 65ch.
- **Mono:** JetBrains Mono. For code, terminal output, audit-row keys.
- **Scale (8pt grid):**
  - Display: 72px / 60px / 48px / 36px
  - Heading: 30px / 24px / 20px / 18px
  - Body: 16px (default) / 14px (compact) / 12px (caption)
  - Mono: 14px (default) / 12px (inline)

### Color palette

**Discipline:** monochrome base + warmth + one signature accent. NO rainbow.

- **Surface:** `--background: oklch(0.99 0 0)` (near-white, warm). `--background-dark: oklch(0.13 0 0)`.
- **Foreground:** `--foreground: oklch(0.15 0 0)`. Dark mode flips.
- **Muted:** warm grays for secondary information.
- **Brand accent:** ONE color, selected from `oklch(0.65 0.16 256)` (deep indigo) OR `oklch(0.68 0.18 28)` (warm coral). Operator picks during foundation hour.
- **Semantic:** success (green), warn (amber), error (red), info (blue) — only used for status, never decoration.

**Forbidden:**
- Gradient hero backgrounds with rainbow.
- Drop shadows that lack purpose.
- Borders that don't separate hierarchy.
- More than 1 accent color in a single view.

### Iconography

- **Single icon family:** Lucide React (already in deps). No mixing with Heroicons or custom icons unless the design system explicitly extends.
- **Style:** 1.5px stroke, sized to text x-height. Never decorative.

### Motion

- **Subtle, intentional.** Framer Motion (already in deps). Default `ease-in-out`, durations 150-300ms.
- **Allowed:** fade-in, slide-in, scale on hover (≤1.02), accordion expand.
- **Forbidden:** parallax, scroll-jacking, attention-stealing entry animations, "wow factor" for its own sake.

### Component primitives (live in `web/components/ui/`)

Existing baseline (already shipped):
- `Button.tsx`, `Card.tsx`, `Avatar.tsx`, `AlertDialog.tsx`, `Breadcrumb.tsx`, `DataCell.tsx`, `NavDrawer.tsx`, `OAuthConnectorRow.tsx`, `ProgressDots.tsx`, `StatusPill.tsx`, `Toast.tsx`

To add (`claude-design-system-component-extension` brief):
- `LandingHero.tsx` — marketing hero with restrained typography + CTA.
- `SignupForm.tsx` — magic-link email entry.
- `OnboardingStep.tsx` — wizard step with progress indicator.
- `PricingCard.tsx` — pricing tier display.
- `FeatureSection.tsx` — feature highlight with icon + headline + body.
- `TestimonialCard.tsx` — customer quote display.
- `KeyFact.tsx` — single stat display (e.g., "489 PRs merged in 2 weeks").
- `CodeBlock.tsx` — syntax-highlighted terminal/code display.
- `Diagram.tsx` — wraps inline SVG diagrams (the existing 5 architecture SVGs).
- `Footer.tsx` — site footer with links + social.
- `Nav.tsx` — top navigation, sticky, with mobile burger.
- `ChannelBadge.tsx` — display Slack/email/SMS/voice icons.
- `AuditTimeline.tsx` — chronological audit row display.
- `WorkerCard.tsx` — display worker portrait + status + duties.
- `ApprovalCard.tsx` — pending-approval display (mobile-friendly).

### Voice & tone

- **Direct:** "Hire your first worker" not "Get started today!"
- **Honest:** "Maya is asking for your approval — Slack reply will land it." not "AI-powered automation in real time!"
- **Intelligent:** assumes the reader knows what an AI agent is + what a vertical means.
- **Restrained:** no exclamation points outside error states. No emoji clutter. The product speaks through its work, not its copy.

Reference comparisons: Linear, Vercel, Stripe, Anthropic.com, Apple product pages.

---

## §5 — Agent swarm strategy

### Account allocation

**Claude lane A (product surface):**
- Wave 2 briefs: spa-signup-flow, tenant-provisioning-endpoint, onboarding-wizard, magic-link-auth-backend, pricing-plans-config, stripe-subscription-integration, stripe-webhook-signature-verification
- Wave 5 fixes (50% reserve)

**Claude lane B (brand + marketing + launch):**
- Wave 0 briefs: brand-identity-system, design-system-component-extension
- Wave 3 briefs: marketing-site-skycot-com, doc-refresh, changelog-page, product-screenshots
- Wave 4 briefs: discord-community-setup, status-page-skycot, support-ticket-queue, design-partner-onboarding, launch-coordination
- Wave 5 fixes (50% reserve)

**Codex swarm (existing):**
- Wave 1 audit closure (14 briefs, 4 still TODO)
- Wave 2 ops briefs: hosted-deployment-flyio, spa-vercel-deployment, dns-cloudflare-setup
- Wave 4 observability: observability-prometheus-grafana
- Plus rustfmt + span-naming cleanup post-F-01

### Parallelism limits

- Cargo lock contention is the biggest sequential bottleneck for Rust. Mitigate with per-brief `CARGO_TARGET_DIR` (already standard pattern).
- Next.js builds are CPU-bound but per-package; multiple SPA agents can work in different `web/app/(group)/` subdirs without conflict.
- Brand identity is FOUNDATIONAL — must merge in hour 0–2 OR everything downstream uses placeholder tokens.

### Conductor workflow (me)

- **Every 15 min:** pull main, check ledger state, Tier-1 review any IN_REVIEW PR, admin-merge if clean, flip ledger row, push.
- **Every 30 min:** survey CLAIMED briefs for stale-claim signals (no commits in 30+ min on a claim).
- **On BLOCK:** investigate + amend may-touch inline + unblock; OR drop if scope-stale.
- **On Wave 5 break:** author rapid-fix brief; assign to next free agent.

---

## §6 — Quality gates

### Per-brief acceptance (every brief must satisfy)

- [ ] Files match may-touch list (Tier-1 boundary check).
- [ ] No file > 500 LOC post-change.
- [ ] No fn > 80 LOC post-change.
- [ ] All acceptance ticks in PR body.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean (Rust) OR `pnpm lint && pnpm type-check` clean (SPA).
- [ ] Test coverage proportional to surface area added.
- [ ] PR body explicitly notes any boundary deviations.

### Pre-launch gates (must pass before recording demo)

- [ ] All 14 audit findings MERGED (F-01 ✅, F-02 through F-14).
- [ ] `scripts/smoke-skycot.sh` reports 29/29 PASS with SKIP-vs-PASS distinction (per F-13).
- [ ] CI workflows (ci.yml, audit.yml, deny.yml, deploy-smoke.yml) green on a fresh PR.
- [ ] Self-serve signup → first worker hire flow validated by non-operator human.
- [ ] `skycot.com`, `app.skycot.com`, `api.skycot.com`, `try.skycot.com` resolve + serve.
- [ ] Stripe test-mode subscription create + cancel + webhook works end-to-end.
- [ ] Lighthouse score ≥ 90 on all marketing-site pages.
- [ ] Accessibility: axe-core clean on signup + onboarding wizard.
- [ ] Brand identity guidelines documented + applied consistently.

### Post-launch monitoring (hour 48+)

- Hosted instance uptime (Statuspage).
- Signup → first-worker-hire activation rate (analytics event).
- Sentry / error tracking on SPA + gateway.
- Discord activity + first-response SLA.

---

## §7 — Risk register

| # | Risk | Likelihood | Impact | Mitigation |
|---:|---|---|---|---|
| R1 | Stripe production approval takes >48hrs | High | Test-mode is good enough for design-partner launch; production-mode submitted in parallel, approved within week 1 |
| R2 | Cargo build contention slows Rust swarm | Medium | Per-brief `CARGO_TARGET_DIR`; existing pattern. F-01 may surface fmt drift that delays Wave 1 merges |
| R3 | DNS propagation > 1 hour | Medium | Use Cloudflare proxy (instant cache invalidation); pre-stage all DNS records in hour 0 |
| R4 | F-08 type canonicalization is 800 LOC, biggest brief | Medium | Split into 4 sub-waves (A/B/C/D); merge incrementally instead of mega-PR |
| R5 | Brand identity decision (indigo vs coral) takes too long | Low | Operator picks at hour 0 from 2 options I prepare; no longer than 10min discussion |
| R6 | Demo recording fails | Medium | Validate flow before recording; have backup screen recordings of each step; can stitch in post |
| R7 | Onboarding wizard requires LLM-side adjustments (Genesis) | High | F-04+ wires Genesis; if not enough, fallback to "pick a vertical pack" simpler flow for v1 |
| R8 | Pen-test surfaces something during 48hrs | Low | No external pen-test scheduled this weekend. If audit findings linger, document v1.x roadmap with explicit timeline |
| R9 | Operator burnout in 48hrs | Medium | Operator manages strategy + decisions; agents do work. Sleep is non-negotiable; plan validates this by minimizing operator-required decisions |
| R10 | Viral moment arrives before plan completes | Low | If it does: shut signups except for whitelisted design partners; "limited access" framing |

---

## §8 — Definition of done

The weekend ships when:

1. ✅ **Audit complete:** all 14 findings MERGED; `BUILD_STATUS.md` reflects ZERO P0/P1 issues.
2. ✅ **Smoke gate honest:** `scripts/smoke-skycot.sh` 29/29 PASS with mission assertions.
3. ✅ **Sign-up works:** `skycot.com` → email entry → magic link → tenant provisioned → onboarding wizard → first worker hired → first duty fires. End-to-end validated by a non-operator test user.
4. ✅ **Hosted:** `app.skycot.com` (SPA), `api.skycot.com` (gateway), `try.skycot.com` (demo workspace) all serving.
5. ✅ **Stripe:** test-mode subscription works; production-mode submitted.
6. ✅ **Marketing site:** `skycot.com` live with hero + features + pricing + signup CTA; Lighthouse ≥ 90.
7. ✅ **Brand identity:** documented design system; consistent across all surfaces.
8. ✅ **Demo recorded:** `docs/skycot/demos/v1-mission-mvp.mp4` against the validated flow (not aspirational script).
9. ✅ **Support infra:** Discord + status page + ticket queue operational.
10. ✅ **3 design partners** identified + onboarded (or onboarding ready).

After this: SKYCOT is **viral-ready**. Operator decides launch timing.

---

## §9 — Operator checkpoints (4 decisions to make in 48hrs)

The operator's bandwidth is the limiting reagent. These 4 decisions need explicit input; everything else swarms execute.

1. **Hour 0** — Brand accent color: deep indigo `oklch(0.65 0.16 256)` OR warm coral `oklch(0.68 0.18 28)`. (Or operator-supplied alternative within constraints.)
2. **Hour 4** — Stripe pricing tiers: Free / Pro $29/mo / Enterprise custom. (Or operator-supplied alternative.)
3. **Hour 24** — Marketing site copy review: hero + features + pricing. (Agent drafts; operator approves or redlines.)
4. **Hour 44** — Demo recording: operator records personally after validating flow works.

Everything else: agents execute against the briefs. Conductor reviews + merges. Operator validates checkpoints.

---

## §10 — Mission alignment summary

This plan exists in service of SKYCOT's 6 mission claims:

1. **Teammate, not tool** — onboarding wizard's first hire flow IS the embodiment. The first 5 minutes of every customer experience.
2. **Delegation is the unit** — onboarding wizard primes the user to articulate role/charter/duties in plain English, not "write a prompt."
3. **Platform builds workers** — Genesis pipeline triggered from onboarding's "first hire" step.
4. **Skills shared and learned** — vertical pack selection in onboarding seeds the user with proven shared skills; learning cron (F-04) ensures their workers learn over time.
5. **Real-world execution on PC/Mac** — macOS-first CU positioning matched by macOS-only demo target initially; Linux/Windows post-v1.
6. **Multi-channel approvals** — wave 2 onboarding asks user to connect Slack/email; first action triggers a real approval round-trip.

Every brief in this plan, when traced back, supports one or more of these claims. **No brief is allowed in if it doesn't.**

---

**Last refreshed**: 2026-05-16. **Status**: ACTIVE 48-hour window. **Authored by**: conductor (Claude Opus 4.7 1M context). **Operator**: Mark Hallam.
