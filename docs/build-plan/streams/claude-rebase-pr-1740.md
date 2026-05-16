# claude-rebase-pr-1740 — recover #1740 codex-skill-library-prompt-selector (F-11 P2)

**Owner**: claude. **Recovery target**: PR [#1740](https://github.com/mark-hallam/Argo-Agent/pull/1740). **Original brief**: `streams/codex-skill-library-prompt-selector.md`.
**Authored against**: origin/main `da8e0e0e` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1740`.

## Why this brief exists

PR #1740 (F-11 P2 audit finding — SkillRelevanceSelector) opened 2026-05-16T06:21 and stayed DIRTY ≥30h. Recovery brief per protocol stale sweep.

## Acceptance: pick ONE recovery path

### Path A — rebase

1. `git fetch origin && git checkout codex-skill-library-prompt-selector`
2. `git rebase origin/main` — likely conflicts in argo-skill + argo-agent-runner (turn_helpers.rs shared with several admin-wave merges).
3. Verification per original brief: `cargo test -p argo-skill -p argo-agent-runner`, `cargo clippy --all-targets -- -D warnings`.
4. `git push --force-with-lease origin codex-skill-library-prompt-selector`.
5. Comment on #1740: "✅ rebased."

### Path B — fresh start

1. Re-read original brief `streams/codex-skill-library-prompt-selector.md`.
2. Validate intent still load-bearing (it is — F-11 closes "provenance and trust" half of Claim 4).
3. Implement on `codex-skill-library-prompt-selector-v2`.
4. Close #1740 with supersedes comment.

**Pick Path A if** rebase resolves in <30 min. **Path B if** conflict surface is too wide.

## Acceptance criteria

- [ ] PR ends MERGEABLE / non-DIRTY.
- [ ] Worker-side verification clean.
- [ ] BUILD_STATUS.md row `codex-skill-library-prompt-selector` flipped TODO → IN_REVIEW with recovery PR number.
- [ ] If Path B: original PR closed.

## Hidden constraints

- **Original brief size: 280 LOC.** Recovery should be roughly similar (rebase preserves it; reimplementation matches).
- **Brief premise unchanged:** SkillRelevanceSelector with 4-axis filter (action-type, vertical, trust, tags) + `[skill_library.relevance]` config.
- **F-11 P2 is closed when this lands.** Tier-S follow-up impact.
