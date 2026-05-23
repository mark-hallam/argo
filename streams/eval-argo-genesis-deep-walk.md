# eval-argo-genesis-deep-walk

**Priority**: P1 (substrate audit — mission claim 3 "The platform builds workers"; gap-manifest §Crates NOT YET TOUCHED row #5)
**Owner**: claude
**Type**: Read-only audit — public surface inventory + cross-tenant pin tests + FINDINGS-ONLY `#[ignore]`'d tests naming fix-briefs
**Size**: ~300-500 LOC (audit report + 1-2 pin test files; argo-genesis is 8882 LOC across 40+ files)
**Authored against**: origin/main `576e48f5c` on 2026-05-24
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-argo-genesis-deep-walk`

## Why

`argo-genesis` is mission claim 3 — "The platform builds workers."
It turns a plain-English role description into a complete vertical
artefact bundle (charter / persona / skills / memory schema / eval
seeds / workflow scaffold / safety judge). 8882 LOC across 40+
source files; sibling smoke tests `eval-genesis-pipeline-end-to-
end-smoke` (PR #2084) and `eval-genesis-output-loads-cleanly-
through-argo-vertical` (PR #2119) cover the happy path end-to-end,
but the **gap manifest §"Crates NOT YET TOUCHED" row #5 marks
`argo-genesis` as 🟡 P1 — needing a deep substrate walk**.

The smoke tests run the pipeline as a black box. They do NOT pin
the public surface contract — which means:
- An additive-breakage upstream (e.g. argo-judge surface change)
  could quietly drop genesis to a degraded path without the
  smoke catching it.
- Tenant scoping at intermediate stages (plan / write_vertical /
  safety_judge / genesis_judge) is not pin-tested.
- The `Charter` / `Persona` / `Archetype` etc. public structs are
  not pinned against a parallel-pub-types collision risk (drift-
  prevention rule §3).

Mission claim 3 is now LIVE end-to-end on `main` post-#2106
(workflow execute route) + #2109 (claim-3-LIVE regression). This
deep-walk audit is the **quality** pass on the LIVE substrate —
fix the latent gaps before they become demo-day surprises.

## What

Mirror the shape of `eval-argo-routing-crate-surface` (#2115),
`eval-argo-replay-after-1992-fix` (#2130), and (companion brief
in same wave) `eval-argo-mcp-core-surface`:

1. **Audit report** at
   `docs/skycot/evals/scheduled/2026-05-24/eval-argo-genesis-
   deep-walk.md`. Sections (≤500 LOC total — keep terse, use
   bullets):
   - **Public surface inventory** — every `pub` from
     `crates/argo-genesis/src/lib.rs` re-exports + every `pub fn`
     in the 40+ source files. Group by stage: parse → broad-role
     → archetype → charter → persona → skills → memory → eval
     seeds → workflow scaffold → safety judge → genesis judge →
     manifest writer.
   - **Production callers** — for each surface, enumerate via
     `rg 'argo_genesis::<Name>' crates/`, skipping
     `crates/argo-genesis/`'s own tests. Confirm every surface
     has either (a) a production caller or (b) a deliberately-
     documented future-additive contract.
   - **Parallel-pub-types check** — for every `pub struct` /
     `pub enum`, `rg 'pub (struct|enum) <Name>' crates/` — flag
     any name collision across crates. This is the drift-
     prevention rule §3 ("No parallel abstractions") applied to
     a single deep crate.
   - **Per-stage verdict**: 🟢 (tenant-scoped + pin-tested) /
     🟡 (present but unpinned) / 🔴 (cross-tenant leak / dead
     surface / parallel collision).
2. **Pin test file** at
   `crates/argo-genesis/tests/genesis_surface_pin_test.rs`
   (~150-300 LOC) — minimum:
   - One round-trip test per public stage that the smoke tests
     don't cover (i.e. unit-level pins, not the existing end-
     to-end).
   - One cross-tenant negative: a tenant-bound stage refuses a
     foreign tenant_id mid-pipeline (if the stage takes a
     `TenantId`).
3. **FINDINGS-ONLY `#[ignore]`'d tests** (zero, one, or more —
   shipped only if the audit surfaces a real broken-wire) each
   naming a fix-brief in its `#[ignore = "FINDINGS-ONLY: …,
   fix in <brief-id>"]` attribute.

## Acceptance

- `cargo test -p argo-genesis --test genesis_surface_pin_test`
  → all non-`#[ignore]`'d tests pass.
- `cargo clippy -p argo-genesis --all-targets -- -D warnings`
  clean.
- Workspace clippy fast-fail unchanged (or surface
  pre-existing-only drift via boundary note).
- Audit report cites the live `origin/main` SHA + `file:line`
  pointers per finding.
- If any FINDINGS surface, each one names a fix-brief that does
  NOT yet exist in `streams/` (verify pre-PR with `ls streams/
  | grep <brief-id>`).
- PR body includes the per-stage verdict tally + a one-line cite
  back to gap manifest row #5.
- Update `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-
  gap-manifest.md` §"Crates NOT YET TOUCHED" row #5 from 🟡 P1 →
  🟢 CLEARED (if no FINDINGS) or 🟡 AUDITED (if FINDINGS shipped
  with fix-briefs queued).

## May-touch

- `crates/argo-genesis/tests/genesis_surface_pin_test.rs` (new).
- `docs/skycot/evals/scheduled/2026-05-24/eval-argo-genesis-
  deep-walk.md` (new).
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-
  manifest.md` (single-line edit to row #5).
- **NOT** any production source — this is an audit. Findings
  ship as `#[ignore]`'d tests with named fix-briefs.

## Source

- Gap manifest §"Crates NOT YET TOUCHED" row #5 explicitly marks
  `argo-genesis` (deep walk through 8881 LOC) as 🟡 P1.
- Mission claim 3 (Genesis pipeline + workflow builder).
- Sibling smoke tests #2084 + #2119 (happy path) + #2109
  (LIVE regression) — this audit is the orthogonal quality pass.
- Drift-prevention rule §3 (no parallel abstractions) +
  `lint-parallel-pub-types.py` baseline check.

This is an evaluator-style audit, **not** a fix brief. Do NOT
inline-fix anything the audit turns up; ship findings as
`#[ignore]`'d tests + named fix-briefs.
