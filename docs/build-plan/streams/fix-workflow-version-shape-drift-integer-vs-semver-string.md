# fix-workflow-version-shape-drift-integer-vs-semver-string

**Owner:** claude
**Authored against:** origin/main `2a9911c6828c4b37e9313b32dd024457e31d15b8` on 2026-05-28
**Validate before claim:** `bash scripts/check-brief-freshness.sh fix-workflow-version-shape-drift-integer-vs-semver-string`
**Source:** UX e2e audit 2026-05-28-0252 (scene-4-and-5 F12)

🟠 **P1 — `/workflows` card formats version as `V3` (capitalised, integer-shaped) from `wf.versions: integer`. `/workflows/<v>/<n>` detail viewer renders `version 1.0.0`-style strings from `workflow.version: string`. Two surfaces show the same workflow with incompatible version shapes. Minor unless the camera lingers, but the inconsistency adds confusion during Scene 5's drill-in.**

## Root cause

The version field has two incompatible shapes flowing through the SPA:

- **Summary envelope** (`web/lib/api/workflows.ts` → `WorkflowSummary`): `version: number` (integer like `3`).
- **Spec envelope** (same file → `WorkflowSpec`): `version: string` (semver-shaped like `"1.0.0"`).

The card formatter at `web/app/(app)/workflows/page.tsx:259-262` does `v{wf.versions}` (note typo: `versions` plural, but in practice referring to the integer count) → renders `V3`. The detail viewer at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:225` does `version {workflow.version}` → renders `version 1.0.0`. The gateway returns `version: 3` on the list endpoint and a different version string on the spec endpoint.

Even if both endpoints carried the same underlying data, the SPA-side display would render them in incompatible casing/format. The locked B3-Etherscan / D3-magazine register doesn't pin a version format — but consistency across the same workflow card → detail flow is load-bearing for the camera.

## Findings this brief closes

- scene-4-and-5 F12 [P1] — Workflow version shape drift between card (V3) and detail viewer (1.0.0)

## Fix recipe

1. **Pick a canonical format**: `v3` lower-case for the integer-shaped count, with the detail viewer also rendering `v<N>` when the underlying spec carries an integer, or `v<semver>` when the underlying spec carries a semver string. Recommendation: render `v3` and drop the semver shape entirely for v1.0 — workflow versions are integer-bumped on each save, not semver-versioned.
2. **Normalize the spec envelope**: in `web/lib/api/workflows.ts` extend the deserializer to coerce `WorkflowSpec.version` from string-or-number to a canonical `number`. If the gateway returns `"1.0.0"`, parse the major version and fall back to the integer.
3. **Add a `formatVersion(v: number | string): string` helper** at `web/lib/format/version.ts` (NEW) that takes either shape and returns `v3` (lowercase + integer). Use it on both the card and the detail viewer.
4. **Card formatter fix**: at `web/app/(app)/workflows/page.tsx:259-262`, replace the inline `V{wf.versions}` with `<span>{formatVersion(wf.versions)}</span>`.
5. **Detail viewer formatter fix**: at `web/app/(app)/workflows/[verticalId]/[name]/page.tsx:225`, replace `version {workflow.version}` with `{formatVersion(workflow.version)}` — same helper, same output.
6. **Coordinate with the gateway** if the spec endpoint genuinely emits semver strings that mean something (e.g. major.minor.patch with semantics). If so, the helper widens to handle both forms but normalises display. The audit suggests the gateway returns `version: 3` on summary — verify what the spec endpoint actually returns before the helper is written.
7. **Vitest** at `web/lib/format/version.test.ts`:
   - `formatVersion(3)` → `"v3"`
   - `formatVersion("1.0.0")` → `"v1"` (major-version reduction)
   - `formatVersion("3")` → `"v3"` (string-int coerce)
   - `formatVersion(0)` → `"v0"` (edge case for unauthored workflows)

## Acceptance criteria

- [ ] Both the workflow card and the detail viewer render the same version format for the same workflow.
- [ ] `formatVersion` helper lives at `web/lib/format/version.ts` with full test coverage.
- [ ] Card no longer renders `V3` (capitalised); detail viewer no longer renders `version 1.0.0`.
- [ ] No regression on the existing card test or detail viewer test.
- [ ] PR body cites scene-4-and-5 F12.

## May-touch

- `web/lib/format/version.ts` (NEW, ~20 LOC)
- `web/lib/format/version.test.ts` (NEW, ~40 LOC)
- `web/app/(app)/workflows/page.tsx` (card formatter, ~3 LOC)
- `web/app/(app)/workflows/[verticalId]/[name]/page.tsx` (detail viewer, ~3 LOC)
- `web/lib/api/workflows.ts` (only if deserializer normalisation needed, ~10 LOC)

## Out of scope

- Workflow versioning semantics (integer vs semver) at the gateway layer — operator decision, out of scope here
- Author-chip canon (covered by sibling `fix-workflow-card-author-chip-mariah-over-indexed` BLOCKED + `fix-sample-workflows-author-chip-rebalance` IN_REVIEW)
- Substrate chip on workflow cards (covered by `fix-workflow-substrate-chip-show-macos-browser-e2b` TODO)

## Effort

XS/S — ~60 LOC total + tests. ~1-2 hours.
