# fix-vertical-packs-ship-zero-workflows-no-duties-after-install

**Owner**: codex.
**Authored against**: origin/main `b77864a000a9738c80f235fc2605d192197cd5c7` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-vertical-packs-ship-zero-workflows-no-duties-after-install`.
**Source**: e2e-ux-audit T0819 — `docs/skycot/ux-audits/2026-05-27-0819/onboarding-hire-an-ai-teammate.md` F16 [P3].

🟢 **P3 — `POST /api/v1/onboarding/install-vertical` returns `{"installed":true,"skills_added":5,"workflows_added":0}` for both `marketing-intern` and `sales-sdr`. The operator is told skills are ready but no schedules ("duties") are installed. Scene 2's demo beat ("click Run now on a duty → audit row lands in 5s") for a freshly hired worker has no duties to run.**

## Root cause

The vertical-pack manifests at `crates/argo-gateway/src/verticals/` (or wherever pack definitions live) ship with `skills[...]` populated but `workflows[...]` empty. `install-vertical` reports `workflows_added: 0` honestly — there's nothing in the pack to install.

Per memory `project_vocabulary_duty_vs_schedule`: UI says "Schedule", code says "duty". For Scene 2's narration to work, every shipped vertical needs ≥1 duty/schedule that:
- Has a cadence (e.g. "daily 06:30", "as items arrive")
- Triggers a turn against the worker
- Lands in the audit feed when "Run now" is clicked

Without this, the freshly hired worker's Overview tab shows the schedules card empty / disabled (which it does — see scene-1-2 audit's resolved findings: "Duty toggle is no longer a fake-mutation stub; renders disabled=true").

## Findings this brief closes

- onboarding-hire-an-ai-teammate F16 [P3] — `install-vertical` reports `workflows_added: 0` for every vertical; Scene 2's "click Run now on a duty" has no target

## Fix recipe

1. **Author one workflow per shipped vertical.** Each vertical pack manifest needs ≥1 entry in `workflows[]`. Minimum content per workflow:
   - `name`: cron-shaped e.g. "Daily blog brief check-in" (marketing-intern), "Morning pipeline triage" (sales-sdr)
   - `cadence`: `"daily 06:30"` or `"as items arrive"`
   - `steps[]`: 2-4 steps using existing step kinds (`trigger`, `skill_invoke`, `notify`)
   - `risk`: `"autonomous"` or `"approval-required"`
2. **Update `install-vertical` to propagate workflows.** Confirm the install handler reads `workflows[]` from the pack manifest and writes them to the tenant's `workflows` table. If the handler already does this and the count is 0 because the manifest is empty, the fix is purely manifest-side.
3. **Verify per-vertical coverage.** Audit all shipped verticals (marketing-intern, sales-sdr, gtm-engineer, recruiter, customer-support, etc.) — each needs ≥1 workflow.
4. **Cargo test** in `crates/argo-gateway/tests/`: install each shipped vertical against a fresh tenant, assert `workflows_added ≥ 1`.
5. **SPA-side**: after install, `/workers/{id}/overview` should render the workflow as a duty row in the Schedules card. Verify the new manifests propagate through to that surface.

## Acceptance criteria

- [ ] Every shipped vertical pack has ≥1 workflow in its manifest.
- [ ] `install-vertical` returns `workflows_added ≥ 1` for each.
- [ ] Freshly hired worker's Overview tab renders ≥1 duty/schedule row.
- [ ] Cargo test asserts the invariant.
- [ ] Scene 2 narration ("click Run now on the daily blog brief check-in") has a real target.

## May-touch

- `crates/argo-gateway/src/verticals/*.toml` (or whatever pack format) — add `workflows[]` entries
- `crates/argo-cli/src/handlers/gateway/onboarding/install_vertical.rs` (verify propagation logic)
- `crates/argo-gateway/tests/install_vertical_test.rs` (NEW or extend)
- Possibly seed-side data files for the canonical 7 verticals

## Out of scope

- The "Run now" gateway 403 (covered by `fix-workflow-execute-demo-principal-capability-grant`)
- Workflows that use `mcp_invoke` + `cu_action` step kinds (covered by `fix-scene-5-customer-research-followup-workflow-and-mcp-cu-step-kinds`)
- Demo seed CLI (covered by `fix-demo-seed-cli-for-real-roster`)

## Effort

S-M — ~50-150 LOC across manifest authoring + tests. ~2-3 hours. P3 because Scene 2 IS recordable today using one of the seed workflows on the catalog page; this fix improves the hire-then-run flow which the runbook doesn't strictly demand.
