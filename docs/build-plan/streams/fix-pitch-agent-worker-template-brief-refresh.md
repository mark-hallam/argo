# fix-pitch-agent-worker-template-brief-refresh

**Owner**: claude.
**Authored against**: origin/main `767876b89` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-pitch-agent-worker-template-brief-refresh`.
**Unblocks**: Track P P-05 (`pitch-agent-worker-template`) — currently re-BLOCKED 2026-05-25T10:22Z by codex heartbeat pre-implementation for API drift against current main.

🔴 **P0 META-BRIEF — refreshes BLOCKED P-05 brief against current main.**

The Track P P-05 brief (`docs/build-plan/streams/pitch-agent-worker-template.md`) was BLOCKED at T10:22Z by codex heartbeat after the worker claimed at T10:16Z and discovered two API drift points:

1. **Pack-loader contract mismatch**: brief specifies `manifest.toml` + `argo-vertical-packs::Loader::load`, but current `argo-vertical-packs` exposes `PackManifest::load(root)` reading `pack.toml`. Implementing the brief as-written would require either changing loader infrastructure (explicitly forbidden) or adding a parallel manifest format (parallel-pub-types violation per CLAUDE.md core-platform invariants).

2. **Cohorts UI tab-registry has moved**: brief allows only `web/app/(app)/cohorts/page.tsx`, but the dept tab registry, tile metadata, and mock/demo cohort rows now live in three sibling files:
   - `web/components/skycot/cohorts/dept-registry.ts`
   - `web/components/skycot/cohorts/tile-meta.ts`
   - `web/lib/sample-cohorts.ts`
   Adding the "Sales & Investor Relations" 8th tab without touching those would require hardcoding page-local data — violates the locked D3-magazine-tiles design pattern (memory `project_cohorts_page_design_locked`).

## Why this brief exists

P-05 is the load-bearing brief that assembles all Track P parts (P-01 schema + P-03 voice + P-04 canonical-QA + persona pack + memory schema + tool permissions + cohorts UI) into a runnable vertical. Without it, P-06/P-07/P-08 stay BLOCKED. P-02 just unblocked + claimed; P-05 needs the same brief-refresh pattern that worked for P-02 (the `fix-pitch-truth-boundary-runtime-enforcement-brief-refresh` meta-brief shipped via #2282).

## Scope (this brief — edit the OTHER brief; do NOT implement P-05 here)

Edit `docs/build-plan/streams/pitch-agent-worker-template.md` in place:

1. **Pack-loader contract fix** — choose ONE option, document the choice + 1-sentence rationale at the top of the brief:
   - **Option A (preferred — minimal scope)**: re-target to current pack format. Add `packs/pitch-agent/` directory with `pack.toml` (the canonical schema), `worker.toml`-style pack files, persona files in `packs/pitch-agent/personas/`. References to `manifest.toml` / `Loader::load` → `pack.toml` / `PackManifest::load`. May-touch list adds `packs/pitch-agent/**`.
   - **Option B (verticals-format)**: also lay down `verticals/pitch-agent/vertical.toml` so the vertical loader (`argo_vertical::load_dir`) round-trips per the `argo-vertical-packs` audit findings. Bigger scope; references both `pack.toml` (marketplace listing) + `vertical.toml` (runtime load). May-touch list adds `packs/pitch-agent/**` + `verticals/pitch-agent/**`.

2. **Cohorts UI fix** — widen may-touch to include the cohorts registry triple:
   - Add to may-touch: `web/components/skycot/cohorts/dept-registry.ts` (register the 8th dept tab `sales-investor-relations`)
   - Add to may-touch: `web/components/skycot/cohorts/tile-meta.ts` (declare the Pitch worker's tile metadata)
   - Add to may-touch: `web/lib/sample-cohorts.ts` (add the Pitch sample-cohort row)
   - `web/app/(app)/cohorts/page.tsx` stays in may-touch (likely no edits needed if the registry triple drives the page render via the existing component layer)

3. **Persona pack mode-count**: the brief specifies a 4-mode persona pack (30-sec / 3-min / 10-min / DD-deep). Verify the current persona-pack schema in `packs/` supports a `mode` discriminator. If not, either (a) flatten the modes into 4 sibling persona files, or (b) flag mode-discriminator as a P-05 sibling brief for `argo-vertical-packs` schema extension.

4. **Re-check freshness** — update `Authored against:` SHA to current `origin/main` (whatever's current at refresh time). Run `bash scripts/check-brief-freshness.sh pitch-agent-worker-template`; confirm exit 0 against current main.

5. **Flip ledger row BLOCKED → TODO** for `pitch-agent-worker-template` (around L1356 of BUILD_STATUS.md) once refresh lands. Conductor handles the flip.

## Acceptance criteria

- [ ] `docs/build-plan/streams/pitch-agent-worker-template.md` may-touch paths all exist on current `origin/main` (verify with `ls`).
- [ ] No reference to `manifest.toml` / `Loader::load` survives — replaced with `pack.toml` / `PackManifest::load` (or whichever option is chosen).
- [ ] Cohorts UI section explicitly names `dept-registry.ts` + `tile-meta.ts` + `sample-cohorts.ts` as may-touch.
- [ ] Option A vs B decision documented in brief intro with rationale.
- [ ] `bash scripts/check-brief-freshness.sh pitch-agent-worker-template` exits 0.
- [ ] BUILD_STATUS.md row for `pitch-agent-worker-template` flipped BLOCKED → TODO.

## May-touch

- `docs/build-plan/streams/pitch-agent-worker-template.md` (refresh)
- `docs/build-plan/BUILD_STATUS.md` (P-05 BLOCKED → TODO flip — conductor handles via ledger script)
- This brief file (status updates)

## Out of scope

- Implementing P-05 itself (separate worker claim after refresh lands)
- Touching `argo-vertical-packs` source code (consume existing public API only)
- Touching `web/components/skycot/cohorts/*.ts` source (consume; declare may-touch)

## Effort

~15-25 min — pure doc edit (~80-150 LOC of brief delta).
