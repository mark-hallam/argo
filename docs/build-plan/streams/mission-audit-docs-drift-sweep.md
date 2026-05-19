# Stream Audit-P2-5 — source-of-truth docs drift sweep

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `d2e4a579` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-audit-docs-drift-sweep`.
**Source**: 2026-05-19 codex audit P2-9, validated by claude `VALIDATION.md` (PARTIALLY CONFIRMED — codex cited paths under `docs/skycot/` that don't exist for some files; the real paths are mixed across `docs/skycot/` and `docs/build-plan/`. Drift IS real but the locations need correcting).

## Scope

Two specific drift items confirmed:

1. **`docs/build-plan/RESUME.md` contradicts itself.** Line 12 says "Codex workers active again since 2026-05-14 (the earlier 'Codex offline' note from 2026-05-09 is stale). Both `claude` and `codex` workers are running." Line 100 (Q&A section) still says "Is codex active? → no, offline 2026-05-09. Skip codex side." These contradict each other directly.

2. **`docs/skycot/V1_DEFINITION.md` blockers section is stale.** Lines 163-164 list `claude-skill-library-worker-load-wire` and `claude-migration-slot-dedup-and-validator` as unchecked `[ ]` ship blockers. Both are SHIPPED per project memory `project_v1_strategic_decisions_2026_05_14.md` (skill-library worker-load merged via #1682; migration-slot guard merged post #1735). The boxes should be checked + a "shipped via #PR" annotation added.

Plus a third lightweight ask:

3. **Add a doc-drift sniff to the conductor's tick.** Today the conductor runs `scripts/find-stale-claim-drift.py`; add a sibling `scripts/find-stale-doc-drift.py` that flags known-stale claims (ship-blockers marked unchecked but referenced as shipped in a memory file, contradicting RESUME.md sections, etc.). Optional; if scope creeps, defer to a follow-up brief.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1

## Files you may touch

- `docs/build-plan/RESUME.md` — at line 100 (the Q&A section), update the codex answer to match line 12's statement: "Is codex active? → yes, came back online 2026-05-14. Use the codex queue and pool normally." Resolve the contradiction.
- `docs/skycot/V1_DEFINITION.md` — at lines 163-164, flip `[ ]` → `[x]` for `claude-skill-library-worker-load-wire` (annotate "shipped via #1682, see project memory") and `claude-migration-slot-dedup-and-validator` (annotate "shipped post #1735"). Add the merge-PR refs as inline comments. If the file has other items in this state, the implementer should sweep them by cross-referencing against `~/.claude/projects/-Users-operator-code-argo/memory/project_v1_strategic_decisions_2026_05_14.md` and `BUILD_STATUS.md`.
- `scripts/find-stale-doc-drift.py` (NEW, optional) — small Python script: reads a hand-maintained "known-shipped" list and grep-checks `docs/skycot/V1_DEFINITION.md` + `docs/build-plan/RESUME.md` for stale references. Output is a punch-list; exit code is 1 if drift found (so conductor can surface it). If skipping this for scope, document the deferral.
- `docs/build-plan/RESUME.md` — at top, add a "Last sweep: <date>" line. The conductor checks this on every tick and warns if it's > 14 days old. (Optional but recommended.)

## Files you must NOT touch

- `BUILD_STATUS.md` — that's the live ledger, owned by the conductor only.
- Per-PR commit messages or git history — drift fix is in the docs, not the history.
- Investigation reports under `argo-investigations/` — those are point-in-time snapshots, fine to be stale.
- The PROTOCOL_LOOP_PROMPT.md — the conductor's prompt is unchanged; the new sniff (if shipped) wraps existing behavior.

## Acceptance criteria

- [ ] `docs/build-plan/RESUME.md:100` no longer contradicts line 12.
- [ ] `docs/skycot/V1_DEFINITION.md` lines 163-164 (and any sibling stale rows the implementer finds) are flipped to checked with PR annotations.
- [ ] If `find-stale-doc-drift.py` is included: passes locally + flags zero drift after the sweep + a CI-style smoke test confirms it would catch the original drift if the changes were reverted.
- [ ] `Last sweep: <date>` line added to RESUME.md top (optional).
- [ ] No code changes — this is docs-only.
- [ ] BUILD_STATUS.md updated with PR number (one row).

## Branch + PR

- Branch: `claude-mission-audit-docs-drift-sweep`
- PR title: `docs: RESUME.md + V1_DEFINITION.md drift sweep (mission-audit-docs-drift-sweep)`

## Notes for the implementer

- The codex audit cited some doc paths under `docs/skycot/` that don't exist (it referred to `docs/skycot/RESUME.md`, `docs/skycot/BUILD_PLAN_V11.md`, `docs/skycot/PRODUCT_ROADMAP.md`). The REAL paths are `docs/build-plan/RESUME.md` + `docs/build-plan/BUILD_PLAN_V11.md` + `docs/build-plan/PRODUCT_ROADMAP.md`. `V1_DEFINITION.md` is the exception — that one IS at `docs/skycot/V1_DEFINITION.md`. Sweep against the actual paths.
- When in doubt whether something is "shipped," check `docs/build-plan/BUILD_STATUS.md` for the merge row + commit SHA. Anything with a merge row is shipped.
- Don't sweep PRODUCT_ROADMAP.md or BUILD_PLAN_V11.md unless the implementer finds specific drift — those are larger files and not in scope for this brief.
- The `Last sweep:` line is a forcing function for periodic drift reviews. Set the date to today's date when you commit.

## Out of scope (follow-up briefs)

- A full conductor-driven doc-drift check on every tick — premature; the sniff script is enough for now.
- Auto-generating V1_DEFINITION.md status from `BUILD_STATUS.md` — that would require a doc-template system; not worth it for one file.
- Updating prior audit reports in `docs/skycot/audit/` to mark closed findings — those are immutable point-in-time records; let them be.
- Cross-referencing memory files with V1_DEFINITION.md automatically — would need a memory-file index, separate brief.
