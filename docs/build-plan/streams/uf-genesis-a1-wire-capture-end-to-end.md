# uf-genesis-a1-wire-capture-end-to-end (UF Phase-A · A1-wire)

**Owner**: claude.
**Depends on**: A1 schema (MERGED — `CapabilitySpec.cadence/exceptions/success_criteria` + `CadenceExceptionsCapture.tsx` are on main). tenant keystone (#2676).
**Authored against**: origin/main `3df1eedb8` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-genesis-a1-wire-capture-end-to-end`.

## Why (surfaced by the A1 / #2711 adversarial review)
A1 landed the **schema + the panel component + the display payload**, but the capture is NOT end-to-end:
- `CadenceExceptionsCapture.tsx` is **NOT mounted** in `web/app/(app)/genesis/page.tsx` (verified: 0 refs).
- The genesis **input** path leaves the slots empty — `CapabilitySpec::from_broad_draft` does not populate
  `cadence/exceptions/success_criteria` (pinned by the test `from_broad_draft_leaves_new_intake_slots_empty`).
The DISPLAY side already works (`routes_genesis_stream_payload.rs::build_final_payload` outputs the 3 fields).
So today `plan.spec.success_criteria` is always empty → A2's emit yields the default KPI skeleton, not the
user's real goals. Wire the capture so the operator's stated schedule/exceptions/goals actually land in the spec.

## Task
1. **Mount the panel.** Add `CadenceExceptionsCapture` to `web/app/(app)/genesis/page.tsx` following the
   existing Phase-2 panel mount pattern (e.g. how `SopUpload` is mounted); thread its captured state into the
   genesis intake submission (the describe/SSE request body). Locked brand tokens only (DEC-027).
2. **Parse on the gateway.** Where the genesis intake builds the `CapabilitySpec` from the user's description
   (grep: `rg 'from_broad_draft|CapabilitySpec::' crates/argo-genesis/src` + the genesis describe/SSE INPUT
   handler in `routes_genesis_*`), populate `cadence/exceptions/success_criteria` from the submitted values.
   Do NOT change `from_broad_draft`'s broad-draft contract if the values arrive via a separate input field —
   wire them at the input-handler layer. Reuse the manifest `KpiKind`/`KpiComparator` for success-criteria.

## Acceptance
An operator entering a schedule + an exception + a measurable goal in `/genesis` has all three captured into
`plan.spec.success_criteria` (**non-empty**) and visible in the `vertical_ready` review. Pairs with A2 (emit) +
A5 (surface) for the full user-goal → real-KPI flow. Back-compat preserved (empty when not provided).

## On ship (same commit)
Update `docs/build-plan/FEATURE_MATRIX.md` (genesis end-to-end capture row) + regen HTML
(`python3 scripts/render-feature-matrix.py`).

## Effort
~1 brief (M) — web mount + gateway input parse + an end-to-end-ish test.
