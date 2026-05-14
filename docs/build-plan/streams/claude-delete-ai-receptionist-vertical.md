# claude-delete-ai-receptionist-vertical — Delete ai-receptionist vertical + cleanup 34 references

**Owner**: claude. **Depends on**: nothing (pure delete).
**Authored against**: origin/main `06c79fbd` on `2026-05-14`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-delete-ai-receptionist-vertical`.

## Scope

`ai-receptionist` is a stale demo vertical agent-authored mid-build that never became part of the v1 roadmap. It's causing two ongoing costs:

1. **Smoke test fails on step 14** (`scripts/smoke-skycot.sh`) because the vertical's `[escalation].human_approval_required` lists action names (`book_meeting`, `send_reply`, `transfer_call`, `share_caller_data_externally`) that the lint flags as orphan-tools.
2. **Stale-fixture noise** in 34 files (tests, docs, example bundles) that have to be reasoned about every audit cycle.

This brief deletes the vertical + every reference to it. **Do not confuse with the `receptionist` vertical** (no `ai-` prefix) — that's a different vertical and stays.

LOC: ~300 (mostly deletions across 34 files; touches average <10 lines each).

## Files you may touch

**`git rm -r` (whole-directory deletions):**
- `verticals/ai-receptionist/` — DELETE WHOLE DIRECTORY (auto_approval.toml, eval/, inventory.json, memory/, prompts/, skills/, vertical.toml).
- `examples/ai-receptionist/` — DELETE WHOLE DIRECTORY (README.md, sample-bundle/, sample-prompts.md).

**`git rm` (single-file deletions):**
- `crates/argo-cli/tests/vertical_ai_receptionist.rs` — vertical-specific test file.
- `docs/build-plan/streams/mission-vertical-ai-receptionist-skills.md` — stale brief for the deleted vertical.
- `docs/build-plan/streams/mission-vertical-ai-receptionist-flesh-out.md` — stale brief for the deleted vertical.

**TOUCH (remove ai-receptionist references only):**
- `crates/argo-team-headline/src/kpi_table.rs` — remove ai-receptionist case + entry from per-vertical KPI table.
- `crates/argo-team-headline/src/lib_tests.rs` — remove ai-receptionist test cases.
- `crates/argo-cli/tests/vertical_eval.rs` — remove ai-receptionist case from the multi-vertical eval test.
- `crates/argo-cli/tests/vertical_sales_follow_up.rs` — remove ai-receptionist reference (if any non-test fixture mentions it).
- `crates/argo-cli/src/handlers/gateway/marketplace_render.rs` — remove ai-receptionist case from marketplace rendering switch.
- `crates/argo-vertical/src/manifest_validate.rs` — remove if it contains ai-receptionist-specific validation rules.
- `crates/argo-vertical/src/manifest_validate_tests.rs` — remove ai-receptionist test fixtures.
- `scripts/smoke-skycot.sh` — remove step 14's ai-receptionist lint OR retarget it at the canonical `gtm-engineer` vertical (whichever is simpler — favor retarget so smoke still exercises vertical-lint).
- `scripts/README.md` — remove ai-receptionist mention.
- `examples/sales-follow-up/README.md` — remove ai-receptionist mention.
- `verticals/AUTHORING.md`, `verticals/AUTHORING_v3_ACTIONS.md`, `verticals/README.md` — remove ai-receptionist examples; substitute with `gtm-engineer` or `sales-follow-up` where an example was illustrative.
- `verticals/receptionist/skills/greet_visitor.skill.md` — remove any cross-reference to `ai-receptionist` (the `receptionist` vertical without the prefix STAYS).
- `docs/skycot/README.md` — remove ai-receptionist mention.
- `docs/skycot/manifest-kpi-schema.md` — remove ai-receptionist example KPI rows.
- `docs/skycot/launch_readiness.md` — remove ai-receptionist line.
- `docs/skycot/CHANGELOG.md` — leave historical entries that say "ai-receptionist landed in X" alone (those are factually correct historical record); only remove forward-looking references to it as a v1 vertical.
- `docs/skycot/PLUGIN_SYSTEM.md` — remove ai-receptionist mention.
- `docs/build-plan/MISSION_BACKLOG.md` — strike all ai-receptionist rows; if any are [SHIPPED] historical record, keep with note "(vertical since deleted, see claude-delete-ai-receptionist-vertical)".
- `docs/build-plan/streams/E3-c-team-headline-generator.md` — remove ai-receptionist case (if any sample test fixture mentions it).
- `docs/build-plan/streams/mission-mcp-connector-zoom.md` — remove ai-receptionist mention (likely just a "callable from" example — replace with `support-tier1` if illustrative).
- `docs/build-plan/streams/mission-per-vertical-regression-test.md` — remove ai-receptionist case from the per-vertical regression matrix; substitute with another v1 vertical.
- `docs/build-plan/BUILD_STATUS.md` — DO NOT TOUCH HERE (the conductor updates this from outside the worker; the worker only updates its own row from IN_REVIEW → MERGED post-PR-creation per standard worker flow).

## Files you must NOT touch

- `verticals/receptionist/` (without `ai-` prefix) — different vertical, STAYS.
- `verticals/gtm-engineer/` — canonical v1 vertical; if smoke-skycot needs a retarget, point at this one.
- Any crate not listed above.
- Anything outside the brief's may-touch set per CLAUDE.md.

## Acceptance criteria

- [ ] `rg ai-receptionist` returns zero matches OR only matches in `docs/skycot/CHANGELOG.md` historical entries.
- [ ] `rg ai_receptionist` returns zero matches.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo test --workspace` passes (removed tests are gone; remaining tests still green).
- [ ] `bash scripts/smoke-skycot.sh` exits 0 (step 14 passes or is removed/retargeted).
- [ ] `bash scripts/lint-brief.sh docs/build-plan/streams/claude-delete-ai-receptionist-vertical.md` exits 0.
- [ ] `pnpm typecheck` clean (web-side).
- [ ] No file > 500 LOC after changes.
- [ ] BUILD_STATUS.md row for this brief updated with PR number.

## Branch + PR

- Branch: `claude-delete-ai-receptionist-vertical`
- PR title: `cleanup: delete ai-receptionist vertical + 34 references (claude-delete-ai-receptionist-vertical)`

## Notes for the implementer

- Mission-claim alignment: production hardening (cleanup; supports all 6 claims by reducing stale-fixture noise + unblocks smoke test).
- The vertical was agent-authored mid-build and never became part of v1 scope. Operator confirmed 2026-05-14 to remove it.
- If you encounter a reference where ai-receptionist was used as an *example* for a more general concept (e.g., "verticals like ai-receptionist can…"), substitute with `gtm-engineer` or `sales-follow-up` — both are canonical v1 verticals.
- Smoke test step 14: prefer retargeting the vertical lint at `gtm-engineer` (canonical v1 vertical) over deleting the step entirely — we still want vertical-lint coverage in smoke.
- For `docs/skycot/CHANGELOG.md`, distinguish *forward-looking* references (remove) from *historical* records of when the vertical landed (keep, with optional " (vertical since deleted)" annotation).
- This is a pure delete — no new pub types introduced — so the parallel-surface drift check is N/A. The brief passes the drift-prevention rules trivially.
