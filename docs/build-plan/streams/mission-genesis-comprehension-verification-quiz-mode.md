# mission-genesis-comprehension-verification-quiz-mode

**Owner**: claude.
**Authored against**: origin/main `a85963b34` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-comprehension-verification-quiz-mode`.
**Depends on**: `mission-genesis-document-upload-and-memory-ingest` + `mission-genesis-integration-import-slack-notion-confluence` + `mission-genesis-access-controls-and-guardrails-spec-ui` MERGED.

🔴 **P0 MISSION CLAIM 3 — Phase 2 Onboarding Studio — the "show me what you learned" gate before Hire.**

Per memory [[genesis-is-ai-teammate-onboarding-studio]]: a real manager doesn't just hand a new hire a stack of SOPs and let them loose on customer tickets. They run a comprehension check first — *"based on what you read, how would you handle X?"* — and refine the worker's understanding before assigning real work. This brief adds that step.

The investor moment: *"Before I hired the worker, I asked them three questions about our refund policy and our escalation matrix. They answered correctly using the SOPs I uploaded. I tweaked one wrong assumption in their memory and re-tested. THEN I hired them. That's the difference between renting an LLM and onboarding a teammate."*

## Scope

1. **SPA surface**: after the Genesis preview card renders (per `mission-genesis-spa-page-describe-and-watch`) but BEFORE the Hire button activates, add a "Quiz me" panel. Operator types a question; the worker (running in a sandbox session) answers with retrieval-augmented response citing uploaded docs + integration imports.
2. **Sandbox session**: the to-be-hired worker is instantiated in a non-production tenant slot ("sandbox") so the operator can interact without committing. The worker has full memory access (uploaded docs + integration imports + the synthesised policy) but its responses don't persist beyond the Genesis session.
3. **Refinement loop**: if a response is wrong, operator can: (a) edit a memory entry directly ("Actually, refunds over $500 require legal sign-off, not just manager approval"), (b) re-upload a corrected doc, (c) tweak the boundary spec. Re-quiz.
4. **Suggested questions**: Genesis pre-populates 3-5 suggested questions based on the uploaded content + role description ("How would you handle a refund request over $500?"). Operator can use as-is or write their own.
5. **Hire gate**: Hire button is enabled only after the operator clicks "I'm satisfied with their answers" (explicit operator confirmation). The button text reflects the journey: *"Hire <name> — they're ready."*
6. **Audit trail**: every quiz exchange is logged as an `audit_record` (kind `worker.onboarding_quiz_exchange`). Tenant-scoped. Provides forensic record for "what was the worker tested on before we deployed them?".

## Acceptance criteria

- [ ] After the preview card renders, "Quiz me" panel is visible + functional.
- [ ] Quiz responses use REAL retrieval over the uploaded documents + integration imports. Function-side check: ask a question whose answer is in an uploaded SOP — the response cites that SOP verbatim or paraphrased.
- [ ] Memory edit + re-quiz round-trips: edit a memory entry → next quiz response reflects the change.
- [ ] Hire button is disabled until the explicit "I'm satisfied" confirmation.
- [ ] Audit trail: 1 quiz session produces N `worker.onboarding_quiz_exchange` rows, tenant-scoped.
- [ ] Cross-tenant: sandbox worker is isolated; tenant A's quiz responses don't leak to tenant B.
- [ ] Function-side: no theatre — if memory is empty, the worker says "I don't have a SOP that covers this" instead of fabricating an answer.

## May-touch

- `web/components/skycot/genesis/QuizPanel.tsx` (NEW)
- `web/components/skycot/genesis/MemoryEditModal.tsx` (NEW — inline memory editing)
- `web/lib/api/genesis-quiz.ts` (NEW — sandbox-session client)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_quiz.rs` (NEW — sandbox-session lifecycle)
- `crates/argo-genesis/src/sandbox_session.rs` (NEW — non-persistent worker instance for quiz mode)
- `crates/argo-audit/src/event_kind.rs` (add `WorkerOnboardingQuizExchange` variant)
- Tests for retrieval-honesty + cross-tenant + memory-edit-roundtrip

## Out of scope

- Adversarial red-team quizzing (mature feature; v1.x)
- Multi-operator quiz collaboration (single operator for v1.0)
- Auto-generated quiz questions from compliance frameworks (separate brief)

## Effort

~400-700 LOC (UI quiz panel + memory edit modal + sandbox-session lifecycle + audit kind + cross-tenant test + retrieval-honesty test). ~5-8 hours.
