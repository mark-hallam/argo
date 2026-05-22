# mission-web-dashboard-mvp-rebase — rebase PR #1092 onto current main, force-push to land MVP-priority web dashboard

**Owner**: claude. **Depends on**: nothing. **Tier M (MVP-priority rebase) — operator escalation 2026-05-10.**

**Authored against**: origin/main b21078ad on 2026-05-10.

## Why

PR #1092 (claude-claude-web-dashboard-mvp branch) is **MVP-day priority** per operator comment 2026-05-09T23:29Z. Substance is sound: 1133 LOC, 14 files, all in argo-cli, 14 unit + 2 integration tests, server-rendered HTMX dashboard at `localhost:7090/dashboard/`. The only blocker is a rebase conflict on `crates/argo-cli/src/handlers/gateway/routes.rs` after #1124 (multi-host-orchestrator) and the routes-msgs/post-writes splits landed.

Operator's priority bump has had ~6h with no worker pickup because the row was buried in long-tail. This brief is a tightly-scoped rebase task at Tier M urgency so the next worker tab grabs it from the top.

## Approach (concrete)

1. **Fetch existing branch:**
   ```bash
   git fetch origin claude-claude-web-dashboard-mvp
   git worktree add /tmp/web-dashboard-mvp-rebase claude-claude-web-dashboard-mvp
   cd /tmp/web-dashboard-mvp-rebase
   ```

2. **Rebase onto current main:**
   ```bash
   git fetch origin main
   git rebase origin/main
   ```

3. **Resolve conflict in `crates/argo-cli/src/handlers/gateway/routes.rs`:**
   - The conflict is in the route-registration block. Main has new route registrations from #1124 (multi-host-orchestrator) + routes-msgs split.
   - Branch has `routes_dashboard::register(...)` call that needs to be added alongside the new main routes.
   - **Resolution rule:** keep ALL route registrations from both sides, in the order they appear in main with `routes_dashboard::register` inserted at a sensible spot (probably alphabetically near other meta/admin routes, or at the end of the registration block — match local convention).

4. **Verify post-rebase:**
   ```bash
   cargo clippy -p argo-cli --all-targets -- -D warnings
   cargo test -p argo-cli --test dashboard_smoke
   cargo test -p argo-cli  # full crate, expect 8 KNOWN_FLAKES boundary
   ```

5. **Force-push:**
   ```bash
   git push --force-with-lease origin claude-claude-web-dashboard-mvp
   ```

6. **Update ledger row** for `claude-web-dashboard-mvp` from TODO → IN_REVIEW with note "rebased onto main YYYY-MM-DD; conflict in routes.rs resolved by keeping both sides; clippy + tests clean."

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes.rs` (conflict resolution only — no other changes)
- `docs/build-plan/BUILD_STATUS.md` (status flip for `claude-web-dashboard-mvp`)

## Files you must NOT touch

- Any other source file. The PR's substance is already complete and Tier 2 verified pre-conflict; do not re-implement or alter.

## Acceptance criteria

- [ ] `git rebase origin/main` succeeds on `claude-claude-web-dashboard-mvp` branch
- [ ] `crates/argo-cli/src/handlers/gateway/routes.rs` cleanly merged (all original main routes + dashboard registration)
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean
- [ ] `cargo test -p argo-cli --test dashboard_smoke` passes
- [ ] `git push --force-with-lease` successful
- [ ] PR #1092 reports `mergeable=MERGEABLE` after the push
- [ ] BUILD_STATUS.md row flipped TODO → IN_REVIEW with brief note

## Branch + PR

- **Existing branch:** `claude-claude-web-dashboard-mvp` (rebase + force-push, do NOT create new branch or new PR)
- **Existing PR:** #1092 (will become MERGEABLE after force-push; no new PR needed)
- **Conductor will merge** on next tick after this brief lands as IN_REVIEW.
