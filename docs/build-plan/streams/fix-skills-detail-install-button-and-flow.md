# fix-skills-detail-install-button-and-flow

**Owner**: claude.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-skills-detail-install-button-and-flow`.
**Source**: e2e-ux-audit Agent 3 finding F5 (`docs/skycot/ux-audits/2026-05-25-1100-scene-4-and-5-claim-4-and-5.md`).

🔴 **P0 DEMO-GATE — Scene 4 step 3 "Click Install on the verified result" — there is no Install button.**

## Root cause

Scene 4 step 3 narration: "Click **Install** on the verified result. Progress strip: 'Verifying provenance… Installing skill… Available to workers.'"

Today, `/skills/[skillId]` (`web/app/(app)/skills/[skillId]/page.tsx`) renders provenance + workers-using + adoption-count, but:
- Zero Install affordance on the page (`grep -i install web/app/(app)/skills/` returns one false-positive in an EmptyState header)
- No `<InstallButton>` component
- No `POST /skills/install` client wrapper
- No three-step progress strip
- No success toast or post-install state

DOM-verified by Agent 3: `document.querySelectorAll('main button, main a')` on a skill-detail page returns one anonymous button (back-to-marketplace link). The narrated Scene 4 install beat is structurally absent.

Per the function-vs-dressing rule: this is function-side — the install IS the claim. Without it, Claim 4 ("Skills shared + learned") has no demonstrable path on `/skills/<id>`.

## Findings this brief closes

- Agent 3 F5 [P0] — `/skills/[skillId]` has no Install button + no install flow

## Fix recipe

1. **Author `<InstallSkillButton>` component** at `web/components/skycot/skills/InstallSkillButton.tsx`:
   - Three states: idle ("Install"), in-flight ("Verifying provenance… → Installing skill… → Available to workers"), success ("Installed · Available to {workerCount} workers"), error.
   - On click: `POST /api/v1/skills/{id}/install` with `{tenant_id, worker_scope: "all" | [worker_ids...]}`.
   - Progress strip animation timed to the actual install steps (or a 3-tick simulated cadence if the gateway returns one-shot).

2. **Wire to `/skills/[skillId]/page.tsx`**: place the Install button prominently in the page header alongside the provenance pill. Visible above the fold.

3. **Gateway endpoint** `POST /api/v1/skills/{id}/install` (if not already shipped):
   - Validates provenance (signature check).
   - Writes a `skill.installed` audit row scoped to the calling tenant.
   - Returns `{installed_at, available_to_worker_count, audit_row_id}` so the SPA can deep-link to the audit row.
   - **Cross-tenant binding test**: install on tenant A; assert tenant B's `/api/v1/skills` does NOT show it. This is the "fresh install propagates to Maya in <5 s" narration beat AND the multi-tenant claim-6 test.

4. **Post-install state on the skill-detail page**: once installed, the button changes to "Uninstall" (or a "Manage" affordance) + a small "Installed on {tenant_id}" badge. Worker-scope picker hidden behind an Advanced disclosure.

## Acceptance criteria

- [ ] `/skills/<id>` renders a visible "Install" button above the fold on every skill detail page.
- [ ] Clicking Install posts to `POST /api/v1/skills/{id}/install` and writes a `skill.installed` audit row.
- [ ] The three-step progress strip from the runbook narration is visible during install.
- [ ] After install, the same skill on `/skills` is marked installed (or filtered to the installed-tab).
- [ ] Cross-tenant: install on tenant A, then `curl -H "X-Tenant-Id: tenant_b" /api/v1/skills` does NOT show it as installed.
- [ ] Audit row created by install has the real `worker_id` (or "system" if install is tenant-level, not worker-level).
- [ ] Component + integration tests for the install flow.

## May-touch

- `web/app/(app)/skills/[skillId]/page.tsx` (add button)
- `web/components/skycot/skills/InstallSkillButton.tsx` (NEW)
- `web/lib/api/skills.ts` (add `installSkill(id, scope)`)
- `crates/argo-cli/src/handlers/gateway/routes_skills*.rs` (new install endpoint, if not already shipped)
- `crates/argo-skill/src/install.rs` (the actual installer logic; may already exist)
- Tests for both layers.

## Out of scope

- Seeding skills the demo searches for (separate brief: `fix-skills-marketplace-search-seed-competitor-pricing`)
- Gateway routing `/skills` vs `/api/v1/skills` + CORS preflight (separate brief: `fix-skills-gateway-route-prefix-and-cors-preflight`)
- Per-skill version upgrade flow (follow-up)

## Effort

~150-300 LOC across SPA + gateway + tests. ~2-3 hours.
