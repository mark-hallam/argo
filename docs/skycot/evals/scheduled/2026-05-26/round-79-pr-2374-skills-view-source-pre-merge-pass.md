# Round 79 — PR #2374 (skills View-source toast → link) PRE-MERGE PASS

**Date**: 2026-05-26
**Round**: 79
**Pillar/Claim**: SPA correctness — Claim 4 (skills shared + learned) UX surface
**Verdict**: 🟢 **PRE-MERGE PASS** — P2

PR: [#2374 fix(skills): replace View-source toast stub with /skills/<name> link](https://github.com/mark-hallam/Argo-Agent/pull/2374)
Head SHA: `8ed4f5209550e5425f0f584f521d18731e88bc58`
State: `MERGEABLE` / `CLEAN`, IN_REVIEW per ledger.
Diff scope: 4 files (PR-diff), +79/-50 — web-only, no Rust touched.

## Setup

```bash
# Create worktree off PR head
git fetch origin pull/2374/head:pr-2374-head
git worktree add /tmp/pr-2374-wt pr-2374-head    # checks out 8ed4f5209

# Symlink node_modules from main (PR doesn't touch package.json or lockfile)
git show pr-2374-head -- web/package.json web/pnpm-lock.yaml   # → empty (no PR changes)
ln -s ~/code/argo/web/node_modules /tmp/pr-2374-wt/web/node_modules

# Run touched test file on PR head
cd /tmp/pr-2374-wt/web
node_modules/.bin/vitest run components/skycot/worker/Skills.test.tsx
```

## Observed

```
RUN  v3.2.4 /private/tmp/pr-2374-wt/web
 ✓ components/skycot/worker/Skills.test.tsx (13 tests) 287ms
 Test Files  1 passed (1)
      Tests  13 passed (13)
   Duration  3.24s
```

All 13 tests pass on PR head SHA `8ed4f5209`. Note: ledger row 322 claimed "16 vitest tests pass" — the actual count on the touched file is 13. Off-by-3 narrative; not material to the verdict.

## Anti-theater check

The PR substitutes one test for another (toast-assertion → link-assertion) and the matching production component (toast UI → `<Link href="/skills/<name>">`). To rule out test-rewrites-to-green:

1. **Ran vitest on `origin/main` against the same test file** → 13 passed (13). On main, the test asserts toast announcement and the component renders the toast. They match → pass.
2. **On PR head**, the test asserts `tagName === "A"` and `href === "/skills/senior-eng-screening"`. If the production component still rendered a toast button, `tagName` would be `"BUTTON"` and `href` would be missing → test would fail.
3. **Verified the production component change is real**: `git show pr-2374-head:web/components/skycot/worker/SkillsActiveCard.tsx` contains:
   ```tsx
   <Link
     href={`/skills/${encodeURIComponent(skill.name)}`}
     data-testid={`skill-view-${skill.id}`}
     className={buttonVariants({ variant: "ghost", size: "sm" })}
   >
   ```
   So the test queries the actual DOM produced by the new code. The component change drives the test pass, not a hand-rigged mock.

4. **`/skills/<name>` is a real surface**, not a 404 — it's the canonical skill-detail page from `mission-skill-discovery-marketplace` (per brief). PR also drops the `onViewSource` prop wiring and the dead `announcement` state/toast/sr-only block — i.e. dead code is being actively removed, not left behind.

## Branch freshness note

`git diff main..pr-2374-head --stat` shows the PR is BEHIND main on several non-conflicting files (`docs/build-plan/BUILD_STATUS.md`, `streams/fix-audit-kind-register-hipaa-compliance-check.md` 79 deletions, `web/__tests__/auth/signup-page.test.tsx` 70 deletions, etc. — all changes that landed on main AFTER the PR branched). `mergeStateStatus: CLEAN` confirms no conflict; auto-merge will handle the lag. This is normal post-merge-train PR drift.

## Gap / follow-up brief candidates

None. PR is small, web-only, replaces a known stub with the canonical link, kills dead state, has a matching test. Falls under overnight `ops-*`-style auth in that it's an evaluator-PRE-MERGE-PASS clean PR that the conductor may admin-merge with `--skip-clippy` if the cargo workspace gates are taking too long (this PR touches zero Rust crates, so cargo gates should NO-OP anyway — full gate is fine).

**Recommendation for next conductor tick**: MERGE PR #2374. Standard admin path. Web-only, no schema, no audit, no runtime.
