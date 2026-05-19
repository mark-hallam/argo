# claude-rebase-pr-1832 — recover #1832 mission-signup-magic-link-spam-protection

**Owner**: claude. **Recovery target**: PR [#1832](https://github.com/mark-hallam/Argo-Agent/pull/1832). **Original brief**: `streams/mission-signup-magic-link-spam-protection.md`.
**Authored against**: origin/main `da578d55` on `2026-05-19`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1832`.
**Priority**: P1 — magic-link signup spam protection is a v1 enterprise procurement gate.

## Why this brief exists

PR #1832 was opened 2026-05-17 by a codex worker. Adds honeypot + 5/hr IP rate limit + 60s email cooldown + IPv6 /64 normalisation + optional Cloudflare Turnstile + `[auth.signup_protection]` config section. Verified clean at open time: tests pass for the spam-protection module + argo-config; clippy clean on touched crates.

`gh api repos/.../pulls/1832` initially reported `mergeable=true` / `state=unstable` (acceptable while GHA-billing-blocked policy applies). Conductor attempted admin-merge in tick-2 (2026-05-19) AFTER #1841 and #1836 landed in the same tick; the merge failed with `GraphQL: Pull Request has merge conflicts (mergePullRequest)`. The two earlier merges in the wave introduced conflicts in the gateway / signup-route surface this PR touches.

This brief drives the rebase. Magic-link signup spam protection is required for v1 enterprise procurement (per the strategic-priority header) — should not stay DIRTY.

## Acceptance: pick ONE recovery path

### Path A — rebase (preferred)

1. `git fetch origin && git checkout mission-signup-magic-link-spam-protection`
2. `git rebase origin/main`. Expected conflicts (post-#1841 + #1836):
   - `crates/argo-cli/src/handlers/gateway/auth_magic_link.rs` — #1841 added `AuthSucceeded`/`AuthFailed` audit emission to the verify path; this PR's signup-protection wraps the signup path; both files may touch the same handler module.
   - `crates/argo-cli/src/handlers/gateway/auth_magic_link_tests.rs` — #1841 added 14 audit-emission tests; this PR adds spam-protection tests; merge both test suites.
   - Possibly `crates/argo-config/src/sections.rs` if `[auth.signup_protection]` lands alongside other recent config additions.
3. Resolution strategy: this PR's signup-side additions and #1841's verify-side additions are complementary — preserve both. Tests from both PRs should land side-by-side; if either file approaches 500 LOC, split the test file before pushing.
4. Verification per original brief: `cargo clippy -p argo-cli -p argo-config --all-targets -- -D warnings` clean; `cargo test -p argo-cli --bin argo auth_magic_link signup` pass; `cargo test -p argo-config` pass; `bash scripts/workspace-clippy-fast-fail.sh` clean.
5. `git push --force-with-lease origin mission-signup-magic-link-spam-protection`.
6. Comment on #1832: "✅ rebased + force-pushed onto origin/main `da578d55`. Merged post-#1841 audit emission + post-#1836 multi-tenant isolation bench."

### Path B — fresh start

If Path A surfaces test-file LOC ceiling conflicts that require restructuring (>2 files newly cross 500 LOC), close #1832 SUPERSEDED and open `mission-signup-magic-link-spam-protection-v2` against current main. Document the LOC-driven structural change in the PR body.

## Acceptance criteria

- [ ] PR #1832 ends MERGEABLE / non-DIRTY against current main.
- [ ] Signup rate limit + honeypot + email cooldown + IPv6 /64 + Turnstile (optional) work as in original PR.
- [ ] `[auth.signup_protection]` config section parses.
- [ ] `cargo clippy -p argo-cli -p argo-config --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli --bin argo auth_magic_link signup` pass.
- [ ] `bash scripts/workspace-clippy-fast-fail.sh` clean.
- [ ] No regressions in #1841 audit tests or #1836 multi-tenant isolation tests (both already MERGED on main).

## Notes for the conductor (lesson from tick-2)

This recovery exists because tick-2 attempted to admin-merge 3 PRs in a wave. The first two (#1841, #1836) landed cleanly. The third (#1832) hit a conflict that didn't exist at the start of the wave — GitHub recomputes mergeability after each upstream change. **Future ticks: re-check mergeable state PER PR via `gh api repos/.../pulls/<n>` (not the gh pr view batch — that returns UNKNOWN). If state shifts to dirty mid-wave, stop merging and author a recovery brief instead of proceeding.**
