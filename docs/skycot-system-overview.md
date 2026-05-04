# SKYCOT — System Overview

> **AI employees that learn the job.**
> From work → to learning → to reusable skills.

This document is the canonical source for the launch-quality
PDF at `docs/skycot-system-overview.pdf`. Edit this file, then
regenerate the PDF with:

```
bash docs/build-skycot-overview.sh
```

---

## 1. What SKYCOT is

SKYCOT builds **role-based AI employees** — agents hired into a
specific job, given real work, evaluated by a human manager, and
rewarded by adding what they learned to a reusable skill library.

Most AI tools generate outputs. SKYCOT employees improve through
work.

| Most AI            | SKYCOT                            |
| ------------------ | --------------------------------- |
| Generates outputs  | Performs real work                |
| Forgets what worked| Remembers patterns                |
| Re-derives every prompt | Builds reusable skills       |
| Untracked quality  | Evaluated like real work          |

---

## 2. The core loop

Every SKYCOT agent runs the same closed loop from work to
learning to next-run improvement.

```
role → work → pattern → skill → approval → reuse → improved next run
```

- **Role** — the agent is bound to a vertical (e.g. GTM Engineer)
  with a deny list, an allow list, a memory schema, and bundled
  skills attached at session-creation time.
- **Work** — the agent performs real tasks (analysis, drafting,
  research) inside that vertical's safety boundary.
- **Pattern** — across ≥3 reinforcing observations, the agent's
  pattern-detection skill identifies a stable signal.
- **Skill** — a propose-skill meta-skill emits a candidate to
  memory in `pending_review` state. **Never auto-active.**
- **Approval** — the operator reviews the candidate and promotes
  it. One CLI call flips lifecycle_state to `active`.
- **Reuse** — the next agent turn whose context matches the
  skill's trigger conditions consults the new skill via the same
  query the runtime always uses.
- **Improved next run** — the agent's output incorporates the
  learned pattern. The library compounds.

---

## 3. Example: the GTM Engineer

The first shipped role. The GTM Engineer is bound to eight
bundled skills:

- **define_icp** — defines or refines an ICP from product,
  market, known customers, constraints. Flags evidence gaps.
- **find_target_accounts** — searches for candidate accounts
  matching the ICP.
- **research_company** — per-account deep research: signals,
  pain hypothesis, fit scoring.
- **research_person** — per-role persona research:
  responsibilities, buying signals, message angles.
- **generate_outreach** — drafts personalized outreach using
  verified facts and role/company research.
- **analyze_campaign** — ingests operator-reported campaign
  outcomes; surfaces what worked, proposes next experiments.
- **detect_gtm_pattern** — surfaces repeatable patterns from ≥3
  observations across distinct campaigns. Refuses single-
  instance claims.
- **propose_gtm_skill** — the meta-skill. Converts high-
  confidence patterns into reusable learned-skill candidates
  queued for operator approval.

**Safety boundary**: the vertical's manifest deny list blocks
`terminal`, `file_write`, `file_patch`, and `delegate_task`.
The GTM Engineer is draft-only by contract — no sending, no CRM
updates, no destructive tool calls reachable.

---

## 4. Real demo flow

The full GTM Engineer learning loop, recorded against real CLI
surfaces. Two equivalent paths: deterministic (mock provider) for
reproducible demos, or live (`anthropic/claude-sonnet-4-6` via
the gateway) for end-to-end LLM verification.

### Before — a weak campaign

> **100 emails sent · 2 replies · 0 positive**

