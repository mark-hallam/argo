# fix-demo-seed-retro-and-orchestration-empty-state

**Owner:** codex
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-demo-seed-retro-and-orchestration-empty-state`
**Source:** UX e2e audit 2026-05-28-0252 (whole-site F08)

🟡 **P2 — `/retro` and `/orchestration` render only "no records yet" empty states in the authed demo session. For the v1.0 demo MP4 the camera doesn't visit either, so not a recording blocker — but for any walkthrough longer than the 6-scene runbook (investor Q&A, longer pitches), these empty states are dead-ends. The demo seed has zero coverage for these two surfaces.**

## Root cause

The demo seed (in `crates/argo-cli/src/handlers/demo_seed.rs` or equivalent — verify via grep `rg "seed.*retro\|seed.*orchestration\|retros\|orchestration_patterns" crates/`) writes no rows for either:

- `/retro`: "0 retros across the tenant — No retros yet — Capture what worked + what didn't last sprint."
- `/orchestration`: "Orchestration — No orchestration yet"

Both pages render their locked empty-state designs correctly — the bug is at the seed layer, not the UI. For investor demos that go beyond the 6-scene runbook (e.g. answering "what about retros?" live), the operator needs at least 2-3 seeded examples to point at.

Sibling briefs on the seed:
- `fix-demo-seed-cli-for-real-roster` (MERGED-STALE-FIXED) — roster.
- `fix-demo-seed-cli-operator-action-runbook` (MERGED #2385) — operator-runbook for the CLI.
- `fix-demo-seed-emit-worker-hired-audit-rows` (BLOCKED) — audit rows.
- `fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona` — workflow seeds.

This brief is the parallel coverage for /retro + /orchestration.

## Findings this brief closes

- whole-site F08 [P2] — Demo seed missing /retro and /orchestration content; both render empty states

## Fix recipe

1. **Locate the existing seed helpers** for cohorts / workers / workflows. Mirror the pattern for retros + orchestration.
2. **Seed 3 retros** tied to existing canon workers (Mariah / Peta / Jamal):
   - "Sprint 28 retro · marketplace pricing experiment" — Mariah-led, mixed verdict, 4 "kept" / 2 "stopped" items.
   - "Q1 review · candidate funnel improvements" — Priya-led, positive verdict.
   - "Standup digest reliability post-mortem" — Patrick-led, negative verdict with 3 action items.
3. **Seed 2 orchestration patterns**:
   - "Marketplace listing → re-price weekly" — Mariah, schedule-driven.
   - "New customer onboarding → handoff to project-coordination" — Peta, event-driven.
4. **Cross-link**: each retro should reference at least one workflow + one worker so the SPA's retro detail can render a real provenance chain.
5. **Integration test** at `crates/argo-cli/tests/demo_seed_retro_orchestration_coverage.rs`:
   - Run the seed against a fresh DB.
   - Fetch `GET /api/v1/retros` (verify endpoint name) → assert ≥3 rows.
   - Fetch `GET /api/v1/orchestration` (verify endpoint name) → assert ≥2 rows.
6. **Verify the SPA renders** — load `/retro` and `/orchestration` in the running preview and confirm rows display correctly.

## Acceptance criteria

- [ ] Demo seed writes ≥3 retros + ≥2 orchestration patterns.
- [ ] `/retro` renders 3+ retro cards in the running preview.
- [ ] `/orchestration` renders 2+ patterns in the running preview.
- [ ] Each retro references a real worker + at least one workflow.
- [ ] `cargo test -p argo-cli --test demo_seed_retro_orchestration_coverage` green.
- [ ] PR body cites whole-site F08.

## May-touch

- `crates/argo-cli/src/handlers/demo_seed.rs` (or equivalent)
- `crates/argo-store/src/seed/retros.rs` / `…/orchestration.rs` (NEW if needed)
- `crates/argo-cli/tests/demo_seed_retro_orchestration_coverage.rs` (NEW)

## Out of scope

- Retro detail-page UX (separate scope)
- Orchestration detail-page UX (separate scope)
- Audit-row seeding (covered by `fix-demo-seed-emit-worker-hired-audit-rows` BLOCKED)
- Workflow seeding (covered by `fix-demo-seed-publish-workflow-definitions-and-fire-one-execution-per-persona`)

## Effort

S/M — ~120 LOC seed code + ~80 LOC integration test. ~4-5 hours including the Rust roundtrip and SPA verification.
