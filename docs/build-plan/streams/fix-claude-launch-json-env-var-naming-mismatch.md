# fix-claude-launch-json-env-var-naming-mismatch

**Owner**: claude or codex.
**Authored against**: origin/main `0653c478d` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-claude-launch-json-env-var-naming-mismatch`.
**Source**: e2e-ux-audit Agent 2 finding F14 (`docs/skycot/ux-audits/2026-05-25-1100-scene-1-and-2-claim-1-and-2.md`).

🟡 **P1 — `.claude/launch.json` sets env vars by the wrong names, so the demo data path silently does not load.**

## Root cause

`.claude/launch.json` sets:
- `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`
- `NEXT_PUBLIC_ARGO_DEMO_MODE=1`

But the SPA reads different names:

1. `web/lib/skycot/demo-mode.ts:20` reads `NEXT_PUBLIC_SKYCOT_DEMO_MODE` (different name from `NEXT_PUBLIC_ARGO_DEMO_MODE`). So `isDemoMode()` returns `false` in the dev profile, and `fetchTeam` does NOT fall back to `SAMPLE_TEAM` on a gateway 404 → team page silently renders the empty state.

2. `web/lib/api/team.ts:42` reads `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS` (matches the var in `middleware.ts`). But `.claude/launch.json` writes `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` (missing the `ARGO_` prefix). So the auth-bypass branch in the lib never fires.

Net effect: a fresh `pnpm dev` from this config gives the operator the empty-state CTA grid ("Hire your first teammate") instead of the demo team. Anyone resuming any audit on a new machine hits the same problem.

Cross-references the project-rename memory: the project is rebranding from "argo" → "skycot" but the `NEXT_PUBLIC_*` namespace mid-migration. This is the inconsistency that's biting us.

## Findings this brief closes

- Agent 2 F14 [P1] — Two env-var naming mismatches in `.claude/launch.json` prevent the demo data path from loading

## Fix recipe

Three options; pick (a) for the cleanest end-state:

**(a) Update `.claude/launch.json` to match the lib names (preferred — narrowest scope)**:
1. Change `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1` → `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1`
2. Change `NEXT_PUBLIC_ARGO_DEMO_MODE=1` → `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1`
3. Confirm a fresh `pnpm dev` from the updated config renders the demo team (5 workers post-seed, or `SAMPLE_TEAM` if gateway returns 404).

**(b) Update the lib readers to honor both names with a deprecation note**:
1. In `web/lib/skycot/demo-mode.ts:20` and `web/lib/api/team.ts:42`, check both old and new names: `process.env.NEXT_PUBLIC_SKYCOT_DEMO_MODE || process.env.NEXT_PUBLIC_ARGO_DEMO_MODE`.
2. Add a `console.warn` if only the old name is set.
3. Deprecate the old name in a follow-up.

**(c) Migrate every consumer to a unified namespace** (e.g. all `NEXT_PUBLIC_SKYCOT_*` per the rename memory). Too broad for this brief.

Recommendation: (a). Keeps the lib stable; just brings the launch config in line with what the code already does.

## Acceptance criteria

- [ ] Fresh `pnpm dev` from this repo's `.claude/launch.json` config: visiting `/team` against the live gateway renders the demo team (5 workers post-seed) without falling through to the empty-state CTA grid.
- [ ] `isDemoMode()` in the dev profile returns `true`.
- [ ] `apiGet` paths that gate on auth-bypass take the bypass branch.
- [ ] Add a top-of-file comment in `.claude/launch.json` listing the env-var names + which lib file reads them, so the next drift is obvious.

## May-touch

- `.claude/launch.json` (the two env-var renames)
- Possibly: `.env.local.example` if it ships the same wrong names
- Possibly: any `README.md` that documents the dev setup

## Out of scope

- The project rename (Argo → Skycot) — coordinated separately per the rename memory; only the two env-var names in launch.json need fixing here.
- Lib-side name unification (separate brief if desired)

## Effort

~5-20 LOC. ~10-20 min.
