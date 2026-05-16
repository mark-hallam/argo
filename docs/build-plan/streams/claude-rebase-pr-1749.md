# claude-rebase-pr-1749 — recover #1749 codex-code-executor-sandbox-failclose (F-12 P2)

**Owner**: claude. **Recovery target**: PR [#1749](https://github.com/mark-hallam/Argo-Agent/pull/1749). **Original brief**: `streams/codex-code-executor-sandbox-failclose.md`.
**Authored against**: origin/main `da8e0e0e` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1749`.

## Why this brief exists

PR #1749 (F-12 P2 — fail-closed Local code-executor + LocalCodeExecution capability) opened 2026-05-16T07:57 and stayed DIRTY ≥30h.

## Acceptance: pick ONE recovery path

### Path A — rebase

1. `git fetch origin && git checkout codex-code-executor-sandbox-failclose`
2. `git rebase origin/main` — likely conflicts in argo-cli (sandbox handler) + argo-capability (enum was canonicalized in #1747 type-canon wave; check for renames) + argo-types + argo-config.
3. Verification: crate-scoped clippy + tests + workspace clippy per original brief.
4. `git push --force-with-lease origin codex-code-executor-sandbox-failclose`.
5. Comment on #1749: "✅ rebased."

### Path B — fresh start

1. Re-read `streams/codex-code-executor-sandbox-failclose.md`.
2. **Critical**: re-check `Capability` enum in argo-types post-#1747 — `LocalCodeExecution` variant may have been added/renamed during the type-canon wave. Brief premise may need adjustment.
3. Implement on `codex-code-executor-sandbox-failclose-v2`. Close #1749.

**Pick Path A if** the type-canon wave didn't rename anything F-12 touches.

## Acceptance criteria

- [ ] PR ends MERGEABLE / non-DIRTY.
- [ ] Local backend fail-closed in production: missing config / no LocalCodeExecution capability → error, not silent allow.
- [ ] Worker verification clean.
- [ ] BUILD_STATUS.md row flipped TODO → IN_REVIEW with recovery PR number.

## Hidden constraints

- **`Capability::LocalCodeExecution` is the new variant the brief adds.** If #1747 type-canon added something similar, dedupe.
- **Brief premise:** Local executor backend in production must fail closed unless tenant has explicit `Capability::LocalCodeExecution` grant. F-12 P2 closes this.
- **Original brief LOC: ~230.** Recovery similar.
