# fix-demo-seed-add-pitch-agent-and-missing-cohort-rows

**Owner:** codex
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-demo-seed-add-pitch-agent-and-missing-cohort-rows`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F06)

🟡 **P2 — `/api/v1/cohorts` returns 5 cohorts (marketing, marketplace, project-coordination, recruiting, support) while the SPA dept-registry declares 9 departments. The Finance & Ops, IT & Dev-ex, Legal, and Sales & Investor Relations dept tabs never render because no cohort feeds them. Track P (Pitch Agent) is in v1.0 scope per operator memory `project_track_p_v1_mvp_scope` — its cohort row is missing from the seed.**

## Root cause

The demo seed (likely in `crates/argo-cli/src/handlers/demo_seed.rs` or equivalent — verify with `rg "seed_cohorts\|seed_cohort\|cohort_seed" crates/`) writes 5 cohort rows that match the 5 canon workers (Mariah/Peta/Priya/Jamal/Nori). The dept-registry at `web/components/skycot/cohorts/dept-registry.ts:18-58` declares 9 departments expecting cohorts for each.

Missing cohorts (4 of 9 dept tabs are empty):

- **Finance & Ops** — no cohort row.
- **IT & Dev-ex** — no cohort row.
- **Legal** — no cohort row.
- **Sales & Investor Relations** — this is where the **pitch-agent cohort** belongs. Per memory `project_track_p_v1_mvp_scope` Track P is in v1.0 scope and the pitch-agent cohort SHOULD be visible on `/cohorts` once Track P workers (Skylar / Fen / Mira — replaced with canon roster per `fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira`) ship.

For the v1.0 demo MP4 the camera doesn't visit `/cohorts`, so this is P2 not P0. But the dressed-empty dept tabs are visible if a viewer pans there, and the pitch-agent cohort absence breaks Track P's "this is in v1.0" claim.

## Findings this brief closes

- whole-site F06 [P2] — Demo seed missing 4 of 9 dept cohorts; Sales & Investor Relations (pitch-agent) absent despite Track P in v1.0 scope

## Fix recipe

1. **Locate the demo seed cohort writer**. Grep:
   ```
   rg "seed_cohort\|cohort.*seed\|insert.*cohorts" crates/
   ```
   Likely lives at `crates/argo-cli/src/handlers/demo_seed.rs` or under `crates/argo-store/src/seed/`. Read the existing 5 cohort inserts as a template.
2. **Add 4 net-new cohort rows** to the seed:
   - **`pitch-agent`** — Sales & Investor Relations dept. Lead worker: a canon Track-P persona (operator-decisional which one; check `fix-vertical-landing-non-canonical-worker-names-skylar-fen-mira` for the canon list).
   - **`finance-ar`** — Finance & Ops dept. Lead worker: TBD (operator-decisional; could be a placeholder until F-named cohorts ship).
   - **`it-devex`** — IT & Dev-ex dept. Lead worker: TBD.
   - **`legal-counsel`** — Legal dept. Lead worker: TBD.
3. **Each cohort row** needs: name, slug, vertical id, dept id, lead-worker id, brief description, and the magazine-grid metadata fields the dept-registry expects. Cross-check the 5 existing rows for the full shape.
4. **Coordinate with `fix-cohorts-dept-registry-add-project-coordination-vertical-alias`** — that brief widens the registry's vertical-id matching; this one widens the seed's cohort coverage. Both can land independently.
5. **Add an integration test** at `crates/argo-cli/tests/demo_seed_cohort_coverage.rs`:
   - Run the demo seed against a fresh DB.
   - Fetch `GET /api/v1/cohorts` with the demo tenant.
   - Assert ≥9 cohorts returned (one per dept).
   - Assert the `pitch-agent` cohort exists with a non-empty lead-worker id.

## Acceptance criteria

- [ ] `argo demo seed` writes ≥9 cohort rows (one per dept).
- [ ] `/cohorts` page renders all 9 dept tabs with at least one cohort each in the running preview.
- [ ] Sales & Investor Relations tab includes the pitch-agent cohort row.
- [ ] `cargo test -p argo-cli --test demo_seed_cohort_coverage` green.
- [ ] Existing 5 cohort rows + workers continue to render unchanged.
- [ ] PR body cites whole-site F06 and links to memory `project_track_p_v1_mvp_scope`.

## May-touch

- `crates/argo-cli/src/handlers/demo_seed.rs` (or equivalent seed location)
- `crates/argo-store/src/seed/cohorts.rs` (if helper extraction needed)
- `crates/argo-cli/tests/demo_seed_cohort_coverage.rs` (NEW)

## Out of scope

- SPA-side dept-registry alias (covered by `fix-cohorts-dept-registry-add-project-coordination-vertical-alias`)
- /retro + /orchestration empty-state seeding (covered by separate brief on whole-site F08)
- Lead-worker selection for the 3 placeholder dept cohorts (operator-decisional; can be a placeholder name until the right cohort ships)
- Track P brief tree (covered by P-01..P-08 stream, separate scope)

## Effort

S — ~80 LOC seed additions + ~60 LOC integration test. ~3-4 hours including the Rust roundtrip for the new rows.
