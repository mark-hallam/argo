# fix-cohorts-memory-drift-project-ops-tab-position

**Owner**: claude.
**Authored against**: origin/main `78f162a5` on `2026-05-26`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-cohorts-memory-drift-project-ops-tab-position`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-26-2152/whole-site-mvp-claims-coverage.md` F12 [P2].

🟡 **P2 — Memory `project_pm_cohorts_peta_patrick` says "Project & Ops as 7th dept tab" but the dept-registry at `web/components/skycot/cohorts/dept-registry.ts:18-50` orders the tabs as: Support (1) → Finance & Ops (2) → Sales (3) → Project & Ops (4) → People & HR (5) → IT & Dev-ex (6) → Marketing (7). Memory drift. Future briefs reading the memory may insert duplicate tabs OR misposition them.**

## Root cause

Persistent global memory `project_pm_cohorts_peta_patrick` records the tab as 7th. Live registry has it at index 3 (4th position, 1-indexed). This is a memory bug, not a code bug. The audit flags it because the memory is consulted when authoring future Peta/Patrick-related briefs — drift between memory and registry can cause briefs to assume a position that doesn't exist.

This brief is doc-only: update the memory file. The registry is the source of truth and remains canonical.

## Findings this brief closes

- whole-site-mvp-claims-coverage F12 [P2] — Memory `project_pm_cohorts_peta_patrick` says "Project & Ops as 7th dept tab" but registry has it at index 3

## Fix recipe

1. **Locate the memory file** at `~/.claude/projects/-Users-operator-code-argo/memory/project_pm_cohorts_peta_patrick.md` (or wherever the project memory is stored — typically under `~/.claude/projects/<repo>/memory/`).

2. **Update the relevant paragraph** to reflect the registry's actual order:
   - Before: "...with Project & Ops as 7th dept tab..."
   - After: "...with Project & Ops as the 4th dept tab (between Sales and People & HR). Registry: `web/components/skycot/cohorts/dept-registry.ts:18-50`. Tab order: Support → Finance & Ops → Sales → Project & Ops → People & HR → IT & Dev-ex → Marketing."

3. **Cross-check any sibling briefs / docs** that quote the "7th tab" position:
   - `grep -rn "Project & Ops as 7th\|7th dept tab" docs/` — fix references.
   - `grep -rn "Project & Ops as 7th\|7th dept tab" docs/build-plan/streams/` — fix any brief that quotes this.

4. **No code change required.** The registry is correct.

5. **Add a follow-up sweep** — if any other persona memories reference dept-registry positions, verify they match the source.

## Acceptance criteria

- [ ] Memory `project_pm_cohorts_peta_patrick` references Project & Ops as the 4th dept tab (between Sales and People & HR).
- [ ] No other doc / brief references the wrong position.
- [ ] Registry remains the source of truth.

## May-touch

- `~/.claude/projects/-Users-operator-code-argo/memory/project_pm_cohorts_peta_patrick.md` (the memory file)
- `~/.claude/projects/-Users-operator-code-argo/memory/MEMORY.md` (the memory index, if it references the position)
- Any brief under `docs/build-plan/streams/` that quotes the 7th-tab position.

## Out of scope

- Cohorts page implementation (covered by `fix-cohorts-empty-state-render-locked-d3-magazine-preview`).
- Cohorts dept registry source order (deliberate; not changing).
- Peta personality data backfill (covered by `fix-peta-personality-trait-level-lookup-coverage`).

## Effort

~5-10 LOC (text edit). ~30 minutes. P2 (doc-hygiene only).
