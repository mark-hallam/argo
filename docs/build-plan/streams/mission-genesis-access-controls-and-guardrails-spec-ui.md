# mission-genesis-access-controls-and-guardrails-spec-ui

> **STATUS 2026-06-10 — IMPLEMENTED (claude worker).** SPA "Set boundaries"
> panel + `argo-genesis::policy_synth` (spec → canonical `TruthBoundaryPolicy`,
> round-trip-enforced via `load_policy`/`evaluate_claim`, cross-tenant bound) +
> SSE `vertical_ready` boundaries echo. Two scope deviations carried per the
> brief's own split guidance: (1) the $-threshold leg does NOT touch
> `routes_approvals_threshold.rs` (that's N-of-M vote quorum, not a $-cap store;
> the real $-cap is `argo_identity::auto_approval::AutoApprovalPolicy` with no
> genesis write route today — split into a follow-up); (2) per-worker RUNTIME
> policy routing is deferred (runtime loading is process-global today). See PR.

**Owner**: claude.
**Authored against**: origin/main `7c81986c2` on `2026-06-10` (REVIVED — originally authored against `a85963b34` on 2026-05-25).
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-genesis-access-controls-and-guardrails-spec-ui`.
**Depends on**: SATISFIED — `mission-genesis-spa-page-describe-and-watch` (#2326) MERGED; `/genesis` page + `web/components/skycot/genesis/` exist on main.

🔴 **P1 MISSION CLAIM 3 — Phase 2 Onboarding Studio — visual access-control + guardrails specification.**

Per memory [[genesis-is-ai-teammate-onboarding-studio]], a real teammate gets access boundaries (what they can/can't claim or do) + escalation rules (when to ask humans) + thresholds (what needs sign-off). This brief makes that step explicit and visual in Genesis — toggles + sliders the operator reviews, instead of trusting Genesis to infer everything from prose.

The investor moment: *"I set the worker's boundaries — must escalate legal/contract questions, refunds over $500 need my approval — and Skycot enforces exactly those at runtime."*

## REVIVED 2026-06-10 — what changed since 2026-05-25 (verified read-only)

The genesis surface and the enforcement spine both shipped; the original may-touch pointed at files that have since been superseded or split. Verified on `7c81986c2`:

1. **Enforcement spine is LIVE (the premise this brief leans on holds).** `TruthBoundaryPolicy` + `evaluate_claim` exist in `crates/argo-capability/src/truth_boundary.rs` (`evaluate_claim` at ~line 414), and runtime enforcement at the agent-turn exit shipped in `crates/argo-agent-runner/src/turn_helpers_truth_boundary.rs` (evaluates final utterances, writes first-class audit events, creates approval requests on Escalate).
2. **HONESTY AMENDMENT — policy attachment is path/env-based today, not per-worker-manifest.** `turn_helpers_truth_boundary.rs` loads the policy via `load_policy` + the `ARGO_TRUTH_BOUNDARY_POLICY_PATH` env (with `ARGO_TRUTH_BOUNDARY_REFUSAL_TEMPLATES_DIR` / `ARGO_TRUTH_BOUNDARY_CHANNEL_KIND` companions). The original brief's "policy attached to the worker's manifest, enforced per worker" overstated current plumbing. Scope correction: `policy_synth` writes the synthesized policy to the tenant-scoped location `load_policy` reads (or minimally extends the existing loading seam) — it must NOT invent a parallel per-worker policy store in this brief. If per-worker policy routing turns out to require a new store surface, STOP and split that into its own brief.
3. **`vertical_ready` payload moved.** The original may-touch said extend `routes_genesis.rs`; the payload is now built in `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_payload.rs` (consumed by `routes_genesis_stream.rs`) — that is where the boundaries spec rides the SSE event.
4. **Approval thresholds have an existing gateway surface.** `crates/argo-cli/src/handlers/gateway/routes_approvals_threshold.rs` (+ tests) exists — the $-threshold leg WIRES INTO it rather than inventing a threshold store.
5. **The `/genesis` page grew a full intake flow** (GenesisModeToggle / ConversationIntake / SopUpload / DocumentUploader / QuizPanel / RoleContractReview / PreviewCard / ProgressStrip). The "Set boundaries" panel slots into `web/app/(app)/genesis/page.tsx` between role capture and the PreviewCard/hire step. Check `page.tsx` LOC headroom before adding — if it is near the 500-LOC budget, the panel wiring must stay a thin import + state hook.
6. **Still genuinely NEW (verified absent):** `BoundariesPanel` / `AccessScopeToggles` / `ApprovalThresholds` / `EscalationRoutes` components (zero hits in `web/`), and `crates/argo-genesis/src/policy_synth.rs` (absent). No parallel surface has shipped.

## Scope

1. **SPA "Set boundaries" panel** on `/genesis`, between role capture and preview/hire:
   - **Access scope**: read-only / read-write toggles per data category (customer PII / financial / contracts / source-code), mapping onto truth-boundary claim classes.
   - **Approval thresholds**: $-amount slider for actions needing human sign-off (refunds, sends) — persisted through the EXISTING `routes_approvals_threshold` surface.
   - **Escalation topics**: free-text "must escalate questions about X" entries (legal, compliance) → Escalate-class policy entries.
2. **Smart defaults**: pre-fill from the role description for at least 3 archetypes (Customer Success, Sales SDR, Marketplace Seller) — deterministic keyword mapping in `policy_synth` is sufficient; no LLM call required for defaults.
3. **Persistence**: `crates/argo-genesis/src/policy_synth.rs` (NEW module file; it declares no type named after, and instead CONSUMES, the existing canonical `argo_capability::TruthBoundaryPolicy`). It translates the UI spec into a policy value, written where `load_policy` reads it (tenant-scoped; see REVIVED note 2), and the spec rides the `vertical_ready` SSE payload (`routes_genesis_stream_payload.rs`) so the SPA can render what was actually persisted (no theater: render the server's echo, not the local form state).
4. **Tenant scoping**: the persisted policy location + the threshold write are tenant-bound; cross-tenant negative test included.

## Acceptance criteria

- [ ] "Set boundaries" panel renders on `/genesis` between role capture and the hire step; vitest component tests for the three sections (toggles/slider/topics) green.
- [ ] Smart-default pre-fill works for the 3 archetypes (unit test on the mapping in `policy_synth` — given "customer success" role text, financial defaults to read-only + refund-cap default present).
- [ ] Round-trip test (Rust): a synthesized policy from a spec with an escalation topic, loaded via the same `load_policy` path the runtime uses, makes `evaluate_claim` return an Escalate disposition for a claim on that topic — and an allowed-class claim still passes. (`cargo test -p argo-genesis policy_synth` + the capability round-trip test — paste tails.)
- [ ] Threshold leg: the chosen $-cap persists through the existing `routes_approvals_threshold` route (extend its tests with one write-then-read assertion from the genesis spec path).
- [ ] Cross-tenant: tenant A's synthesized policy is not readable/effective for tenant B (negative test).
- [ ] No-theater check: the panel state shown at the hire step is rendered from the `vertical_ready` payload echo, not from client-only form state (component test asserts the rendered values come from the SSE payload fixture).
- [ ] `cd web && npx tsc --noEmit` clean; all touched Rust files `cargo clippy` clean; every file < 500 LOC.

## May-touch (REVISED 2026-06-10)

- `web/components/skycot/genesis/BoundariesPanel.tsx` (NEW; may compose the three sections as internal components in ONE file if they stay small — prefer fewer files over the original 4-file split unless LOC forces it)
- `web/components/skycot/genesis/BoundariesPanel.test.tsx` (NEW)
- `web/app/(app)/genesis/page.tsx` (slot the panel + carry the spec into the generate/hire calls)
- `web/lib/api/genesis.ts` (extend the request/payload types with the boundaries spec)
- `crates/argo-genesis/src/policy_synth.rs` (NEW) + `policy_synth_tests.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_payload.rs` (carry the spec on `vertical_ready`)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream.rs` (thread the spec through; minimal diff)
- `crates/argo-cli/src/handlers/gateway/routes_approvals_threshold.rs` + its tests (one write-path extension)
- This brief file (status updates)

Explicitly NOT touched (changed from the 2026-05-25 version): `routes_genesis.rs` (superseded anchor — the payload lives in `routes_genesis_stream_payload.rs`); no new policy store / no per-worker manifest schema (see REVIVED note 2 — split if needed).

## Out of scope

- Policy-template marketplace; time-based access windows — v1.x.
- Per-worker (rather than per-tenant) policy routing if it needs new store surface — own brief.
- "Why was this blocked" explainability — existing audit chain covers it.

## Effort

~300-450 LOC (one SPA panel + synth module + round-trip tests + SSE payload threading). ~4-6 hours. Manager-worker allowed: no.
