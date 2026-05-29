# Design plan — Self-improvement: learn from real usage

**Status:** ready-for-go implementation plan. Authored 2026-05-29 (overnight).
**Closes:** competitive gap #3 (self-improvement / feedback loop).
**Effort:** training-data export = MEDIUM (maps onto data we already have) ·
trace-clustering = HIGHER (depends on telemetry).

> **Honesty note from the gap analysis:** this gap is **thinner than it looks**.
> Relevance AI's "self-improvement" claim *failed* verification (0–3). The
> orchestration leaders do **not** uniformly ship a real "gets better over time"
> loop — it's over-marketed across the board. So this is worth doing, but it's a
> *parity-plus* play, not a frantic catch-up. Don't over-invest.

---

## Plain-English summary (for the operator)

We tell buyers our teammates "learn." Today that means: they remember facts, and
you can install/promote skills. What we *don't* have is a closed loop where a
teammate **measurably gets better from real feedback** — the thing CrewAI markets
as `crewai train`.

We already sit on the raw material for it: every action is in the
**tamper-evident audit log**, every **approval decision** (you approved / you
rejected) is recorded, and the **eval harness** scores behaviour. We just don't
*package* that into something a teammate (or a human, or a fine-tune) can learn
from.

Two steps, smallest-first:

- **Step 1 — Export real feedback as training data (medium, buildable now).**
  One command turns "every approve/reject decision + the audit context + eval
  pass/fail" into a clean, downloadable dataset. After this: a customer can see
  *exactly* what their teammates did and how it was judged — and can use it to
  fine-tune or analyse. This is real, honest, and uses 100% real data.
- **Step 2 — Auto-find failure patterns (bigger, later).** Cluster the failures
  ("she keeps over-discounting") and surface them as suggested retrains/policies.
  LangSmith's "Insights Agent" class of feature. Depends on richer telemetry;
  defer.

**Recommendation:** build **Step 1** (it's bounded, uses real data, needs no
external dependency, and gives a concrete "your teammates' track record, on
tap" artifact). It also pairs beautifully with our **audit + truth-boundary**
moat — "not only is every action provable, you can learn from the record."
Hold **Step 2** until we have a customer asking for it.

---

## What exists today (verified against the repo)

- `argo-eval` is **case-based only**: name / input / expected / metric, loaded
  from jsonl. No production-trace ingestion, no feedback-mining, no export.
- The **audit log** records every action (hash-chained, tamper-evident).
- The **approval queue** records every human approve/reject decision
  (`argo-store::approvals`), with the draft content + decision + actor.
- "Skills shared + learned" today = install/promote (Claim 4) — real, but it's
  *distribution*, not a *gets-better-over-time* loop.

## The data we already have (the export's raw material)

| Source | Signal it carries |
|---|---|
| `audit_record` | what the teammate did, with full context + chain integrity |
| `approval_request` + decision | human judgement: this action was good / bad |
| `argo-eval` results | scored behaviour against expected outcomes |
| retrain commits (`worker.retrain.committed`) | explicit human corrections |

That last row is notable: the retrain log we just built (Q1) is *already* a
human-feedback signal — every committed retrain is a labelled "do it this way
instead." The export should include it.

## Decisions the operator should weigh (export format / purpose)

Only one real choice — **what is the export FOR?** It changes the format:

| Purpose | Format | Notes |
|---|---|---|
| **Fine-tuning / SFT** (recommended default) | JSONL of `{messages, label, judged_outcome}` (OpenAI/Anthropic SFT shape) | Industry-standard; drops straight into a fine-tune pipeline |
| **Analysis / BI** | CSV/Parquet of decisions + outcomes | For a data team, not a model |
| **Eval-case generation** | New `argo-eval` jsonl cases derived from real rejected actions | Closes the loop *inside* our own harness — turn real failures into regression cases |

**Recommendation:** ship **JSONL (SFT shape)** first — it's the CrewAI-parity
artifact buyers expect. The **eval-case-generation** variant is a strong
follow-up because it improves *our own* eval coverage from real usage (a genuine
loop), with no external pipeline needed.

## Architecture — read-only export over existing stores

```
argo-eval (extend) — no new persistence, pure read + transform
├── struct TrainingExample { worker_id, context, action, judged_outcome, source }
├── async fn export_training_data(tenant, filters) -> Vec<TrainingExample>
│     • join audit actions ↔ their approval decisions ↔ eval results ↔ retrains
│     • redact PII at the boundary (reuse pii_detect, as the audit sink does)
└── serializers: jsonl_sft(), eval_cases_jsonl()
CLI:  argo eval export-training-data --tenant <t> --format sft|eval-cases --out file.jsonl
(optional) GET /api/v1/eval/training-data/preview  → counts by outcome (no PII)
```

Invariant-safe: **read-only** (no migration, no new table), tenant-scoped,
PII-redacted at the edge, additive CLI + optional preview route. It "wires" to a
real consumer: the operator/customer downloads a real artifact (a terminal
deliverable, like a report — not scaffold).

## Implementation phases

### Step 1 — training-data export (MEDIUM, buildable now, no external deps)
- `export_training_data` join + the two serializers + the CLI command.
- PII redaction on the way out (consistent with `argo-audit`'s `pii_detect`).
- **Verify:** seed a tenant (we have the demo seed), export, assert the JSONL is
  well-formed, includes approve+reject+retrain rows, is tenant-scoped (a
  different tenant exports nothing), and carries no raw PII when autoredact is on.

### Step 1b — eval-case generation variant (strong follow-up)
- Turn rejected actions / committed retrains into `argo-eval` cases so real
  failures become regression tests. This is the honest "closed loop" — the
  system gets measurably better at *not repeating* judged-bad behaviour.
- **Verify:** generated cases load + run in the existing eval harness.

### Step 2 — auto trace-clustering (HIGHER, defer)
- Cluster audit/eval failures into named failure-modes + suggest retrains.
  Depends on richer telemetry/embeddings. LangSmith-Insights class. Defer until
  a customer asks.

## Risks / honesty notes
- **Don't oversell it.** Per the gap analysis, rivals' self-improvement is mostly
  marketing — so frame ours precisely: "export your teammates' real, judged track
  record for fine-tuning or to harden their evals," not "self-learning AI."
- PII is the live hazard in any export — redact at the boundary, never put data
  in URLs, and the export is a *download the operator triggers*, not an
  auto-send (privacy rule).
- This is the most "just build it" of the three plans — least operator-decision
  surface (one format choice) — so it's a good candidate for the next build
  block if the operator wants a shipped feature rather than a plan.

## Brief decomposition (ready to author)
`eval-export-training-data-join-and-jsonl` →
`eval-export-cli-command` → `eval-export-eval-case-generation-variant` →
(later) `eval-trace-clustering-insights`. Each carries the freshness header +
names its consumer per DRIFT_PREVENTION.md.
