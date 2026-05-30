# Genesis Done Right — production-quality custom AI teammates

**Date:** 2026-05-30
**Status:** Design / operator-alignment ONLY. No code in this pass.
**Author:** synthesis of three design lenses (QUALITY-FIRST, SAFETY-FIRST, CAPABILITY-FIRST) + an adversarial business-ruin critique, all verified against the live tree.

> Plain-language note (abbreviations used below, expanded once):
> **SSE** = Server-Sent Events, the one-way stream the `/genesis` page watches while a teammate is built.
> **MCP** = Model Context Protocol, the plug-in standard that lets a teammate actually touch Gmail / Stripe / Slack.
> **Capability fence** = a hard list of which tools/credentials a worker may use, checked at the instant it tries to act.
> **Prompt injection** = malicious instructions hidden inside a document or inbound message that the worker reads and obeys as if they came from its owner.
> **eval** = an automated test of the worker's behaviour. **Idempotency** = doing a thing twice has the same effect as doing it once (no double-charges). **RBAC** = Role-Based Access Control (who-can-do-what).

---

## Where we are today (verified against code, not narrative)

Genesis turns a plain-English role into a real, tenant-isolated, audited vertical template via `POST /api/v1/genesis` (SSE), consumed by the `/genesis` page into a 2-step hire that mints a real worker row. That much is real and already beats a prompt template. But against the bar of *"a teammate doing real, safe work in a paying customer's business,"* the live path is a **competent scaffold generator with one genuinely strong guardrail (approval-gating on sensitive verbs) and several written-but-unwired safety primitives.** Confirmed in the tree on 2026-05-30:

- The flagship SSE handler calls `plan_with` only — **never** `plan_or_broad`. So the from-scratch ("broad") path **and** the LLM semantic safety judge are dead on the product path (live in tests only). `routes_genesis_stream_llm.rs:119`.
- `judge_verdict:"pass"` is a **hardcoded string** in the SSE payload — the eval seeds are generated and *counted*, never *run*. `routes_genesis_stream.rs:389`.
- `quality::require_pass` (action≥3, eval≥10, memory≥1, prompt≥200 words) is wired into **tests only**; the pipeline computes the report and discards it. `quality.rs:129`.
- `dry_run`/`dry_run_until` is written to `worker.toml` and read by **no runtime dispatcher** (grep of `argo-agent-runner` / `argo-runtime` / `argo-orchestrator` = empty). The "first 24h are safe" promise is inert.
- Genesis emits **zero** `[tools]` / `[mcp_servers]` / `[escalation]` sections, binds **no** truth-boundary `policy_id`, and every generated action ships `executor = "human"`. So the teammate has an action *vocabulary* but no wired execution surface, no least-privilege fence, and no claim-gating — and it inherits the human principal's role capabilities (an owner principal = `ToolDispatch '*'`, `NetEgress '**'`, `SecretRead '*'`).
- The flagship SSE hire path is **strictly weaker** than the older `/onboarding/first-hire` path: it sets no `genesis_plan_id` and seeds no memory, so a teammate hired through the headline page has an empty Memory tab and no provenance chip.
- The `/genesis` hero copy already advertises document upload, wiki, Slack and voice intake that **do not exist** — a live "no-theater" violation. `web/.../genesis/page.tsx:268`.

---

## (1) WHAT "Genesis Done Right" IS

**The bar.** A teammate is "production-quality" when, *before it can touch a paying customer*, all nine hold:

1. **Role-synthesized (not bucket-templated) prompt** — scope, responsibilities, named systems, and limits derived from *this* role, not pasted into a 1-of-12 archetype template.
2. **The right skills**, vetted, bound at birth — not `skills:[]`.
3. **Real but tightly scoped execution surface** — actions map to actual fenced integrations (or stay human-run), never the owner's `*` keys.
4. **Correct memory + knowledge** — knows the business (ingested SOPs/policies), with a trust boundary on what it reads.
5. **Typed escalation routing** — "ask a human when X," enforced, not decorative prose.
6. **Conservative human-approval defaults** — irreversible/sensitive actions always gated; auto-approval off.
7. **Guardrails the operator can see and edit before hiring.**
8. **Evals it provably passes** — a real graded run gates hire; the verdict is honest and audited.
9. **Full observability** — hash-chained provenance of the synthesis and every gate verdict.

