# uf — extract helpers to create LOC headroom in genesis-gate + materialize (split-prep)

**Owner**: unclaimed (split-prep / unblocks A4 + C1)
**Depends on**: none.
**Authored against**: origin/main `b5cd734a7` on `2026-06-05`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh uf-genesis-gate-and-materialize-extract-helper`.

> **Why this exists:** A4 (`uf-hire-gate-wire-live-eval`) and C1 (`uf-genesis-workflow-emit`)
> are stuck TODO — workers skip them because `check-pre-claim-ceiling.sh` refuses any
> brief whose may-touch `.rs` file is >480 LOC, and both target files are at-risk:
> `routes_genesis_gate.rs` = 494, `materialize.rs` = 497. This is a **pure-extraction
> split-prep** (no behaviour change) that pulls a cohesive block out of each into a
> sibling module, dropping both **well under 480** so A4 + C1 become claimable. This
> brief is itself exempt from the ceiling check (split/extract brief).

## Extraction 1 — `routes_genesis_gate.rs` (494 → target ≤460)

Move the **judge / safety-signal cluster** into a new sibling
`crates/argo-cli/src/handlers/gateway/routes_genesis_gate_judge.rs`:
- `run_safety_signal` (~line 324)
- `struct GateJudgeClient` + its `impl GateJudgeClient` + `impl GenesisJudgeClient for GateJudgeClient` (~361-399)

…or another cohesive block of similar size if that reads cleaner (e.g. the
`run_*_floor` helpers at 208-308). Pick whichever keeps each side coherent.
Add `mod routes_genesis_gate_judge;` (and `use` re-exports as needed) so the call
sites in `routes_genesis_gate.rs` are unchanged. **Do not** alter behaviour, signatures,
or the `ContractSubject` logic (A4 will replace that next — leave it in place here).

## Extraction 2 — `materialize.rs` (497 → target ≤400)

Move the **name / slug-derivation helper cluster** into a new sibling
`crates/argo-genesis/src/materialize_naming.rs`:
- `derive_display_name`, `name_from_charter`, `take_name_tokens`,
  `humanize_persona_slug`, `title_case_token`, `truncate_description`,
  `derive_vertical_id`, `validate_vertical_id` (the run of fns ~342-494)

Add `mod materialize_naming;` + `use` so `materialize.rs` call sites are unchanged.
Keep `GenesisError` usage intact (these fns return it). **No behaviour change.**

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_genesis_gate.rs` (remove the moved block)
- `crates/argo-cli/src/handlers/gateway/routes_genesis_gate_judge.rs` (NEW)
- `crates/argo-cli/src/handlers/gateway/mod.rs` (one `mod` line)
- `crates/argo-genesis/src/materialize.rs` (remove the moved block)
- `crates/argo-genesis/src/materialize_naming.rs` (NEW)
- `crates/argo-genesis/src/lib.rs` (one `mod` line, if materialize's mod is declared there)
- this brief's BUILD_STATUS.md row only

## Acceptance

- `routes_genesis_gate.rs` ≤460 LOC **and** `materialize.rs` ≤400 LOC (re-check with
  `wc -l`; paste both in the PR body). Every new sibling file also <500.
- **Pure move** — `git diff` shows only relocation + `mod`/`use` wiring, no logic edits.
- Crate-scoped tests still pass: `cargo test -p argo-genesis` and
  `cargo test -p argo-cli routes_genesis_gate` (the gate's tests live in a `tests`
  sibling — they must still compile + pass). ⚠️ **1-slot memory caution:** another
  worker may be compiling — run crate-scoped, your own per-brief target dir, not a
  workspace build.
- After this lands, A4 + C1's `check-pre-claim-ceiling.sh` passes → they unblock.

## On ship

No FEATURE_MATRIX change (no capability ships — pure refactor). Update this brief's
BUILD_STATUS.md row to IN_REVIEW with the PR number in the same push that opens the PR.

## Drift / safe-git (inherited)

- No new pub types beyond the relocated items (which keep their existing visibility).
- `git add` explicit files only; rebase-before-push; never `git reset` shared main.
