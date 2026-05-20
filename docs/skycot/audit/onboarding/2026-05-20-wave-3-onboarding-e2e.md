# Wave-3 audit #3 — onboarding + /hire end-to-end

**Date:** 2026-05-20
**Auditor:** Wave-3 agent #3 (read-only static analysis)
**Scope:** `web/app/(setup)/onboarding/*` and `web/app/(hire)/hire/*`
**Method:** read every page, every step, every CTA, every form input, every transition. No browser session — static source analysis only.
**Inputs that informed the audit:**
- wave 1 (mission-alignment): scored `/onboarding` at 3.8/10 — worst surface on the demo path.
- wave 2 (brand-consistency): flagged `/onboarding` as the most off-brand surface.
- operator memory: locked design is F1 editorial-center (Audiowide hero + #fafaff bg + #1e1b4b ink + purple→pink gradient + `WizardEditorialShell` shared component). The /hire wizard adopted it. `/onboarding` did not.

---

## Wizards in scope — flow diagrams

### `/onboarding` (5 steps)

```
/onboarding/welcome
  → router.push("/onboarding/workspace")            [welcome/page.tsx:31]
/onboarding/workspace
  → POST /api/v1/onboarding/provision               [use-provision.ts:34]
  → router.push("/onboarding/connect-provider")     [workspace/page.tsx:38]
/onboarding/connect-provider
  → POST /api/v1/onboarding/provider-key            [use-provider-key.ts:33]
  → router.push("/onboarding/pick-vertical")        [connect-provider/page.tsx:39]
  OR "Skip" → router.push("/onboarding/pick-vertical")  [connect-provider/page.tsx:47]
/onboarding/pick-vertical
  → fetch POST /api/v1/onboarding/install-vertical  [pick-vertical/page.tsx:74]
  → router.push("/onboarding/first-hire")           [pick-vertical/page.tsx:85, 92]
/onboarding/first-hire
  → POST /api/v1/onboarding/first-hire              [use-first-hire.ts:41]
  → router.push(`/team/${res.workerId}`)            [first-hire/page.tsx:47]
                                                     ^^^^^^^^^^^^^^^^^^^^^^^^
                                                     ROUTE DOES NOT EXIST — 404
```

5 hand-rolled pages, no shared shell. Layout at `web/app/(setup)/onboarding/layout.tsx` provides only a sticky header with `<ProgressDots>` and a `Sparkles`-icon brand mark (no SKYCOT wordmark, no Audiowide, no gradient).

### `/hire` (6 stages: 1 entry + 4 dynamic + done)

```
/hire (page.tsx — Step 1 "Land")
  → useStep1 → draft.update({step:2, problem:value})
  → router.push("/hire/step/2")                     [hire/page.tsx:62]
/hire/step/2 (Step2Listen)
  → goNext() → draft.update({step:3, answers})
  → router.push("/hire/step/3")                     [step/[step]/page.tsx:274]
/hire/step/3 (Step3Drafting — auto-advance)
  → handleStep3Complete → draft.update({step:4, workerDraft})
  → router.push("/hire/step/4")                     [step/[step]/page.tsx:187]
/hire/step/4 (Step4Introduce — Hire/Adjust)
  → handleStep4Commit(workerId) → draft.update({step:5})
  → router.push("/hire/step/5")                     [step/[step]/page.tsx:219]
/hire/step/5 (Step5Connect)
  → handleStep5Continue(workerId)
  → router.push(`/hire/done/${workerId}`)           [step/[step]/page.tsx:242]
  OR "Skip" → router.push(`/hire/done/${workerId}?inert=1`)  [step/[step]/page.tsx:249]
/hire/done/[workerId] (celebration — 1.5s redirect)
  → setCelebration(...) + draft.clear() + router.replace("/")  [done/[workerId]/page.tsx:55-61]
```

6 stages total. Step 1 is the entry page; Steps 2–5 are dynamic `/hire/step/[step]/page.tsx`; Step 6 is `/hire/done/[workerId]`. All hosted in `WizardEditorialShell` (the locked F1 shell).

---

## Step-by-step matrix

### `/onboarding`

| # | Step | Path | Wires correctly | Validates input | Brand-on (F1) | Mission-surfaced |
|---|---|---|---|---|---|---|
| 1 | Welcome | `/onboarding/welcome` | yes | n/a (no input) | **no** | no |
| 2 | Workspace | `/onboarding/workspace` | yes (POST `/provision`) | yes — trims both, blocks empty | **no** | no |
| 3 | Connect provider | `/onboarding/connect-provider` | yes (POST `/provider-key`) | yes — requires `sk-ant-` prefix; offers Skip → mock provider | **no** | partial — "skip uses a mock provider" hints at observability, but no audit/self-host/enterprise framing |
| 4 | Pick vertical | `/onboarding/pick-vertical` | partial — hand-rolled `fetch()` bypasses `apiPost` client, swallows 404 silently | no — choice auto-commits on click; can't go back to "no choice" | **no** | partial — "approval gates" mentioned once in card 3, once in heading copy |
| 5 | First hire | `/onboarding/first-hire` | **broken** — pushes `/team/${workerId}` to a non-existent route | yes — empty prompt blocks; bounces to step 4 if vertical missing | **no** | no |

**Onboarding brand-on score:** 1/10. Zero F1 elements. No Audiowide. No purple→pink gradient. No SKYCOT wordmark. No radial-blob bg. No `WizardEditorialShell`. Uses generic `--primary` CSS var + a `<Sparkles>` icon as the "brand mark". `ProgressDots` is a generic dot indicator — nothing like the JetBrains-Mono pip-strip in the locked shell. Layout still uses `bg-[var(--background)]` not `#fafaff`. The entire flow looks like a v0 ShadCN wireframe, not the brand.

**Onboarding mission-surfacing score:** 2/10. Audit-first / single-binary / self-host / enterprise-grade — **zero mentions across all 5 pages**. The only mission-adjacent words: "approval gates" (twice on pick-vertical), "mock provider" (once on connect-provider). No SOC 2 / GDPR trust strip. No "every action is audit-logged" reassurance. No "self-host, your data stays on your binary" framing.

### `/hire`

| # | Step | Path | Wires correctly | Validates input | Brand-on (F1) | Mission-surfaced |
|---|---|---|---|---|---|---|
| 1 | Land | `/hire` | yes (resumes mid-flow via draft TTL) | yes — `useStep1.canSubmit` | **yes** — full F1 shell | no |
| 2 | Listen | `/hire/step/2` | yes; bounces to `/hire` if no problem | yes — `step2Snapshot.canContinue` from QuestionCards | **yes** | no |
| 3 | Drafting | `/hire/step/3` | yes; auto-advances on completion | n/a (auto step) | **yes** | no |
| 4 | Introduce | `/hire/step/4` | yes; Hire button commits then advances | yes — Hire CTA gated on Step4Introduce internal state | **yes** | partial — `ChannelPreferenceRow:88` mentions "How should they reach you for approvals?" |
| 5 | Connect | `/hire/step/5` | yes; Continue/Skip both route to `/done` | yes — per-connector state | **yes** | partial — "channel for approvals" framing |
| 6 | Done | `/hire/done/[workerId]` | yes; 1.5s celebration → `/` | n/a | **yes** | no |

**Hire brand-on score:** 9/10. `WizardEditorialShell` is the locked F1 implementation in full — Audiowide hero, purple→pink→coral gradient on accent words, radial-blob bg, JetBrains-Mono step strip, A-style top bar with SKYCOT wordmark + Help + SAVE & EXIT, trust-strip footer with SOC 2 / GDPR / NO CREDIT CARD / 2,847 companies. Minor: step 3 has no progress-pip update because it's auto-advancing — but the pip strip still reads correctly.

**Hire mission-surfacing score:** 3/10. The shell's trust footer ("SOC 2 TYPE II · GDPR ARTICLE 30 · NO CREDIT CARD · 2,847 COMPANIES THIS MONTH") is the only enterprise signal. "Approvals" appears twice in body content. **Audit-first / single-binary / self-host never appear in either wizard.** "Workers ask before they act" is implicit in the genesis Q&A but never named.

---

## Top 5 broken / dead / regressive items (file:line)

1. **`/onboarding/first-hire/page.tsx:47` — push to non-existent route.** After successful hire, `router.push(\`/team/${res.workerId}\`)` — but `/team/[workerId]` does NOT exist. Only `/team/page.tsx` (index) and `/workers/[workerId]/page.tsx` (the real detail route). Result: success path 404s. Almost certainly should be `/workers/${res.workerId}` or `/hire/done/${res.workerId}` to reuse the celebration page.

2. **`/onboarding/pick-vertical/page.tsx:74` — hand-rolled fetch bypasses the `apiPost` client.** Every other onboarding call uses `lib/api/client.ts`'s `apiPost` (which centralises auth, error shape, and base URL). This one uses raw `fetch()` with `${process.env.NEXT_PUBLIC_GATEWAY_URL ?? ""}` — the empty-string fallback means a relative URL when env unset, breaking the audit promise of "all gateway traffic goes through one client". Plus the `if (!res.ok && res.status !== 404)` silently treats 404 as success — masking the fact that the install-vertical endpoint **doesn't exist on the gateway** today.

3. **Three of four onboarding endpoints have no backend.** `grep -n "POST /api/v1/onboarding/(first-hire|provider-key|install-vertical)"` returns no matches in `crates/`. Only `/provision` is implemented (at `crates/argo-cli/src/handlers/gateway/onboarding_provision.rs`). The wizard's connect-provider and pick-vertical have graceful fallbacks (Skip / 404-tolerant); the first-hire mutation will hard-fail on the live gateway and the user is stuck on step 5 with no Skip button.

4. **`/onboarding/welcome/page.tsx` has no `data-testid="onboarding-welcome-back"` and no Back affordance.** Acceptable for step 1, but inconsistent with the F1 shell which renders a Back button on every step including step 1 of `/hire` (Discard dialog handles the exit). Onboarding has zero "save & exit" / "discard" UX — if a user wants to bail mid-flow they have to use browser back, and the draft persists in `localStorage` indefinitely with no surface to clear it.

5. **`/onboarding/layout.tsx:30-36` brand mark is a `<Sparkles>` lucide icon in a solid `--primary` box.** No "SKYCOT" wordmark. No Audiowide. No gradient. Compare to `WizardEditorialShell.tsx:96-110` which renders `<span class="wiz-brand-mark">` (gradient block) + `<span class="wiz-brand-name">SKYCOT</span>` (Audiowide). Onboarding's brand mark is a v0 ShadCN placeholder.

### Honourable mentions (not in top 5 but worth flagging)

- `connect-provider/page.tsx:81-83` "Anthropic console" link opens in a new tab — but the wizard's draft is in `localStorage`, so this is safe. Still — the operator's only escape hatch on this step is "Skip for now" → mock provider. No "I'll add this later in Settings" copy explicitly states that the user can change their mind.
- `pick-vertical/page.tsx:68-96` auto-commits the install on first click — `onChoose` both calls the API AND advances the route. There's no "review your choice" affordance and no way to un-pick. Users who click the wrong tile are forced forward into first-hire and have to use Back.
- `useOnboardingDraft` store has no TTL (compare to `useHireDraft` which has 7-day TTL). Stale onboarding drafts persist forever. Not a bug today but a latent issue.

---

## Brand recipe fit, by step

The locked F1 recipe per operator memory:
- Audiowide hero typography (76px on desktop)
- Page bg `#fafaff` with warm radial-blob (purple top-right + pink bottom-left)
- Ink `#1e1b4b` for body text
- Purple → pink → coral gradient on accent words + CTA
- A-style top bar (brand mark gradient block + "SKYCOT" Audiowide wordmark + step strip + Help + SAVE & EXIT)
- Trust strip footer (NO CREDIT CARD · SOC 2 · GDPR · N companies)

**`/onboarding` matches:** 0/6. `/hire` matches: 6/6.

`/onboarding` is locked-in to a `--primary` CSS variable theme that defaults to a generic ShadCN purple-ish blue. The radial-blob bg, Audiowide font load, and gradient class are all inside `WizardEditorialShell` and not exported. To "fix" `/onboarding` brand-wise is essentially: wrap every step in `<WizardEditorialShell>`, port the field content into the body slot, drop the layout's bespoke header.

---

## Mission pillars surfaced — where?

The four mission pillars per the wave-1 audit are:
1. Workers ask before they act (approval-first)
2. Every action is audit-logged
3. Single-binary you can self-host
4. Enterprise-grade

| Pillar | `/onboarding` mentions | `/hire` mentions |
|---|---|---|
| 1. Approval-first | "approval gates" × 2 (pick-vertical/page.tsx:35, 106) | "approvals" × 2 (ChannelPreferenceRow.tsx:88, celebration redirect copy) |
| 2. Audit-logged | **0** | **0** |
| 3. Single-binary / self-host | **0** | **0** |
| 4. Enterprise-grade | **0** | "SOC 2 TYPE II · GDPR ARTICLE 30" in shell footer (`WizardEditorialShell.tsx:418-421`) |

`/onboarding` mission score 2/10. `/hire` mission score 3/10 (the shell's trust footer carries pillar 4, but pillars 2 and 3 are absent from both wizards).

---

## Highest-impact single fix

**Wrap the 5 `/onboarding` pages in `<WizardEditorialShell>` and delete the bespoke `(setup)/onboarding/layout.tsx`.**

Rationale:
- One change addresses brand-on (1→9), the trust strip immediately surfaces pillar 4 (enterprise-grade), and the unified shell gives the operator a consistent SAVE & EXIT affordance across both flows.
- The form-content of each step (workspace inputs, API-key input, vertical grid, hire textarea) ports into the shell's `children` body slot with little change — only the page-level heading/sub copy moves into the shell props.
- Aligns the two wizards a user might enter (signup → onboarding → first-hire vs. team-empty-state → /hire) so the visual language is identical regardless of which path the user took.

Bonus fix (5 minutes): change `first-hire/page.tsx:47` from `router.push(\`/team/${res.workerId}\`)` to `router.push(\`/workers/${res.workerId}\`)`. This is the dead-route regression — pure 1-line fix, no design implication.

Tertiary fix (1 hour): port the genesis Q&A interaction pattern from `/hire`'s Step 2 (which actually asks "what do you want this worker to do?") into onboarding's first-hire step, replacing the single free-text prompt. That collapses the duplication between the two wizards and gives onboarding the "approval-first / charter" framing that `/hire` already has.

---

## Recommendation in one paragraph

`/onboarding` is a v0 stub that ships at 1/10 on brand-on and 2/10 on mission-surfacing — completely silent on audit-first, single-binary, and enterprise-grade. `/hire` is brand-correct (9/10) but only partially mission-surfaced (3/10). The fastest path to a 7+/10 demo path is to consolidate the two wizards behind `WizardEditorialShell`, replace `/onboarding/first-hire` with a link into `/hire` (or merge the two), wire the dead `/team/${workerId}` route to `/workers/${workerId}`, and add one sentence per step that names the mission pillars. The backend gap (3 of 4 onboarding endpoints unimplemented) is a separate ticket but the wizard's graceful degradation hides it from demo paths today, except for first-hire which has no fallback.
