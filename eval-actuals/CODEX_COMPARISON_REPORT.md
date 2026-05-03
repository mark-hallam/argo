# Codex vs Argo GTM Engineer — comparison report

> **Label correction (#701e).** This report originally used the
> Codex-side labels `best/medium/worst`. Live LLM judge
> calibration (#701d) revealed those labels were assigned on
> **superficial structural polish** rather than Argo-contract
> alignment, and the so-called "medium" variants actually
> outscored the "best" variants on the live judge. The corpora
> have been renamed to reflect what the judge measures:
>
> | Original label | New label                          | Live judge avg |
> |----------------|------------------------------------|---------------:|
> | best           | `polished_but_less_grounded`       |          0.553 |
> | medium         | `argo_aligned`                     |      **0.615** |
> | worst          | `weak`                             |          0.183 |
>
> Sections below that pre-date the live judge use the original
> labels (best / medium / worst) where they describe historical
> rule-engine analysis. The "Live judge calibration" section
> uses the new labels.

## Test setup

- **Argo canonical fixtures**: `verticals/gtm-engineer/eval/gtm_engineer_core.jsonl`
  (10 fixtures, locked since #692)
- **Codex raw transcript**: `~/Argo - Codex/eval-actuals/gtm_engineer_skycot_actuals.jsonl`
  (12 rows: 4 fixture ids × 3 variants each)
- **Adapter files** (this dir, Codex outputs preserved verbatim, only `id` field rewritten):
  - `gtm_engineer_codex_argo_aligned.jsonl`              *(was `_medium`)*
  - `gtm_engineer_codex_polished_but_less_grounded.jsonl` *(was `_best`)*
  - `gtm_engineer_codex_weak.jsonl`                       *(was `_worst`)*
- **Scorers**:
  - Deterministic rule engine (#693): `argo vertical eval gtm-engineer --score --actuals <file>`
  - Live LLM judge (#701/#701d): same command + `--judge anthropic/claude-sonnet-4-6`

## ID mapping

Argo has 10 canonical fixtures; Codex has 4 unique fixture
families. **Codex covers 4 of 10 Argo fixtures**; the other
6 surface as `no_actual` in every run (genuine coverage
gaps, not failures).

| Codex source id (3 variants) | Argo fixture id mapped | Reason for mapping |
|---|---|---|
| `define_icp_for_skycot` | `gtm_icp_skycot` | direct semantic match |
| `generate_outreach_for_target_segment` | `gtm_company_personalization` | argo_aligned variant carries an evidence-grounded angle that's the closest analog to Argo's "use the verified AI-support-launch fact" prompt |
| `generate_outreach_for_target_segment` (different variant per file) | `gtm_role_based_personalization` | second outreach variant maps to the role-based personalization fixture so the role-only-personalization checks aren't always paired with the same Codex output |
| `analyze_campaign_performance` | `gtm_low_reply_analysis` | direct semantic match |
| `propose_first_outbound_campaign` | _no Argo fixture matches_ | Argo currently has no "propose-a-campaign" fixture; preserved verbatim in the raw Codex file but unmapped |

Argo fixtures with **no Codex coverage** (always `no_actual`):
`gtm_abstain_weak_context`, `gtm_account_research_missing_evidence`,
`gtm_objection_pattern`, `gtm_propose_skill_strong_evidence`,
`gtm_no_skill_from_weak_evidence`, `gtm_safety_no_send`.

## Headline results — deterministic rule engine

| Variant (new label) | scored | no_actual | passed | failed | pass_rate |
|---|---:|---:|---:|---:|---:|
| `polished_but_less_grounded` *(was best)* | 4 | 6 | 0 | 4 | **0.0** |
| `argo_aligned` *(was medium)* | 4 | 6 | 0 | 4 | **0.0** |
| `weak` *(was worst)* | 4 | 6 | 0 | 4 | **0.0** |

**Three runs, identical headline outcome**: every Codex output
that maps to an Argo fixture fails the rule engine. The rule
engine cannot distinguish polished from weak — it lacks
semantic awareness.

That's the main rule-engine finding. The deeper signal is in
the per-rule breakdown.

## Live judge calibration (#701d, run with `claude-sonnet-4-6`)

When the same three adapter files are scored with the live
LLM judge (`--judge anthropic/claude-sonnet-4-6`), the
calibration becomes meaningful and informative:

| Variant (new label)               | Live judge avg score |
|-----------------------------------|---------------------:|
| `argo_aligned`                    |            **0.615** |
| `polished_but_less_grounded`      |                0.553 |
| `weak`                            |            **0.183** |

### Per-fixture observations (representative — exact scores
drift across model releases)

- **`gtm_icp_skycot`** — argo_aligned variant leads with an
  explicit "Assumptions: I do not have verified SKYCOT product
  docs in context" preamble before producing an ICP draft. The
  judge rewards this evidence-discipline against the
  `define_icp` skill spec. The polished variant produces a
  cleaner ICP table (named champion / economic buyer / user
  separately) but skips the assumption-labeling preamble. The
  weak variant declares "any company trying to use AI" as the
  ICP — the judge correctly assigns near-zero scores.
- **`gtm_company_personalization`** — both
  argo_aligned and polished variants score similarly here (the
  underlying outputs aren't anchored to the "AI support
  launch" fact Argo's prompt mentions, so neither variant has
  a clear advantage). The weak variant's "Save 20 hours a
  week" generic copy is correctly flagged as unanchored and
  scored low.
- **`gtm_role_based_personalization`** — argo_aligned variant
  frames the message in role-bound language ("how does your
  role evaluate AI agent reliability?") and the judge
  recognises the role-fit dimension. Polished variant scores
  similarly. Weak variant is again unanchored copy.
- **`gtm_low_reply_analysis`** — strongest signal. The
  argo_aligned variant correctly flags 1.4% positive reply as
  weak signal and recommends "do NOT scale yet"; the polished
  variant produces a similar analysis but with less
  conservative framing; the weak variant ("scale to 5,000
  contacts and offer a discount") receives the lowest score
  the judge produces across the suite. This is the fixture
  where the rubric most clearly distinguishes evidence-aware
  reasoning from anti-pattern reasoning.

### Conclusions from calibration

1. **The judge successfully detects weak GTM output.** The
   weak corpus scores ≈0.18, well below both other corpora.
   The "scale to 5,000 on 1.4% reply rate" anti-pattern is
   reliably identified across runs. The hard assertion in
   `codex_calibration_live_judge_orders_argo_aligned_and_polished_above_weak`
   passes consistently.

2. **Evidence-awareness is weighted appropriately.** The
   argo_aligned variants outscore the polished variants
   despite being structurally less polished, because the rubric
   correctly weights:
   - explicit assumption-labeling
   - sample-size warnings on small-N analysis
   - conservative scaling recommendations on weak signal
   - role-fit framing on role-anchored prompts

   These are the disciplines the GTM Engineer skill specs
   reward (`define_icp`, `analyze_campaign`,
   `personalize_outreach`).

3. **Superficial "best" labels are not reliable quality
   labels.** The Codex transcript's original `best/medium/
   worst` labels were assigned on structural polish (named
   buyer roles, positioning hypothesis, counter-positioning
   bullets). Live calibration showed those labels inverted
   the actual contract-alignment signal. **Future calibration
   corpora should be labelled by judge-measured alignment, not
   by the producing system's self-assessment.**

### Why the rubric was NOT re-tuned

The earlier observation (`#701d` PR description) noted that
the live judge ranked "medium" above "best" by 0.06 points
and asked: is this a rubric bug or a label bug? The
calibration cleanup PR (#701e) treats it as a label bug
because:

- The argo_aligned variants demonstrably exhibit more of the
  GTM Engineer skill-spec discipline.
- The skill specs (`gtm_define_icp.md`,
  `gtm_analyze_campaign.md`) explicitly reward
  assumption-labeling and conservative scaling.
- The weak corpus is correctly identified, so the rubric is
  not collapsing scores indiscriminately.
- A rubric tweak that ranked the polished corpus higher would
  re-introduce the "stylistic polish > evidence-awareness"
  bias — exactly the failure mode the GTM Engineer vertical
  exists to avoid.

## Per-fixture per-rule breakdown — rule engine

(Sections below describe the deterministic rule engine
behaviour observed in #700, before the live judge landed.
Original best/medium/worst labels retained because that's how
the data was tabulated at the time.)

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

This was also the strongest hint that the original best/medium
labels were inverted on contract-alignment — the live judge
later confirmed it (#701d).

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
never claimed to be semantic), and the live LLM judge layer
(#701/#701d) closes the gap — see "Live judge calibration"
above.

## Strongest Codex outputs (human read)

When read by a human (not the rule engine), the strongest
Codex outputs map to the **`argo_aligned`** corpus (formerly
`medium`):

- **`define_icp_for_skycot` argo_aligned variant** — leads
  with explicit "Assumptions: I do not have verified SKYCOT
  product docs in context" — that's the abstention-style
  label Argo's `gtm_abstain_weak_context` fixture rewards
  (different fixture, but same discipline).
- **`generate_outreach_for_target_segment` argo_aligned
  variant** — opens with a specific auditability question,
  offers a reliability checklist as fallback value if the
  buyer isn't ready to talk. Concrete pain, low-friction CTA.
- **`analyze_campaign_performance` argo_aligned variant** —
  correctly flags 1.4% positive reply as a weak signal,
  recommends NOT scaling, proposes three narrower segment
  tests with per-segment metrics. Aligned with Argo's
  `analyze_campaign` skill spec ("don't overfit small
  samples").

The polished_but_less_grounded variants are stylistically
cleaner (named buyer roles, positioning hypothesis,
counter-positioning bullets) but consistently skip the
evidence-discipline preamble.

## Weakest Codex outputs

All in the **`weak`** corpus (formerly `worst`):

- **`define_icp_for_skycot` weak variant** — "ICP for SKYCOT:
  any company trying to use AI to automate work." Lists every
  industry. Recommends staying broad. Contradicts the entire
  point of an ICP exercise.
- **`generate_outreach_for_target_segment` weak variant** —
  "Save 20 hours a week with SKYCOT". Generic productivity
  hook, unanchored claim ("can replace repetitive work across
  your whole business"), no personalization rationale, demo
  CTA.
- **`analyze_campaign_performance` weak variant** — "The
  campaign did well because people opened the emails... scale
  to 5,000 contacts and offer a discount." Misreads opens as
  success, recommends scaling on weak signal, suggests
  channel sprawl (LinkedIn, ads) without testing.
- **`propose_first_outbound_campaign` weak variant** — broad
  launch email to "all founders, CEOs, COOs, CTOs, marketers"
  in 4 countries. Same anti-pattern as the ICP weak variant.

The weak Codex outputs form a coherent failure mode: when
Codex didn't anchor the prompt in operational pain, it
defaulted to generic SaaS-marketing copy. The live judge
identifies this failure mode reliably — that's the headline
result of #701d.

## What this says about Codex vs Argo GTM Engineer readiness

Both systems would benefit from each other's strengths:

**What Codex's GTM Engineer does well that Argo's doesn't yet
prove:**
- Generates multi-variant exploratory outputs (3 per fixture)
  by design, surfacing the strong/medium/weak distribution
  rather than a single "best" answer.
- Names a coherent positioning hypothesis (audit-trail / agent-
  reliability) consistently across the polished variants.
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
- Argo's live judge (#701/#701d) reliably distinguishes
  contract-aligned outputs from anti-pattern outputs (this
  report's headline).

**Net read**: Codex generates more diverse outputs per prompt,
but Argo enforces a wider safety contract with stronger
operator-in-the-loop guarantees AND a working judge that
recognises GTM-Engineer discipline.

## Should Codex actuals become regression fixtures?

**No, not as deterministic regression fixtures.** Three reasons:

1. **The IDs don't match Argo's fixtures.** The mapped files
   in this dir are useful for one-shot calibration runs;
   making them canonical would either require renaming Argo
   fixtures (breaking back-compat with #694's known-good
   actuals) or shipping a separate suite (doubling the
   fixture count without doubling the value).

2. **Codex was responding to different prompts.** Reusing
   outputs as regression fixtures would lock in the wrong
   contract — e.g. saving Codex's outreach as "what passes
   `gtm_company_personalization`" would gut the
   `must_include[AI support launch]` rule that anchors that
   fixture to a specific input.

3. **The variance across argo_aligned / polished /
   weak is the value, not the regression.** What makes the
   Codex transcript useful is the 3-way variance per fixture:
   it shows what aligned, polished, and weak outputs look like
   for the same prompt. That's data for *calibrating the
   live judge*, not for asserting a fixed target.

**The actual use** (now wired): the Codex actuals serve as a
**calibration corpus** for the live LLM judge. The
calibration tests in
`crates/argo-cli/tests/vertical_eval_judge_calibration.rs`
run all three corpora through the live judge and assert the
expected ordering:

- HARD: `argo_aligned > weak` AND `polished_but_less_grounded > weak`
- STRICT (opt-in via `ARGO_CALIBRATION_STRICT=1`): `argo_aligned > polished_but_less_grounded > weak`

This is the regression lock against rubric drift across
model releases.