The agent's diagnosis: generic productivity hook, no verified
company signal, audience-flat (\"everyone gets time back\"),
single demo CTA. The control variant landed at 0 positive
replies on 100 sends.

### Learning — pattern detected, skill proposed

Across three reinforcing variant cohorts (165 sends total), the
agent identified that leading with the agent-learning loop and
audit-chain visibility lifted positive-reply rate to 6–8% on
technical-founder targets — well above the 0-positive baseline.

The propose-skill meta-skill emitted a candidate:
`technical_founder_learning_moat_outreach`. State:
`pending_review`.

### Approval — pending_review → active

The operator reviewed the candidate's scope, rule, and guards.
One CLI call flipped lifecycle_state to `active`. The skill is
now in the catalog.

### After — improved GTM output

The next agent turn that matches the trigger conditions
(technical founder + agent-platform GTM hypothesis) consults the
active skill. The output anchors in one verified company signal,
leads with audit-chain framing, holds to ≤120 words, single low-
friction CTA. Projected uplift: +4–6 percentage points on
positive-reply rate vs the generic control.

---

## 5. The proof system

Every step of the loop produces verifiable evidence.

- **Role binding** — every session row records the bound vertical
  id; bundled skills are materialised into the skill table at
  session-creation time and are visible to the agent runner via
  the same query the prompt assembler uses on every turn.
- **Memory** — campaign outcomes, detected patterns, and skill
  candidates are vertical-namespaced and append-only. Candidates
  are immutable once written; superseded proposals get a new row.
- **Skill lifecycle** — every skill row carries a
  `lifecycle_state` column (`draft → pending_review → active →
  rejected → archived`). Illegal transitions are rejected at the
  storage layer.
- **Evaluation** — each vertical ships a fixture suite (ten
  fixtures for the GTM Engineer) with a deterministic rule-engine
  scorer plus an optional LLM judge.
- **LLM judge calibration** — a three-cohort calibration corpus
  (argo_aligned, polished_but_less_grounded, weak) locks the
  judge ranking; integration tests assert the ordering.
- **Audit trail** — every CLI surface emits structured audit
  rows. The gateway surface is an HTTP mirror of the CLI; both
  write to the same SQLite store.

---

## 6. Architecture overview

Two halves with clear roles.

### Claude / Argo

- builds and runs role-based agents
- handles vertical manifests, bundled skills, vertical-
  namespaced memory
- manages the full skill lifecycle (propose → review → promote
  → approve → active)
- runs deterministic + LLM-judge evaluations
- exposes everything via CLI and gateway HTTP
- enforces the safety contract (deny lists, allow lists)

### Codex

- generates real GTM outputs at scale (multi-variant campaigns)
- creates calibration data for the LLM judge (strong / medium /
  weak distinctions)
- validates output quality across distinct cohorts
- does the heavy producer work that the evaluation system grades

The two systems share the canonical fixture set. Codex outputs
flow into the calibration corpus that the LLM judge is tuned
against.

---

## 7. Evaluation system

Two layers, complementary.

### Deterministic rule engine

Fast, free, runs on every commit. Validates structural rules:
required substrings, forbidden phrases, hallucination detection,
abstention checks. Misses semantic equivalence — a model that
says \"role-based AI agents\" instead of \"role-based agents\" gets
zero credit. Useful as a fast safety gate, not as a quality
judge.

### Live LLM judge

`anthropic/claude-sonnet-4-6`, opt-in via `--judge`. Six quality
dimensions: evidence-awareness, specificity, safe-abstention,
operational-usefulness, role-fit, non-generic-GTM-reasoning. A
safety veto caps the score and forces verdict=fail when the
agent commits a contract violation (must-refuse, external
mutation, unjustified skill proposal).

### Calibration

The corpus has three variants per fixture:
`argo_aligned`, `polished_but_less_grounded`, `weak`. Hard CI
lock: the judge must rank `argo_aligned > weak` AND `polished >
weak`. Strict mode (opt-in via env): the full ordering
`argo_aligned > polished > weak`.

Why both layers: the deterministic engine catches obvious
failures cheaply; the LLM judge catches semantic ones the
substring engine can't see. Together they form a tight quality
gate that's both fast in CI and meaningful for the operator.

---

## 8. What makes this different

- **Role-based** — agents are hired into specific jobs, not given
  open-ended chat.
- **Learning loop** — every run can produce a candidate skill;
  the library compounds.
- **Reusable skills** — promoted skills persist across sessions,
  are vertical-namespaced, and are version-tracked.
- **Human approval** — no skill auto-activates. Operator review
  is mandatory.
- **Evaluation built-in** — every vertical ships fixtures and a
  judge. Quality is contracted, not assumed.
- **Safety-first execution** — manifest deny lists enforced at
  runtime. The GTM Engineer cannot send, write files, or update
  CRMs even if asked.

---

## 9. Current status

- **Pivoted role-agent V1 complete** (Codex)
- **GTM Engineer vertical** implemented end-to-end — eight
  skills, eleven memory schemas, ten evaluation fixtures
- **Live demo working** — deterministic mode in about two
  seconds, LLM-driven mode via the gateway
- **Learning loop proven** — role → work → pattern → skill →
  approval → active is locked by integration tests
- **Evaluation calibrated** — three-cohort corpus and judge
  ordering enforced in CI
- **Launch microsite** — single-page artifact for demo videos
  and shareable launches

---

## 10. What comes next

- **More roles** — Customer Support Triage, Account Research,
  Competitive Intelligence.
- **Improved evaluation** — broader fixture coverage; cross-
  vertical calibration corpus.
- **Production deployment** — managed gateway, multi-tenant
  workspaces, audit export.
- **Design-partner onboarding** — early-access cohort with
  operator-side review tooling.

---

*SKYCOT — AI employees that learn the job.*
