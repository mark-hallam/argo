# fix-genesis-intake-fidelity-captured-slots-must-land

**Owner**: claude or codex.
**Depends on**: none (A1 #2711/#2713 + A2 #2715 already merged; this closes their last mile).
**Authored against**: origin/main `aac402865` on `2026-06-10`.
**Validate before claim**: bash scripts/check-brief-freshness.sh fix-genesis-intake-fidelity-captured-slots-must-land

## Goal ([DEC-048] rule 2 — captured-equals-built)

Every slot the Genesis intake captures MUST verifiably land in the minted
teammate, and the pre-hire review must display all of it. From this brief on,
a silently-dropped captured field is a P1 bug class (NO THEATER applied to
intake).

## Why now (live e2e 2026-06-10 + drilldown, all VERIFIED)

The capture side already works (merged #2711/#2713/#2715/#2724): cadence /
exceptions / success_criteria reach `CapabilitySpec`
(`routes_genesis_stream_intake.rs:63-96`) and KPIs reach the manifest
(`materialize.rs:285-310`). Then three losses:

1. **Cadence DISCARDED at hire.** `buildHireDraftFromGenesis`
   (`web/lib/api/genesis.ts:594-635`, hardcode at :611-618) never reads
   `ready.role_contract.cadence`; every Genesis hire gets cron `0 9 * * 1-5`
   + canned copy "Pick up inbound messages on the morning shift". Live repro:
   user typed "every weekday at 5pm", worker got the canned 9am schedule.
2. **Exceptions are display-only.** Enriched into the role contract for
   display (`routes_genesis_stream_payload.rs:52-60`,
   `role_contract.rs:227-232 with_intake_slots`) but never written into the
   minted pack — `system_prompt_from_spec`
   (`materialize_writer.rs:30-76`) has no exceptions section.
3. **The review card hides the capture.** `RoleContractReview.tsx:128-298`
   renders will_not / approval_required / escalation but NOT
   cadence / exceptions / success_criteria — the operator cannot catch loss
   1 or 2 before hiring.

## Fix

1. **Cadence → real schedule** (`genesis.ts:611-618`): delete the hardcoded
   cron + canned description. Thread `ready.role_contract.cadence`:
   - duty description = the USER'S OWN cadence words (never canned copy);
   - parse common English cadences to cron client/server-side
     deterministically ("every weekday at 5pm" → `0 0 17 * * 1-5`,
     "weekly"/"daily at H", weekday names). **Reuse** the canonical
     normalizer `normalize_stored_cron`
     (`routes_duties_helpers.rs:145-176`) + the `cron` crate
     `Schedule::from_str` (as `duty_scheduler/cron_eval.rs:24` does) — do
     NOT write a parallel cron stack;
   - unparseable cadence: keep the user's words in the description, fall
     back to the current default cron, and SAY SO on the review card
     ("schedule needs confirmation — edit in Schedules") rather than
     silently substituting.
2. **Exceptions → minted pack**: add a "What you will NOT do" section to
   `system_prompt_from_spec` (`materialize_writer.rs:66-75`, ~15 LOC)
   listing `spec.exceptions`. (Soft enforcement now; HARD fence enforcement
   is DEC-025 Phase-2 scope, coordinate with
   `mission-genesis-access-controls-and-guardrails-spec-ui` — do not fork a
   policy store here.)
3. **Review displays everything**: add three panels to
   `RoleContractReview.tsx` after the escalation panel (~60 LOC; mirror the
   existing `will_not` panel pattern + CSS classes): "When it runs"
   (cadence), "Exceptions / hand-offs", "How success is measured" (each KPI:
   name, kind, comparator, target). Keep props additive — this component now
   renders in every route group (#2777), so no breaking prop changes.
4. **Round-trip verifier (the contract)**:
   - `pipeline_tests.rs` (argo-genesis): spec with all 3 slots →
     `materialize_spec` → assert slots in plan.spec + KPIs in manifest +
     `with_intake_slots` carries all three;
   - gateway test: POST /api/v1/genesis with all 3 fields → `vertical_ready`
     payload carries all three;
   - web vitest: `buildHireDraftFromGenesis` respects cadence (cron parsed,
     no hardcode; canned-copy string asserted ABSENT) +
     `RoleContractReview` renders all three panels.

## May touch

- `web/lib/api/genesis.ts` (690 LOC, grandfathered — keep delta small; new
  helpers go in a sibling `genesis-*.ts`)
- `web/components/skycot/genesis/RoleContractReview.tsx` (298 LOC)
- `crates/argo-genesis/src/materialize_writer.rs` + `pipeline_tests.rs`
- `crates/argo-cli/src/handlers/gateway/routes_genesis_stream_*` tests
- web vitest siblings

## Acceptance / verifier

1. Genesis hire with cadence "every weekday at 5pm" → minted duty cron is
   17:00 weekdays AND duty description contains the user's words; the canned
   "Pick up inbound messages on the morning shift" string no longer exists in
   the repo's hire path (`rg` clean).
2. Exceptions text appears in the minted pack's `prompts/system.md`.
3. Review card shows cadence + exceptions + each typed KPI pre-hire;
   unparseable cadence shows the needs-confirmation note (no silent
   substitution).
4. Round-trip tests above green; `cargo test -p argo-genesis -p argo-cli` +
   web vitest + tsc green; LOC budgets respected.

## Size / owner / priority

~180-280 LOC incl. tests. **🔴 P1 — DEC-048 rule 2.** The hero intake
currently asks and then ignores; that manufactures the "useless teammate"
DEC-048 exists to prevent. Sibling briefs:
`genesis-interview-quality-bar-and-access-escalation-slots`,
`genesis-interview-default-mode-and-remove-speed-promises` (review-EDIT seam
lands there; display lands HERE — C depends on A's payload fields).
