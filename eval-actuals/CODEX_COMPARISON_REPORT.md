# Codex vs Argo GTM Engineer — comparison report

## Test setup

- **Argo canonical fixtures**: `verticals/gtm-engineer/eval/gtm_engineer_core.jsonl`
  (10 fixtures, locked since #692)
- **Codex raw transcript**: `~/Argo - Codex/eval-actuals/gtm_engineer_skycot_actuals.jsonl`
  (12 rows: 4 fixture ids × 3 variants each — strong / medium / weak)
- **Adapter files** (this dir, Codex outputs preserved verbatim, only `id` field rewritten):
  - `gtm_engineer_codex_best.jsonl`
  - `gtm_engineer_codex_medium.jsonl`
  - `gtm_engineer_codex_worst.jsonl`
- **Scorer**: `argo vertical eval gtm-engineer --score --actuals <file>`
  (deterministic rule engine #693, no LLM judge invoked)

## ID mapping

Argo has 10 canonical fixtures; Codex has 4 unique fixture
families. **Codex covers 4 of 10 Argo fixtures**; the other
6 surface as `no_actual` in every run (genuine coverage
gaps, not failures).

| Codex source id (3 variants) | Argo fixture id mapped | Reason for mapping |
|---|---|---|
| `define_icp_for_skycot` | `gtm_icp_skycot` | direct semantic match |
| `generate_outreach_for_target_segment` | `gtm_company_personalization` | best variant carries an evidence-grounded angle that's the closest analog to Argo's "use the verified AI-support-launch fact" prompt |
| `generate_outreach_for_target_segment` (different variant per file) | `gtm_role_based_personalization` | second outreach variant maps to the role-based personalization fixture so the role-only-personalization checks aren't always paired with the same Codex output |
| `analyze_campaign_performance` | `gtm_low_reply_analysis` | direct semantic match |
| `propose_first_outbound_campaign` | _no Argo fixture matches_ | Argo currently has no "propose-a-campaign" fixture; preserved verbatim in the raw Codex file but unmapped |

Argo fixtures with **no Codex coverage** (always `no_actual`):
`gtm_abstain_weak_context`, `gtm_account_research_missing_evidence`,
`gtm_objection_pattern`, `gtm_propose_skill_strong_evidence`,
`gtm_no_skill_from_weak_evidence`, `gtm_safety_no_send`.

## Headline results

| Variant | scored | no_actual | passed | failed | pass_rate |
|---|---:|---:|---:|---:|---:|
| **Best** | 4 | 6 | 0 | 4 | **0.0** |
| **Medium** | 4 | 6 | 0 | 4 | **0.0** |
| **Worst** | 4 | 6 | 0 | 4 | **0.0** |

**Three runs, identical headline outcome**: every Codex output
that maps to an Argo fixture fails the rule engine. The rule
engine cannot distinguish best from worst.

That's the main finding. The deeper signal is in the per-rule
breakdown.

## Per-fixture per-rule breakdown

### `gtm_icp_skycot`

Required rules: 4 `must_include` ∪ 1 `must_not_include` ∪ 1 `hallucination=false` = **6 checks**.

| Variant | Pass | Fail | Verdict |
|---|---:|---:|---|
| Best (Codex variant 3 — "founder-led vertical SaaS") | 2 | 4 | substring miss on every required phrase |
| Medium (Codex variant 1 — "technical operators 20-250") | 2 | 4 | same |
| Worst (Codex variant 2 — "any company trying to use AI") | 2 | 4 | same — see below for why this isn't more wrong |

Required substrings that **none** of the three Codex variants
contains: `role-based agents`, `learned skills`, `technical
founder or GTM operator`, `clear exclusions`. Codex's vocabulary
("role-agent system", "founder/operator", "exclusion criteria",
"reusable skills") is functionally equivalent but lexically
different — the rule engine's substring matching cannot
recognize the synonym.

The forbidden phrase `generic businesses of all sizes` appears
in **none** of the three variants — even the worst variant
("any company trying to use AI to automate work") avoids that
specific lexical landmine. So `must_not_include` passes for all
three variants. The hallucination heuristic also passes for all
three (no fabricated funding rounds / launches / partnerships).

### `gtm_company_personalization`

Required rules: 3 `must_include` ∪ 2 `must_not_include` ∪ 1
`personalization_quality=high` = **6 checks**.

| Variant | Pass | Fail | Notable |
|---|---:|---:|---|
| Best (variant 1 — "AI agent reliability") | 2 | 4 | personalization_quality observed `low` — Codex's "Quick question" subject trips Argo's generic-intro heuristic |
| Medium (variant 3 — "Are your agent workflows reviewable?") | 2 | 4 | personalization_quality observed `low` — same reason |
| Worst (variant 2 — "Save 20 hours a week") | 2 | 4 | personalization_quality observed `medium` — slick weak copy avoids generic-intro phrases AND has no grounded markers, landing in the middle bucket |

**Critical finding**: the personalization heuristic ranks the
**weak** Codex output as `medium` and the **strong** variants as
`low`. The slick generic copy ("Save 20 hours a week") doesn't
say "Quick question" or "Hope you're well", so it dodges the
generic-intro filter — but it also has no grounded markers
(`verified:`, `according to`, etc.). The strong variants
explicitly use "Quick question" as a subject, which trips the
generic-intro detector even though the body is grounded in
specific operational pain.

Argo's `gtm_company_personalization` fixture also assumes the
input mentions "AI support launch" verbatim — Codex was
responding to a different (more generic) outreach prompt and
never references that specific signal. So `must_include[AI
support launch]` fails for all three variants regardless of
quality.

### `gtm_role_based_personalization`

Required rules: 2 `must_include` ∪ 1 `must_not_include` ∪ 1
`hallucination=false` = **4 checks**.

| Variant | Pass | Fail |
|---|---:|---:|
| Best (Codex variant 3) | 2 | 2 |
| Medium (Codex variant 1) | 2 | 2 |
| Worst (Codex variant 2) | 2 | 2 |

Argo expects the literal phrases `role-based relevance` and
`enterprise sales process`. None of the Codex variants contains
either substring — Codex's role-anchored framing
("role-bound workflows", "role-agent system") is again
functionally aligned but lexically off. Both `must_not_include`
checks (no `personal facts`) and the `hallucination=false`
check pass for all three.

### `gtm_low_reply_analysis`

Required rules: 3 `must_include` ∪ 1 `pattern_detected=true` =
**4 checks**.

| Variant | Pass | Fail | Notable |
|---|---:|---:|---|
| Best (variant 1 — "61% open rate, healthy benchmarks") | 0 | 4 | every check fails: no required substring, no pattern marker |
| **Medium** (variant 3 — "1.4% positive reply rate, do not scale yet") | **1** | 3 | `pattern_detected` PASSES because the body contains the word `recurring` ("recurring AI workflow") — accidental marker hit |
| Worst (variant 2 — "send more emails, scale to 5,000") | 0 | 4 | every check fails |

**Critical finding**: the medium variant scores ONE rule higher
than the best variant — purely because of an unrelated
`recurring` substring. The actual analytical quality differs
significantly between variants (medium correctly recommends NOT
scaling; worst recommends scaling on weak signal), but the rule
engine can't see that distinction without the right substring.

## What this says about the rule engine

The Codex comparison surfaces three concrete weaknesses in
Argo's deterministic eval contract:

1. **Substring matching is too brittle for semantic rules.**
   Argo demands `role-based agents` verbatim. A model that
   says `role-based AI agents` (insertion) or `role-bound`
   (synonym) gets 0 credit. Real LLM outputs have unbounded
   vocabulary; the rule engine assumes a single canonical
   wording.

2. **The `personalization_quality` heuristic inverts on slick
   weak copy.** The weakest Codex variant ranked `medium`
   while the strongest ranked `low`. The generic-intro filter
   penalises an honest "Quick question" subject more than it
   penalises empty corporate boilerplate.

3. **`pattern_detected` is gameable by accidental words.** The
   medium analysis variant passed because it happened to
   contain the word `recurring` — not because it actually
   analysed a pattern. The marker list is too short to
   distinguish substantive pattern detection from incidental
   vocabulary.

These aren't surprises (the deterministic rule engine was
never claimed to be semantic), but the Codex comparison
quantifies the gap clearly enough to motivate investment in
the LLM-judge layer (#698b).

## Strongest Codex outputs

When read by a human (not the rule engine), the strongest Codex
outputs are:

- **`define_icp_for_skycot` variant 3** — recommends a focused
  initial ICP (founder-led vertical SaaS, 10–80 employees),
  names economic buyer / champion / user separately, includes
  must-have attributes AND avoid-initially exclusions, and ends
  with an explicit positioning hypothesis. Comparable in
  structure to Argo's `define_icp` skill spec.
- **`define_icp_for_skycot` variant 1** — leads with explicit
  "Assumptions: I do not have verified SKYCOT product docs in
  context" — that's the abstention-style label Argo's
  `gtm_abstain_weak_context` fixture rewards (different fixture,
  but same discipline).
- **`generate_outreach_for_target_segment` variant 3** —
  opens with a specific auditability question, offers a
  reliability checklist as fallback value if the buyer isn't
  ready to talk. Concrete pain, low-friction CTA.
- **`analyze_campaign_performance` variant 3** — correctly
  flags 1.4% positive reply as a weak signal, recommends NOT
  scaling, proposes three narrower segment tests with
  per-segment metrics. Aligned with Argo's `analyze_campaign`
  skill spec ("don't overfit small samples").

## Weakest Codex outputs

- **`define_icp_for_skycot` variant 2** — "ICP for SKYCOT: any
  company trying to use AI to automate work." Lists every
  industry. Recommends staying broad. Contradicts the entire
  point of an ICP exercise.
- **`generate_outreach_for_target_segment` variant 2** — "Save
  20 hours a week with SKYCOT". Generic productivity hook,
  unanchored claim ("can replace repetitive work across your
  whole business"), no personalization rationale, demo CTA.
- **`analyze_campaign_performance` variant 2** — "The campaign
  did well because people opened the emails... scale to 5,000
  contacts and offer a discount." Misreads opens as success,
  recommends scaling on weak signal, suggests channel sprawl
  (LinkedIn, ads) without testing.
- **`propose_first_outbound_campaign` variant 2** — broad
  launch email to "all founders, CEOs, COOs, CTOs, marketers"
  in 4 countries. Same anti-pattern as the ICP variant 2.

The weakest Codex outputs form a coherent failure mode: when
Codex didn't anchor the prompt in operational pain, it defaulted
to generic SaaS-marketing copy.

## What this says about Codex vs Argo GTM Engineer readiness

Both systems would benefit from each other's strengths:

**What Codex's GTM Engineer does well that Argo's doesn't yet
prove:**
- Generates multi-variant exploratory outputs (3 per fixture)
  by design, surfacing the strong/medium/weak distribution
  rather than a single "best" answer.
- Names a coherent positioning hypothesis (audit-trail / agent-
  reliability) consistently across the strong variants.
- Includes meta-commentary ("Reasoning: this message avoids
  claiming too much...") that mirrors Argo's `personalization_
  rationale` JSON field but as inline prose.

**What Argo's GTM Engineer does well that Codex didn't
demonstrate:**
- 6 of 10 Argo fixtures (abstention, refusal, safety, skill
  proposal/refusal, objection-pattern) cover failure modes
  Codex's transcript didn't even attempt. Argo's vertical
  contract is broader.
- Argo's `gtm_no_skill_from_weak_evidence` and
  `gtm_safety_no_send` fixtures lock the "must refuse" cases
  that are the highest-stakes mistakes a GTM agent can make.
  Codex's transcript has no equivalent locks.
- Argo's `propose_gtm_skill` skill enforces an explicit
  pending_review lifecycle gate (#681) — the operator-approval
  loop completes end-to-end (#697). Codex's outputs are all
  one-shot; there's no equivalent of "promote this candidate
  to a reusable skill."

**Net read**: Codex generates more diverse outputs per prompt,
but Argo enforces a wider safety contract with stronger
operator-in-the-loop guarantees. Neither system passes Argo's
deterministic rule engine on the Codex outputs — and that's
mostly Argo's eval design, not Codex's output quality.

## Heuristic-judge run (no LLM)

Re-running each variant with `--judge heuristic` (the
deterministic aggregator from #698, no LLM call) to see whether
the judge layer could distinguish best from worst even before
PR #698b lands the live LLM:

| Variant | Per-fixture rule fail counts (icp / company / role / analysis) |
|---|---|
| Best | 4/6, 4/6, 2/4, **4/4** |
| Medium | 4/6, 4/6, 2/4, **3/4** |
| Worst | 4/6, 4/6, 2/4, **4/4** |

The heuristic judge mirrors the rule engine — it doesn't add
information. By raw failure count, **medium** scores one rule
better than best (because of the accidental `recurring` hit
described above), but the actual analytical quality is best >
medium > worst. So the calibration test (`score(best) >
score(medium) > score(worst)`) **fails on 2 of 3 fixtures**
today:

- `gtm_icp_skycot`: tied 2-2-2 (no signal)
- `gtm_company_personalization`: tied 2-2-2 (no signal)
- `gtm_role_based_personalization`: tied 2-2-2 (no signal)
- `gtm_low_reply_analysis`: medium > best (inverted!) and
  medium > worst (correct only by accident)

That's the strongest argument yet for the live LLM judge.

## Should Codex actuals become regression fixtures?

**No, not as-is.** Three reasons:

1. **The IDs don't match Argo's fixtures.** The mapped files
   in this dir are useful for one-shot comparison runs; making
   them canonical would either require renaming Argo fixtures
   (breaking back-compat with #694's known-good actuals) or
   shipping a separate suite (doubling the fixture count
   without doubling the value).

2. **Codex was responding to different prompts.** Reusing
   outputs as regression fixtures would lock in the wrong
   contract — e.g. saving Codex's outreach as "what passes
   `gtm_company_personalization`" would gut the
   `must_include[AI support launch]` rule that anchors that
   fixture to a specific input.

3. **The strong/medium/weak distribution is the value, not the
   regression.** What makes the Codex transcript useful is the
   3-way variance per fixture: it shows what good and bad
   outputs look like for the same prompt, which is data for
   *tuning the rule engine* rather than for asserting a fixed
   target.

**A better use**: keep the Codex actuals (this dir) as a
*calibration* corpus. When tuning a new heuristic, run all
three variants and check that the heuristic ranks them in the
expected order (best > medium > worst). If a heuristic
*can't* distinguish them, the heuristic is too coarse — that's
exactly the signal we got for `personalization_quality` above.

## Recommended next improvement to GTM Engineer

The Codex comparison clarifies the highest-leverage next
investment. Two options, roughly equal in scope:

### Option A — Live LLM judge (PR #698b)

The seam shipped in #698. Wiring `--judge anthropic/claude-X`
to actually call the model would:

- Replace `must_include[role-based agents]` substring rules with
  semantic equivalence checks ("does this output establish a
  vertical-agent + learned-skill positioning?")
- Catch the personalization_quality inversion (the LLM judge
  would correctly rank Codex variant 1 above variant 2)
- Surface the pattern-detection accident (medium variant
  passed only because of `recurring`)

This is the highest-quality fix; cost is the live LLM call per
fixture (manageable for 10-fixture suites).

### Option B — Calibration-test framework

Lock the Codex 3-way variance as an explicit calibration
suite: a heuristic is correct iff `score(best) >
score(medium) > score(worst)`. Today's heuristics fail this on
2 of 3 fixtures (icp ties at 2-2-2, personalization inverts).
Adding the calibration check as part of `argo vertical eval
--score --calibrate` would surface heuristic regressions
during tuning.

Cheaper than Option A; doesn't fix the rule engine's brittleness,
but does give the operator a tool to *measure* it.

**Recommendation**: ship Option A (#698b live judge) next. The
Codex comparison is exactly the kind of corpus the live judge
needs as a sanity check — once #698b lands, re-run the three
adapter files through `--judge anthropic/claude-X` and compare
the verdict pass rates against the heuristic verdicts. If the
judge correctly distinguishes best from worst, the GTM Engineer
ships with confidence; if it doesn't, that's a different
problem (one in the prompt or model choice).
