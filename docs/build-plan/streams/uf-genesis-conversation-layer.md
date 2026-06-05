# uf-genesis-conversation-layer

**Owner**: claude.
**Status**: ENQUEUED 2026-06-06 (conductor) — rowed as TODO in BUILD_STATUS.md. All depends-on verified MERGED at enqueue.
**Depends on**: A1 MERGED (`uf-genesis-capture-cadence-exceptions-success` + `uf-genesis-a1-wire-capture-end-to-end`) — provides the `CapabilitySpec` slots + the intake route + capture component this EXTENDS; `/genesis` SPA (#2326) MERGED; tenant keystone (sprint-ws1) MERGED. Governed by **[[DEC-039]]** (additive "Talk it through" mode, NOT a replacement of the one-shot flow).
**Authored against**: origin/main `dc982d9c3` on `2026-06-05`. (Re-stamped at enqueue 2026-06-06 AEST; depends-on verified MERGED: #2711 + #2713 (A1) · #2326 (/genesis SPA) · #2676 (tenant keystone).)
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-genesis-conversation-layer`.

## Why

The operator's #1 ask for Genesis was "interview me." Today `/genesis` is a one-shot
"describe + capture panels" flow — it never asks a follow-up. DEC-039 locked an
**additive** conversational mode that elicits the same requirements by dialogue, for
users who can't describe a role cold. This is the conversation layer; it does NOT
replace the describe-and-watch flow (DEC-039), and it reuses the slots A1 already
ships — it must not fork them.

## What

Add a **multi-turn conversational intake** ("Talk it through") that:
- proposes the next best question and extracts the user's answer into the **existing**
  `CapabilitySpec` slots — `responsibilities`, `cadence` (`spec.rs:205`), `exceptions`
  (`spec.rs:210`), `success_criteria: Vec<SuccessCriterion>` (`spec.rs:215`) — plus
  charter/persona;
- converges on the **same** `CapabilitySpec → materialize → review → hire` path as the
  one-shot flow (no parallel materializer);
- drives the LLM via the shared transport (`complete_once` → `argo_provider::Transport`,
  the same path the classifier / author / rubric use, provider from
  `ARGO_GENESIS_LLM_PROVIDER`);
- **degrades fail-safe**: when the provider is unavailable, a deterministic wizard asks
  the next fixed question for the next unfilled slot, so a no-LLM demo still completes.

Surfaced as a **"Talk it through"** entry beside **"Describe it"** on `/genesis`; both
land on the identical review → hire. Additive only — DO NOT remove or restructure the
one-shot flow, and DO NOT fork the materializer or the slot types.

## May-touch

- **Backend (new, each <500 LOC):**
  - `crates/argo-genesis/src/intake_machine.rs` — pure, testable slot state machine:
    given filled slots + transcript → `NextStep::{Ask(question) | Confirm(spec) | Done(spec)}`.
    Deterministic required-slot ordering; the wizard fallback uses this.
  - `crates/argo-genesis/src/intake_llm.rs` — two bounded calls per turn (propose next
    question; extract latest answer → slot delta JSON), token-capped like the existing
    classifier; any transport/parse error → `None` → caller falls back to the wizard.
  - Multi-turn turn endpoint: extend `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_intake.rs`
    or a sibling `routes_genesis_intake_conversation.rs` (SSE), registered in the
    dispatch chain. Persist transcript + partial slots in a **tenant-scoped** table
    (reuse any A1 intake-persistence table if present — grep first; else an additive
    migration matching the slot-file system).
- **Reuse (do NOT fork):** `CapabilitySpec` slots (`crates/argo-genesis/src/spec.rs:175`),
  `SuccessCriterion`, the `materialize` path, `role_contract.rs`.
- **Frontend:** `web/components/skycot/genesis/` — a "Talk it through" toggle + a chat
  surface + a live "what we've captured" panel reusing `CadenceExceptionsCapture.tsx`'s
  slot rendering; `web/lib/api/genesis.ts` (add the conversation-turn calls beside the
  existing intake calls).
- Invariants: no `unwrap`/`expect` outside `#[cfg(test)]`; `tenant_id` on the table +
  route; files <500 LOC (split machine / llm / route).

## Acceptance criteria
- [ ] A back-and-forth fills responsibilities + cadence + exceptions + success_criteria +
      charter, ending in a valid `CapabilitySpec` that materializes **identically** to the
      one-shot path (same review → hire). Test pins the round-trip.
- [ ] The one-shot "Describe it" flow still works unchanged (additive per DEC-039) — regression test.
- [ ] Provider failure mid-conversation degrades to the deterministic wizard (next fixed
      question), never crashes / never a blank screen — test forces a transport error.
- [ ] Conversation persists per tenant; tenant A cannot read tenant B's intake — isolation test.
- [ ] No parallel slot/spec types introduced (reuses `CapabilitySpec`). `cargo` green;
      `pnpm vitest run` green on touched specs (report counts); all touched files <500 LOC.

## Out of scope
- Voice intake → `mission-genesis-voice-input-via-argo-voice-io` (separate brief).
- Guardrails / systems / SOP-doc capture panels → the `mission-genesis-*` Phase-2 tree.
  This conversation captures the core slots; the richer panels stay their own briefs.
- Replacing or removing the one-shot describe-and-watch flow (DEC-039 forbids it).
- **SIZING:** if the backend engine + the SPA chat together exceed one safe work-unit,
  split at claim time into `uf-genesis-conversation-layer-backend` (machine + llm + route +
  table + tests) and `uf-genesis-conversation-layer-spa` (chat UI + toggle + captured-panel +
  tests); the SPA depends on the backend.

## Parallel-surface check
- `rg 'pub struct CapabilitySpec|cadence|exceptions|success_criteria' crates/argo-genesis/src/spec.rs` — EXTEND these slots, never fork.
- `rg -l 'intake' crates/argo-cli/src/handlers/gateway/ web/components/skycot/genesis/` — build on the A1 intake route + `CadenceExceptionsCapture`, no parallel capture family.
- `rg 'intake|genesis_intake' crates/argo-store/src/migrations/` — reuse any A1 intake table before adding one.

## Source
[[DEC-039]] (additive conversation mode) · `docs/skycot/plans/USEFULNESS-FLOW-RECONCILED-BRIEFS-2026-06-05.md` (brief A3) · [[DEC-025]] (Genesis core build).