**True from-scratch (archetype-free) synthesis.** Today every output is keyed to 1 of 12 archetypes; a genuinely novel role (a niche compliance officer, a clinical-intake coordinator, a "chief vibes officer") is jammed into the nearest bucket *or* errors, and the keyword classifier dishonestly reports `confidence=1.0` on any match. "Done right" means a role is decomposed into a **typed capability spec** and compiled into one loadable teammate **whether or not it matches an archetype**. The 12 archetypes survive only as cheap *priors* that pre-fill the spec for common roles — they no longer cap the taxonomy or the artifact richness. **The single missing seam that makes this possible is a `BroadRoleDraft → loadable-vertical materializer`** (today `GenesisOutcome::Broad` is a terminal value that can be judged and slug-read but **never written** — there is no `write_broad_vertical` anywhere in the workspace).

**Honesty constraint:** the four "Onboarding Studio" inputs (document upload + ingest, Slack/Notion/Drive import, voice intake, comprehension quiz) are the real moat over "ChatGPT-with-prompts" — and all four are currently **BLOCKED** and **unbuilt**. "Done right" sequences them *after* the teammate is structurally safe, strips the theater copy *now*, and never claims an input that does not run.

---

## (2) ARCHITECTURE TO BUILD

The chosen spine is **SAFETY-FIRST (bounded-by-construction) + the QUALITY-FIRST graded hire-gate at a single fail-closed seam + CAPABILITY-FIRST's typed escalation**, with two safeguards the critique proved all three lenses missed (prompt-injection trust boundary; deterministic irreversibility gate) added as first-class. Everything reuses existing machinery; the genuinely new core piece is **one materializer + one typed spec.**

**Pipeline:** `role-description → CapabilitySpec (typed IR) → materializer → loadable vertical + bound least-privilege capability profile → graded-eval + comprehension gate → hire (fail-closed, single seam).`

