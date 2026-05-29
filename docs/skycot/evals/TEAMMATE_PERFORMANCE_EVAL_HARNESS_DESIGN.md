# Teammate Performance Eval Harness — design (item-6)

**Status**: DESIGN — operator-aligned 2026-05-29; phase-1 build authorized. See [[DEC-017]].
**Author**: claude conductor, 2026-05-29 (item-6 of the 6-item program).
**Decisions locked** (DEC-017): two scenario-source modes — **synthetic golden-companies FIRST**, historical replay a later seam; **Portia** (the Pitch Agent, DEC-016) is the reference teammate (existing 141-case suite); **Mariah** is the first synthetic-company teammate; all six scoring axes in scope; full rollout post-MVP, phase-1 scaffold lands now.

## 0. Two scenario-source modes (operator-confirmed framing)

The harness scores a teammate against situations from one of two interchangeable sources feeding the SAME scoring engine:

- **Historical replay (later seam, strongest signal):** replay a real customer's past data (tickets / CRM / emails) + the human's actual outcome as the answer key. Needs real customer data → privacy-gated, unavailable until design-partners exist. Build the seam now, switch on later.
- **Synthetic golden-companies (built first):** generated, reproducible fixtures with known-correct expected behaviours. Generation pattern: **Opus drafts the company + scenarios → ratify the "correct answers" once → freeze as committed fixtures** (realism without losing reproducibility).

## 1. Why this exists (the operator's ask)

> "Write tests to put AI teammates through their paces… we may need to set
> up a test company / or companies… proper scope and implementation."

We can already prove the platform *works* (6 mission claims LIVE, 88 e2e specs,
141 pitch-eval cases). What we cannot yet answer crisply is: **"How GOOD is a
given teammate at its job, and did this week's changes make it better or
worse?"** That is a *performance* question about the worker's behavior, not a
*functionality* question about the platform. This harness answers it.

## 2. What already exists (extend, don't rebuild)

| Piece | What it does | Gap for teammate-performance |
|---|---|---|
| `argo-eval` (`runner.rs`, `task.rs`) | single-turn `input→expected→metric` (ExactMatch / SubstringContains / JaccardTokens); powers `argo forge bench` | scores a STRING, not a teammate's *behavior* (escalation, boundary, skill choice) |
| `argo-eval::honesty` | honesty scoring primitive | not wired into a per-teammate scorecard |
| per-vertical `eval/*.jsonl` (~18 verticals) | a `seed.jsonl` + 1 small suite each (sparse) | thin; no scenario rubric; no scorecard/regression gate |
| `packs/pitch-agent/evals` (141 cases) | the mature reference suite — proves the pattern | pitch-specific; not generalized to other teammates |
| `argo-demo-seed` | seeds personas/workers/rows into a tenant | seeds the demo, not reproducible *named golden companies* for eval |

**Principle:** the harness is a thin layer ON TOP of `argo-eval` + `argo-demo-seed`,
not a new engine. We add (a) richer scoring, (b) synthetic companies, (c) scorecards.

## 3. The unit of evaluation: a *scenario*

A **scenario** is one realistic situation a teammate faces, with a behavioral
rubric. Beyond the existing string metrics, a scenario asserts:

- **Correctness** — output matches expected (existing metrics still apply).
- **Escalation** — did it ask for human approval when (and only when) it should? (binary, both directions matter — false-confidence AND over-escalation are failures).
- **Boundary adherence** — stayed within its capability/truth boundary (reuses the pitch `evaluate_claim` primitive — already shipped).
- **Skill usage** — invoked the expected skill(s); didn't invent capability.
- **Honesty** — flagged uncertainty rather than fabricating (reuses `argo-eval::honesty`).
- **Budget** — within a per-scenario latency + cost ceiling (reuses `argo-forge::cost`, now incl. the Opus-4.8 pricing pinned this session).

Each scenario yields a per-axis score; a teammate's **scorecard** aggregates
across its scenario suite.

