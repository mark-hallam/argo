# genesis-interview-quality-bar-and-access-escalation-slots

**Owner**: claude.
**Depends on**: none (extends the merged A3 conversation layer #2724/#2725; coordinate with `mission-genesis-access-controls-and-guardrails-spec-ui` — CLAIMED — which owns boundaries/policy UI).
**Authored against**: origin/main `aac402865` on `2026-06-10`.
**Validate before claim**: bash scripts/check-brief-freshness.sh genesis-interview-quality-bar-and-access-escalation-slots

## Goal ([DEC-048] rule 4 — quality bar, adaptively)

Deepen the "Talk it through" interview from "accept the first non-empty
answer" to "probe vagueness once, and ask about access needs + escalation
thresholds" — without ever blocking intake on an LLM failure, and without
fixed-length questioning (rich answers must keep collapsing multiple slots
per turn, which the machine already does).

## Why now (live e2e 2026-06-10)

The interview accepted "Weekly" as a complete cadence (no day/time), never
asked what "sensitive clients" means, and never asked about access (the job
described lived in Xero + email) or escalation preferences — then went
straight to "Ready to build". `slot_filled`
(`crates/argo-genesis/src/intake_machine.rs:252-261`) is a binary non-empty
check; there is no follow-up mechanism and no Access/Escalation slot.
Operator-locked: depth over speed; useless teammates are the failure mode.

## Fix

**Part 1 — two new slots (asked, skippable).**
1. Add `IntakeSlot::Access` + `IntakeSlot::Escalation` variants
   (`intake_machine.rs:45-60`), slug arms (:74-84), wizard questions
   (:89-108) — e.g. Access: "What systems or accounts will this teammate
   need to look at or use — email, a CRM, a finance tool? (They start
   draft-only; nothing is connected without you.)"; Escalation: "When should
   they stop and bring something to you — after how many failed tries, or
   above what dollar amount?"
2. New `IntakeState` fields (`:176-202`) + `SlotDelta` fields (`:135-154`),
   all `#[serde(default)]` (backward-compatible with in-flight
   `.genesis-intake/*/*.json` state — add a deserialization test on an
   old-shaped JSON).
3. `slot_filled` arms; **completeness semantics**: REQUIRED slots gate
   `is_complete()` as today; Access/Escalation are ASKED but an explicit
   skip ("skip" / "not sure") marks them answered — they must not wedge the
   confirm gate (`routes_genesis_intake_conversation.rs:142`). Track
   asked-state so the loop doesn't re-ask.
4. `apply_delta` (:271-289), `apply_raw_answer` (:296-313 — Escalation:
   parse first integer, bounds-check 1-100 else None; Access: split lines /
   commas), `assemble_spec` (:342-364): escalation lands on the EXISTING
   `EscalationSpec` (`spec.rs:130-139` — `escalate_after_failures`,
   `escalate_to`; keep `Some(3)`/"the operator" defaults when skipped);
   access lands in a new `CapabilitySpec.access_integrations:
   Vec<String>` (`spec.rs:175-216`, `#[serde(default)]`, initialized in
   `from_broad_draft`) and surfaces via the contract's EXISTING
   `draft_integrations` (`role_contract.rs:99`) as named draft-only
   connectors — free text is fine (no MCP-registry validation at intake;
   unknown names are honest "to be connected" items).
5. `build_extract_prompt` + `slot_topic` (`intake_llm.rs:74-141`): add the
   three new JSON keys so one rich answer can fill them adaptively.
   `captured()` envelope (`routes_genesis_intake_conversation.rs:215-224`):
   include the new fields so the SPA capture panel shows them.

**Part 2 — per-slot quality bar (bounded).**
6. After a slot fills, run a vagueness check; if vague, ask ONE follow-up
   (hard cap 1 re-ask per slot, tracked in `IntakeState`), then accept
   whatever comes. Heuristics-first (deterministic, no provider needed):
   cadence lacking a time-of-day/day-of-week token; exceptions shorter than
   ~3 words. Optional LLM adequacy judge reusing the `SafetySignal`
   Allow/Block/**Unavailable** pattern (`routes_genesis_gate_judge.rs:23-54`)
   over the same `complete_once` transport — `Unavailable` (provider
   down/slow) ALWAYS degrades to accept. Intake must never error or stall on
   a model failure (mirror the existing wizard-fallback discipline at
   `routes_genesis_intake_conversation.rs:184-194`).

## May touch

- `crates/argo-genesis/src/intake_machine.rs` (427 LOC — ~70 LOC delta, fits)
- `crates/argo-genesis/src/intake_llm.rs` (171 LOC), `spec.rs` (360 LOC),
  `intake_store.rs` tests
- `crates/argo-cli/src/handlers/gateway/routes_genesis_intake_conversation.rs`
  (280 LOC) + tests; optional new `routes_genesis_intake_adequacy.rs` (~40
  LOC) if the LLM judge is included
- `web/components/skycot/genesis/ConversationIntake.tsx` capture-panel
  rendering of the two new sections (keep `genesis/page.tsx` delta ~0 — it
  is at 481/500 LOC)

## Acceptance / verifier

1. Conversation test: vague cadence "Weekly" triggers exactly ONE follow-up
   ("Which day and time?"); a second vague answer is accepted (cap honored).
2. Access + Escalation are asked; explicit "skip" completes them; captured
   values land in `assemble_spec` output (escalation overrides the
   `Some(3)`/"the operator" defaults; access lands in
   `access_integrations` → `draft_integrations`).
3. Adaptive regression: one rich answer filling cadence+access still
   collapses both (extend the existing
   `extract_delta_parses_embedded_json`-style test).
4. Provider-failure test: adequacy judge transport error → answer accepted,
   no user-visible error (Unavailable degrade).
5. Old intake-state JSON (without the new fields) still deserializes and
   completes (`#[serde(default)]` pinned by test).
6. `cargo test -p argo-genesis -p argo-cli` + vitest green; LOC budgets
   respected; no new parallel pub types (`EscalationSpec` reused, no new
   comparator enums).

## Size / owner / priority

~120-200 LOC incl. tests (+~40 if LLM judge included). **🟠 P2 — DEC-048
rule 4.** Sibling: `fix-genesis-intake-fidelity-captured-slots-must-land`
(P1 — lands capture-to-built fidelity; this brief deepens what gets
captured). Coordinate with `mission-genesis-access-controls-and-guardrails-spec-ui`
(CLAIMED): that brief owns the boundaries/approval-threshold UI panels;
THIS brief only adds intake slots that FEED existing spec fields — no policy
store, no BoundariesPanel here.