### 2a. The CapabilitySpec IR (`crates/argo-genesis/src/spec.rs`)
A typed intermediate representation that **supersedes** the bare 6-field `BroadRoleDraft` (the very shape that can't be materialized today). Fields: `charter` (verbatim); `persona{name, description, onboarding_script}`; `responsibilities`; `capabilities: Vec<CapabilityDecl{action_name, intent, risk, reversible:bool, proposed_executor: ExecutorIntent{Human|Mcp{server,tool}|Builtin}, requires_approval}>`; `skills_wanted`; `knowledge_intents`; `memory_namespaces`; `tool_scope{allow,deny}`; `secret_scope`; `egress_scope`; `escalation{contact, escalate_after_failures, human_approval_required}`; `success_metrics`.

Two producers behind one `SpecSynthesizer` trait, both emitting the same IR so there is exactly **one** materializer:
- **ArchetypePrior** — when `match_template` / the confidence classifier lands a known archetype ≥0.6, pre-fill from the existing per-archetype seed vocabulary + memory schema (cheap, deterministic; the 12 archetypes are preserved as priors, regression-locked by the current pipeline tests).
- **LlmSpecSynthesizer** — one structured JSON call over the **same** transport the Pitch Agent + the shipped `GenesisLlmClassifier` already use (`argo_agent_runner::build_transport`; provider resolves `ARGO_GENESIS_LLM_PROVIDER → ARGO_PITCH_LLM_PROVIDER → claude-code-cli` per DEC-012). For archetype-prior roles it runs a cheap *enrich* pass (role-specific addendum + a few extra candidate actions); for off-taxonomy roles it synthesizes the whole spec.

### 2b. The materializer (`crates/argo-genesis/src/materialize.rs`) — the missing seam
`materialize_spec(&CapabilitySpec) -> Result<GenesisPlan>` + `write_capability_vertical(...)`, mirroring `write_vertical_with_overlap` (`pipeline.rs:218`) but keyed on the **spec**, not an `Archetype` enum:
1. Derive a validator-legal `vertical_id` (lowercase / digits / `-`; run through `validator::validate_id` **before** write — fail-closed rather than write an unloadable dir).
2. Wrap each `CapabilityDecl` into an `ActionDeclaration` carrying `risk` + `requires_approval` + a **real** `executor` from `ExecutorIntent` (closing the `executor="human"`-everywhere gap; `Mcp{server,tool}` becomes a real mcp-kind binding — but see safeguard #3 below: not auto-bound at birth).
3. Emit `[tools]` (allow/deny), `[mcp_servers]`, and `[escalation]` sections — all of which Genesis never writes today.
4. Emit typed `[[kpi]]` from `success_metrics` (falling back to the existing `build_success_metrics` SLA skeleton) so `role.kpis` is no longer empty.
5. Companion files via **new archetype-free variants**: `build_system_prompt_from_spec` / `build_persona_prompt_from_spec` (sourced from persona + responsibilities, replacing the `archetype.label()` template); `memory/schema.toml` from the spec's namespaces; `eval/seed.jsonl` + `eval-seed.toml` via the existing `eval_seed::generate` (already takes actions, not an Archetype); `worker.toml` via the existing `WorkerConfig::new_dry_run`.
6. **Run `safety_judge::auto_fix` + `judge`** over the wrapped actions — NON-NEGOTIABLE: the broad/spec path is the one place sensitive verbs arrive un-classified, so the approval invariant silently drops without this.
7. **Round-trip through `argo_vertical::load_dir` and assert zero `LoadError`s** — the identical assertion the archetype path's test already makes (`pipeline.rs:389`). A spec that can't produce a loadable vertical fails closed.

### 2c. Where it plugs in
- **Re-point `routes_genesis_stream_llm.rs::resolve_plan`** from `plan_with` → `plan_or_broad` (constructing the `BroadRoleClient` + `GenesisJudgeClient` over the existing transport). This single change turns ON the from-scratch path **and** the LLM semantic safety judge that fire only in tests today.
- **Slot the materializer into `broad_role_plan.rs::build_broad_outcome`** so `GenesisOutcome::Broad` carries a *written, loadable* vertical instead of being terminal.
- **Fence the hire-commit seam:** the composite gate (materialize ✓ ∧ quality ✓ ∧ evals ✓ ∧ comprehension ✓ ∧ safety ✓ ∧ irreversibility-classified ✓) runs in `routes_workers_hire.rs` immediately **before** `create_worker_from_draft`, so **all three hire paths** (flagship SSE, `/onboarding/first-hire`, CLI) are fenced by **one** gate — no per-surface divergence. Failing → `422` with `QualityReport::format_failures()` + a `GenesisGateFailed` audit row.
- **Reach parity:** carry `genesis_plan_id` into the SSE payload + flagship `HireDraft`, and reuse the existing `seed_worker_memory_from_plan` so the headline page stops being the weaker path (populated Memory tab + provenance chip).

---

## (3) QUALITY + SAFETY MECHANISMS — guarantee real work, cannot ruin a business

Every must-have from the adversarial critique is folded in. Items marked **[REUSE]** wire existing-but-dormant machinery; **[NEW]** is net-new.

**Quality (provably does real work):**
- **Graded eval gate at one fail-closed hire seam [NEW wiring of existing `run_eval`].** Run the 10 generated seeds through the generated worker (the Pitch Agent already wires `argo-eval` via `pitch_runner.rs`); replace the hardcoded `judge_verdict:"pass"` with the real pass@(score≥0.5). Hard-difficulty seeds = MUST_PASS. A worker that fails its own starter suite cannot enter normal operation. *Caveat to the operator: the seed metric is `SubstringContains{rubric_keywords}` — a floor against thin/empty workers, gameable by keyword-parroting; not a competence proof. Upgrade to LLM-graded rubric is a fast-follow.*
- **Quality hard-gate [REUSE `quality::require_pass`].** Flip it from test-only to pipeline-enforced (action≥3, eval≥10, memory≥1, prompt≥200 words), counting the on-disk `eval/seed.jsonl` (the real 10 seeds), with `--allow-low-quality` as the documented escape hatch.
- **Comprehension check [NEW].** ~5 closed-form questions derived from the teammate's *own role contract* (its scope, which actions need approval, what it escalates, what it must not claim), graded the same way. Verifies the teammate understood its *own scope/limits* before acting — gradeable **today**, without document ingestion. (The richer SOP-comprehension quiz waits on doc-upload; see open decisions.)
- **Spec-completeness gate [NEW].** Replaces the dishonest `confidence=1.0`; below threshold the spec is flagged *low-fit* and surfaced to the operator before hire.
- **Loadability + conflict gates [REUSE].** `load_dir` zero-errors round-trip; overlap refused at score ≥0.7.

**Safety (cannot ruin a business) — folding in all critique must-haves:**
1. **Deny-by-default capability + credential fence, bound to the WORKER not the principal [NEW derivation + REUSE enforcement].** Materializer writes `[tools]` with `allowed=minimal, denied=everything-else`; novel roles get `allowed=[]`. Enforced at the seam that **already** hard-blocks denied tools (`turn_vertical.rs:57`) — **zero new runtime code**. Fixes the "inherits owner's `*` keys" gap. *Lowest-risk because the fence is empty-by-default, immune to an LLM hallucinating a too-broad executor.*
2. **No live integration auto-bound at birth [NEW posture].** Generated actions are `human`-run (or curated-but-approval-gated). Genesis emits **no** live MCP / card-charging / computer-use binding. Real integration access is a **deliberate, audited, granular, post-hire operator grant** that itself flows through the approval ledger. A day-one teammate can draft and propose but **physically cannot transmit.**
3. **Deterministic irreversibility gate [NEW] — the critique's #4 must-have.** A static rule (not the 7-word lexical denylist, not the non-deterministic LLM judge alone) forces `wire_funds | issue_refund | drop_table | cancel_subscription | terminate_employee | refund | chargeback | wipe | revoke`-class actions to **always-human**, keyed off the spec's `reversible:bool` AND a curated irreversible-intent list. This is the deterministic floor under the LLM semantic judge.
4. **Two safety judges, layered [REUSE both].** Deterministic structural judge (`send|post|pay|publish|delete|share|message`, offline, un-bypassable) stays the floor; the **LLM 3-question semantic judge** (illegal/harmful, restricted financial/medical/legal surface, real-person impersonation; fail-closed to Block) goes **live** automatically once `resolve_plan` routes through `plan_or_broad`. Catches semantic risk the lexical list misses.
5. **Enforced probation / dry-run at EVERY dispatch seam [NEW enforcement of existing flag] — the critique's #1 + #6 must-have.** Read `dry_run_until` (or a `probation_until`) at runtime so every emitted action lands *pending* for the window. **Critical:** the check must live at **one chokepoint or be replicated in every path** (`argo-agent-runner/dispatch.rs`, `argo-tool/registry.rs`, **and** the workflow executor / orchestrator) — a chat-driven teammate and a workflow-driven teammate take different paths; missing one is a bypass. Load `dry_run_until` once at session boot (not per-action I/O on the hot loop).
6. **Truth-boundary policy bound at birth, on-by-default for generated workers [NEW binding + REUSE evaluator].** Assign a conservative default `policy_id` (Escalate on unverified factual/financial/legal/medical claims) to every generated worker; the already-wired runner enforces it when a `policy_id` is present (replacing the env-var-only activation). *Ceiling the operator must know: the evaluator matches only producer-declared claim classes — the promised P-02 semantic classifier was never built — so an undeclared over-claim is Allow-by-default. This is a real improvement over today's policy-less worker, not a complete fabrication guarantee.*
7. **Prompt-injection / poisoned-content trust boundary [NEW] — the single biggest gap all three lenses missed.** Before ANY knowledge ingestion ships: provenance-tag every retrieved chunk as *untrusted data*; enforce an instruction/data separation boundary in prompt assembly; an egress/recipient allowlist that an injected instruction **cannot widen**; and **human approval on any action whose target (recipient, destination, amount) was derived from ingested/inbound content rather than from the operator's original charter.** Rationale: the capability fence bounds *which* tools fire, never *why* — a poisoned ticket that tells a support worker (legitimately granted `send_email`) to email the database dump sails through every cap check. This must land **before** the moat features, not after.
8. **Audited *degraded* mode, not binary fail-closed [NEW] — the critique's #6 must-have.** Distinguish "verification said NO" (hard fail, surface the reason) from "verification couldn't run" (judge/eval subprocess errored or quota-out → degrade to structural-judge-only + `needs_review` + **audit the degradation**, hold in dry-run). Pure fail-closed bricks all hiring under the exact memory/quota pressure this repo documents; this avoids a self-inflicted denial-of-service while never silently passing.
9. **Operator-editable "what can this teammate do" review screen BEFORE Hire [NEW] — the critique's #7 must-have.** Human-readable summary of the fence, escalation rules, approval-gated actions, bound skills, and (later) ingested knowledge — reviewable and editable before Hire fires. Genesis must not Hire-on-`vertical_ready` into a black box. Cheapest, highest-trust safeguard available.
10. **No-theater payload contract as a hard CI gate [NEW] — the critique's #10 must-have.** The SSE payload and `/genesis` hero may advertise only inputs that actually run; a CI assertion fails the build on mismatch. Strip the upload/wiki/Slack/voice copy now.
11. **Tenant-match assertion at hire, fail-closed [NEW] — the critique's #9 must-have.** Assert the worker's `tenant_id` matches the template's tenant segment; fail-closed on mismatch (latent today under single-tenant-per-cwd, live the moment the gateway goes multi-tenant).
12. **Hash-chained audit of the full synthesis + every gate verdict [REUSE].** One row per stage (spec synthesized with honest `llm|keyword` flag, each safety verdict, the real eval pass-rate, the bound capability set, probation window, hire). Per-tenant, tamper-evident. *Known limit: tamper-evident, not tamper-proof — no external anchoring; a separate enterprise track.*

**Two residual holes the operator should know are NOT fully closed by v1** (named, not papered over):
- **"Escalate when uncertain" needs a confidence/abstention signal in the loop** — none of the three lenses wired one, so a *confidently-wrong-on-a-novel-non-sensitive-task* worker still trips no gate. Mitigated by the comprehension gate + the eval floor, not eliminated.
- **Action-level idempotency / replay** (double-charge, duplicate dunning) is unaddressed by all three lenses and by the current runtime. Probation delays the first action but does nothing about a retry after the window. Flagged for a dedicated runtime brief.

---

## (4) BUILD PHASES (in sequence, each gated on the prior)

**Phase 0 — Wire the gates that already exist (highest ROI, lowest risk, no new types).**
(a) `quality::require_pass` enforced in the write path + fix `check_plan` to count the on-disk `eval/seed.jsonl`. (b) Re-point `resolve_plan` → `plan_or_broad` (turns on the from-scratch path + LLM semantic judge + disclaimer flow). (c) Strip the theater hero copy + add the no-theater CI assertion.
*Gate:* existing pipeline + safety_judge suites stay green; add an assert that `resolve_plan` returns a `Broad` outcome for a deliberately off-taxonomy charter.

**Phase 1 — The materializer (the single missing piece).**
`materialize_spec` / `write_capability_vertical` + archetype-free prompt/memory variants; run safety_judge; round-trip `load_dir`. Slot into `build_broad_outcome`.
*Gate:* an off-taxonomy charter ("chief vibes officer") produces a vertical `load_dir` accepts and `vertical_exists()` finds — the archetype path's round-trip test, replicated for broad.

**Phase 2 — Deny-by-default capability fence + irreversibility gate (the safety core).**
Materializer emits `[tools]`/`[mcp_servers]`/secret/egress; hire derives a default-deny per-worker `CapabilityProfile` intersected with the principal. Add the deterministic irreversibility classifier.
*Gate:* a generated worker is provably blocked from a denied tool at the dispatch seam; an irreversible action is forced always-human; a broad worker has `allowed=[]`.

**Phase 3 — Enforced probation at EVERY dispatch seam.**
Read `dry_run_until`/`probation_until` at runtime; enumerate and cover agent-runner, tool-registry, **and** workflow/orchestrator paths.
*Gate:* an action emitted at +23h lands pending; at +25h dispatches — proven on *both* the chat-driven and workflow-driven paths.

**Phase 4 — Real eval grading + comprehension gate at the single hire-commit seam.**
Replace the cosmetic pass with `run_eval`; add the role-contract comprehension check; fence `routes_workers_hire.rs` before `create_worker_from_draft`; add the audited degraded mode.
*Gate:* a deliberately-broken draft POSTed to `/api/v1/workers` is refused `422` with per-check reasons; the reference worker passes; a quota-out path degrades to `needs_review`, not a brick and not a silent pass.

**Phase 5 — Parity + truth-boundary binding + operator review screen.**
Carry `genesis_plan_id` + seed memory on the flagship path; bind the conservative default truth-boundary `policy_id`; ship the human-readable "what can this teammate do" review-before-Hire screen; tenant-match assertion.
*Gate:* flagship-hired worker has a populated Memory tab + provenance chip; an unverified factual claim to a customer enqueues an approval; the operator sees and can edit the fence before Hire.

**Phase 6 — The moat, in dependency order, with the injection boundary FIRST.**
(a) **Prompt-injection trust boundary** (provenance tags + instruction/data separation + egress allowlist + content-derived-target approval) — *must precede ingestion.* (b) Document upload + ingest into worker memory (reuse `seed_worker_memory_from_plan`). (c) Slack/Notion/Drive import; access-controls UI; voice intake. (d) **SOP-comprehension quiz LAST** — gates Hire on a passed quiz against *ingested* knowledge; the investor-defining differentiator that cannot be faked without real ingestion.
*Gate:* a teammate hired from a role + an uploaded SOP starts with vetted skills AND business-specific knowledge, and a poisoned document cannot make it act outside the charter-derived egress allowlist.

**Phase 7 — Named-but-deferred runtime hardening:** action-level idempotency/replay safety; a confidence/abstention signal feeding escalation; LLM-graded eval rubric (replacing substring metric). Separate briefs; not v1-safety-core.

---

## (5) OPEN DECISIONS — operator must align BEFORE building

1. **Human-review-before-activation: default ON?** Recommend **yes** — every generated teammate lands in an explicit operator review-and-edit screen before Hire, and stays in enforced probation (no live executor) until the operator activates it. This is the cheapest trust anchor and the demo's "you stay in control" beat. *Decision: accept review-before-activation as the default, or make it opt-in for speed?*
2. **Probation window length and behaviour.** 24h (today's written value) vs 72h (SAFETY-FIRST proposal); and does probation force *approval-on-every-action* or *no-executor-at-all*? Recommend **72h, no-live-executor**, operator can shorten per-worker. *Decision: window + strictness.*
3. **Archetype-free vs archetype-seeded.** Recommend the **hybrid**: archetypes survive as cheap deterministic *priors* for common roles (fast, no LLM spend), full LLM spec-synthesis only for off-taxonomy roles + a cheap enrich pass on archetype roles. *Decision: accept the hybrid, or insist on pure archetype-free (higher cost/latency, more non-determinism) or pure archetype (cheap, but the "novel role" demo fails)?*
4. **How far to take comprehension verification for v1.** Two tiers: **(A) role-contract comprehension** (scope/limits/approval rules) — gradeable **today**, no ingestion; **(B) SOP comprehension** — needs doc-upload, is the investor-defining moat, ships in Phase 6. Recommend ship **A now**, **B in Phase 6**. *Decision: is role-contract comprehension enough to gate Hire in v1, or is the moat-defining SOP quiz required before any demo?*
5. **Multi-modal onboarding scope for the demo.** The four moat inputs (doc-upload, Slack/Notion/Drive import, voice intake, comprehension quiz) are all currently BLOCKED. Recommend: **doc-upload + role-contract comprehension** for the first investor demo (smallest real moat), defer voice + connector-import. *Decision: which moat inputs are in-scope for the v1.0 demo MP4 vs deferred — and is voice intake a demo blocker or a fast-follow?*
6. **Live integration binding policy.** Recommend integrations are **never** auto-bound by Genesis; access is a deliberate post-hire operator grant through the approval ledger. *Decision: accept "draft-only until the operator grants integrations," or allow Genesis to auto-bound low-risk read-only connectors at birth for demo smoothness?*
7. **Eval grading rigour for v1.** Substring-keyword grading is gameable. Recommend ship the substring floor now (gates thin/empty workers), upgrade to an LLM-graded rubric as a fast-follow. *Decision: is the substring floor acceptable to gate Hire in v1, or must LLM-graded rubric land first?*
8. **Cost/latency posture on the hire path.** Real eval + comprehension grading adds ~15 model calls (claude-code-cli subprocess, on the subscription). Recommend **async grading during the SSE stream** (worker starts in dry-run until the verdict lands, which it must anyway). *Decision: accept async-grade-into-dry-run, or require synchronous pre-Hire grading (slower stream, but no "graded after the fact")?*
9. **Multi-tenant timing.** The SSE-template-tenant vs hire-worker-tenant mismatch is latent under single-tenant-per-cwd. Recommend the fail-closed tenant-match assertion now (cheap), full multi-tenant Genesis as a separate enterprise track. *Decision: hold single-tenant-per-cwd for v1, or pull multi-tenant Genesis forward?*
