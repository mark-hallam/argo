# claude-rebase-pr-1801 — recover #1801 claude-gateway-boot-split-routes-v2 (P0 unblocker)

**Owner**: claude. **Recovery target**: PR [#1801](https://github.com/mark-hallam/Argo-Agent/pull/1801). **Original brief**: `streams/claude-gateway-boot-split-routes-v2.md`.
**Authored against**: origin/main `de02c80a` on `2026-05-17`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1801`.
**Priority**: 🎯 **P0 unblocker**.

## Why this brief exists

PR #1801 was opened by a claude worker and verified MERGE-ADMIN-clean by the triage agent. It went DIRTY mid-admin-wave-5 (conflicts vs #1740 codex-skill-library landing in the same wave — both touched gateway routing paths). The boot-split unblocks 4 currently ceiling-refused claude briefs:

- `claude-jwt-cookie-security-hardening` (P1)
- `claude-error-page-design-system` (P2)
- `claude-pricing-tier-enforcement-runtime` (P0)
- `claude-support-inbox-spa-page` (P2)

All four touch `crates/argo-cli/src/handlers/gateway/boot.rs` (currently 497/500 LOC). The boot-split drops it to ~347 LOC and creates ~150 LOC of headroom for downstream mount-lines.

## Acceptance: pick ONE recovery path

### Path A — rebase (preferred)

1. `git fetch origin && git checkout claude-claude-gateway-boot-split-routes-v2`
2. `git rebase origin/main`. Likely conflicts:
   - `crates/argo-cli/src/handlers/gateway/boot.rs` (#1740 touched argo-skill mount paths; #1778 + #1794 may have touched route surfaces)
   - `crates/argo-cli/src/handlers/gateway/boot_router.rs` — new file created by #1801; should rebase clean
   - Possibly `mod.rs` if any new route handlers were registered in the wave
3. Verification per original brief: `cargo test -p argo-cli boot_`, `cargo clippy -p argo-cli --all-targets -- -D warnings`, `wc -l crates/argo-cli/src/handlers/gateway/boot.rs` (must be ≤350).
4. `git push --force-with-lease origin claude-claude-gateway-boot-split-routes-v2`.
5. Comment on #1801: "✅ rebased + force-pushed."

### Path B — fresh start (if rebase surface is too wide)

1. Re-read `streams/claude-gateway-boot-split-routes-v2.md`.
2. Run `wc -l crates/argo-cli/src/handlers/gateway/boot.rs` against current main to confirm the LOC pressure.
3. Implement on `claude-gateway-boot-split-routes-v3`.
4. Close #1801 with supersedes comment.

**Path A is strongly preferred.** This is pure mechanical extraction (verified by triage agent as such); rebasing should be tractable since the conflicts are in the same boot.rs file the brief is extracting from. Worker should accept "ours" / "theirs" per route-mount line as appropriate to preserve mount order.

## Acceptance criteria

- [ ] PR ends MERGEABLE / non-DIRTY against current main.
- [ ] `crates/argo-cli/src/handlers/gateway/boot.rs` is ≤350 LOC.
- [ ] `crates/argo-cli/src/handlers/gateway/boot_router.rs` exists and is ≤350 LOC.
- [ ] All existing tests pass: `cargo test -p argo-cli boot_`, `cargo test -p argo-cli gateway`.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] No behavior change (smoke: `argo gateway start` boots cleanly, `GET /healthz` returns 200).
- [ ] BUILD_STATUS.md row for `claude-gateway-boot-split-routes-v2` flipped TODO → IN_REVIEW with recovery PR number (or stays #1801 if Path A).

## Hidden constraints

- **Conflicts are likely on `.route()` mount lines** — preserve order. Order matters in axum for prefix matching.
- **boot_router::assemble_router(state) -> Router** — the contract from the original brief. Don't restructure the signature.
- **`pub(super)` visibility** — boot_router is a sibling helper, not public API.
- **Don't merge mount lines from other PRs into the wrong file.** Each new route mount added during admin-wave-5 belongs in `boot.rs::start()`'s call to `boot_router::assemble_router` — not in a stale boot.rs fragment.
- **Tests stay** — `boot_tests.rs` + `boot_bind_tests.rs` continue to pass without modification.

## Unblocks (post-merge)

4 claude briefs become claimable the moment this lands. Workers will pick them up within minutes.