## 4. Synthetic test companies — THE key decision

A teammate only performs realistically against realistic context. We need
reproducible "companies." Three options:

- **Option A — one shared golden company** ("Northwind"): a single seeded
  tenant with cross-functional data (listings, tickets, candidates, invoices)
  that every teammate evaluates against. *Cheapest; least realistic per-role.*
- **Option B — one golden company per vertical-family** (e.g. "Northwind
  Marketplace" for Mariah, "Acme Support" for Jamal, "Talently" for the
  recruiter). Each is a small, fixed, version-controlled fixture seeded via an
  extension of `argo-demo-seed`. *Recommended — realistic, still bounded.*
- **Option C — generative companies** (LLM-synthesized context per run).
  *Most realistic, but non-reproducible → bad for regression gating. Reject for v1.*

**Recommendation: Option B.** 3–5 named golden companies as committed JSONL
fixtures under `eval/companies/<name>/`, seeded into an isolated eval tenant.
Reproducible (regression-safe), realistic per-role, and reuses the demo-seed
machinery we already trust. No-theater compliant: real backend, seeded real
rows, no runtime mocks.

## 5. Architecture

```
eval/companies/<company>/         # committed golden fixtures (JSONL)
  context.jsonl                   # CRM rows / tickets / listings / policies
  team.jsonl                      # which teammates operate here
eval/scenarios/<teammate>/        # scenario suites (one JSONL per teammate)
  *.jsonl                         # {situation, expected_action, should_escalate,
                                  #  required_skills, boundary, budget_ms, budget_usd}
crates/argo-eval/src/
  teammate.rs (NEW)               # TeammateScenario + behavioral scoring axes
  scorecard.rs (NEW)              # per-teammate aggregate + regression delta
  bin/teammate-eval.rs (NEW)      # `argo forge bench --teammate <name>` entrypoint
```

- Runs against an **isolated eval tenant** (seeded from `eval/companies/`), never
  the demo tenant — keeps eval data out of the demo recording path.
- Reuses `EvalSubject` (wraps the worker's `Agent` with its real charter +
  skills), `evaluate_claim` (boundary), `honesty`, and `cost`.
- Output: a JSON scorecard per teammate + a regression delta vs the last
  committed baseline.

## 6. CI / regression gate

- A teammate's scorecard baseline is committed (e.g. `eval/baselines/<teammate>.json`).
- `argo forge bench --teammate <name>` re-runs the suite; CI fails if any axis
  regresses beyond a tolerance (e.g. correctness −1 case, or escalation-precision drop).
- Mirrors the existing pitch-eval gate (`scripts/check-pitch-eval.sh`) and the
  evaluator-round cadence — fits the swarm protocol without new ceremony.

## 7. Proposed phasing (design-first; nothing built until you align)

- **Phase 1 (prove the pattern, 1 teammate):** build `teammate.rs` + `scorecard.rs`,
  ONE golden company ("Northwind Marketplace") + ONE teammate scenario suite
  (Mariah, ~10 scenarios), the `teammate-eval` bin, and a committed baseline.
  Generalizes the pitch-eval pattern to a non-pitch teammate.
- **Phase 2 (breadth):** golden companies + suites for the demo teammates
  (Jamal/support, Peta/PM, Priya/recruiter), + CI gate.
- **Phase 3 (depth):** multi-turn scenarios, self-healing loop integration
  (`self_healing.rs` already exists), operator-facing "teammate report card" UI.

## 8. Open questions — RESOLVED 2026-05-29 (DEC-017)

1. **Synthetic-company approach** — ✅ Option B (per-family golden companies), synthetic first.
2. **Reference + first teammate** — ✅ Portia (Pitch Agent, existing 141 cases) is the reference; Mariah (marketplace) is the first synthetic-company teammate.
3. **Behavioral axes for v1** — ✅ all six (§3).
4. **v1.0-blocking?** — ✅ post-MVP instrument; phase-1 scaffold lands now, full rollout after the demo MP4 records.
