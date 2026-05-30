---
id: DEC-025
date: 2026-05-30
status: active
supersedes: none
---

# DEC-025 — Genesis done right: full from-scratch teammate synthesis is the committed core build, to a production-quality bar

## Decision

Genesis is **the core differentiator** — the platform's central promise is that it *births custom AI teammates*. The operator locked (2026-05-30): build Genesis **right**, not merely demo-able.

- **Full from-scratch, archetype-free synthesis.** A teammate is generated to fit ANY role description — not selected from the 12 fixed archetypes. The LLM-archetype-*selection* shipped this session (commit `3f8a6f3`, flag `ARGO_GENESIS_LLM_CLASSIFIER`) is a **stepping stone, not the destination**.
- **Production-quality bar.** Generated teammates must do **real work** and must **not ruin a customer's business**. Quality + safety are existential here, because these teammates act inside paying customers' operations.
- **Align before building.** This is a difficult task; rushing ruins it. Design-first → operator alignment → then build.
- **Ultracode effort** (Opus 4.8): workflow orchestration, exhaustive, correctness over token cost.

## Why

Without Genesis producing genuinely capable + safe bespoke teammates, the platform does not deliver the thing it tells customers it provides. "Describe a role → get a real, trustworthy colleague" is the reason to choose Skycot over wiring up ChatGPT-with-prompts yourself. A generated teammate that takes a wrong, irreversible action would ruin a customer relationship — so the safety/quality gates are *part of the feature*, not an add-on.

## How to apply

1. **Design-first.** The `genesis-done-right-design` workflow produces the alignment proposal at `docs/skycot/design/2026-05-30-genesis-done-right.md`. **No Genesis build code ships until the operator aligns on that proposal.**
2. **The build must include the quality + safety mechanisms** that guarantee teammates do real work and cannot ruin a business: eval/comprehension gates (never hire a teammate that fails its own evals), the safety judge, approval-first defaults, capability scoping, truth boundary, escalation policy. The adversarial "how could a generated teammate ruin a business" critique in the design workflow enumerates the must-haves.
3. The `BroadRoleDraft → loadable-teammate` materializer (the currently-missing piece in `argo-genesis`) is the spine of from-scratch synthesis.

Related: [[DEC-010]] (Genesis is MVP-must-have), [[DEC-022]] / [[DEC-023]] (program scope + priority — this elevates Genesis to the active core build), [[DEC-007]] (no theater — the teammate must be genuinely capable, not a convincing shell).
