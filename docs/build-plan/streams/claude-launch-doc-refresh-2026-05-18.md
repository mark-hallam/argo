# claude-launch-doc-refresh-2026-05-18 — README + CLAUDE.md + workflows-policy refresh

**Owner**: claude. **Sizing hint**: fast-mode candidate — doc-only edits.
**Depends on**: none.
**Authored against**: origin/main `bc3841a4` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-launch-doc-refresh-2026-05-18`.
**Source**: 2026-05-18 audit P2 findings #13 + #14 + #15 + #16 — small doc-drift items to clean before launch.

## Mission alignment

- **Repeatability** (CLAUDE.md priority #3) — docs match current state, not aspirational state.
- **Commercial usability** (#5) — first-impression docs (README) must reflect current snapshot.

## Scope

P2 fast-mode. Three doc edits + one brief closure.

### Edit 1: README snapshot date + PR count

`README.md` line 7 currently says snapshot `2026-05-17 / ~1000 PRs`. Bump to today's date + current count.

Today: 2026-05-18. PR count: run `gh pr list --state merged --json number | jq length` (or count via `git log --merges --oneline | wc -l` for an approximation). Expect ~1010-1020.

### Edit 2: CLAUDE.md status line

CLAUDE.md L122 still flags `#1796 magic-link rebase (P0 SHIP-GATE for demo recording)`. This is stale — #1823 (recovery PR) landed and includes the magic-link surface. Update L120-122 to:

```
**Status 2026-05-18:** **Magic-link signup+verify LIVE on main via #1823.**
Mission claim 4 PROVEN via #1822 (skill-loop closure smoke). All 6 audit-derived
briefs from 2026-05-17 closed. CI gates GREEN locally (parallel-pub-types-enforce
+ loc-budget + prepared-statements + unsafe-safety + audit-sink-tenant-guard).
All 11 disabled GHA workflows intentionally kept disabled per
`.github/workflows/DISABLED_WORKFLOWS_POLICY.md` (v1 strategy: admin-merge bypass
while GHA billing is operator-blocked). Codex pool at full Pro 20x. Remaining
for launch: SQLCipher 4-PR sequence (v1.0 scope per operator 2026-05-18) + tonight's
demo recording.
```

### Edit 3: DISABLED_WORKFLOWS_POLICY entry for load-soak.yml.disabled

`.github/workflows/DISABLED_WORKFLOWS_POLICY.md` is missing an entry for `load-soak.yml.disabled` (added by PR #1811 `codex-gateway-load-soak-baseline`). Append a row to the inventory:

| Workflow | Status | Rationale |
|---|---|---|
| `load-soak.yml.disabled` | keep-disabled-v1 | Operator-triggered via `workflow_dispatch`; expensive; not needed for routine merge gating |

### Edit 4: Close `claude-rebase-pr-1796.md` brief as STALE-FIXED

The brief's intent (recovery rebase of #1796) was delivered by #1823. Flip its ledger row to MERGED-VIA-RECOVERY (already done last tick). Add a `STALE-FIXED` note to the brief file's header so future reads know not to re-claim:

```markdown
> **STALE-FIXED 2026-05-18**: intent delivered by #1823 (which merged the
> recovery work). This brief file remains for traceability; do not re-claim.
```

LOC: ~80 net.

## Files you may touch

- `README.md` — ≤2 modified LOC.
- `CLAUDE.md` — ≤25 modified LOC.
- `.github/workflows/DISABLED_WORKFLOWS_POLICY.md` — ≤5 modified LOC.
- `docs/build-plan/streams/claude-rebase-pr-1796.md` — add STALE-FIXED header. ≤5 modified LOC.

## Files you must NOT touch

- Any source code.
- Other briefs.
- Audit reports.

## Acceptance criteria

- [ ] README snapshot reads `2026-05-18` (or current date at PR open) with accurate PR count.
- [ ] CLAUDE.md L120-122 reflects current launch-status (magic-link landed, SQLCipher in progress).
- [ ] `DISABLED_WORKFLOWS_POLICY.md` lists `load-soak.yml.disabled`.
- [ ] `claude-rebase-pr-1796.md` carries STALE-FIXED header.
- [ ] BUILD_STATUS.md row updated.

## Branch + PR

- Branch: `claude-launch-doc-refresh-2026-05-18`
- Title: `docs: refresh README/CLAUDE.md/workflows-policy for 2026-05-18 launch state (claude-launch-doc-refresh-2026-05-18)`
- Worker verification: doc-only diff; no clippy/tests needed.

## Hidden constraints

- **Numbers in README must be accurate at PR-open time.** If days pass and another tick adds 20 PRs, re-run the count before push.
- **CLAUDE.md is operator-amendable.** If the operator has touched L120-122 between brief-author and worker-claim, preserve operator edits and bolt on the new status.
- **Closes 2026-05-18 audit P2 #13, #14, #15, #16.**
