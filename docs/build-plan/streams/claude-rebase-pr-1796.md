# claude-rebase-pr-1796 — recovery rebase for magic-link signup/verify (CRITICAL v1.0 demo unblock)

**Owner**: claude. **Sizing hint**: rebase + conflict resolution; LOC-neutral.
**Depends on**: none.
**Authored against**: origin/main `b866e8f7` on `2026-05-18`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh claude-rebase-pr-1796`.
**Source**: PR #1796 (branch `claude-claude-rebase-pr-1756`) has been CONFLICTING
on `origin/main` since 2026-05-17 — multiple admin-merge waves have moved the
base out from under it. PR last update 2026-05-17T10:30:21Z (~28h stale). The
prior recovery brief `claude-rebase-pr-1756` already iterated once and produced
PR #1796; that iteration is now itself stale.

## Why this matters

**v1.0 SHIP GATE.** Tonight's demo recording cannot exercise scenes 2–8
(signup → verify → dashboard) without these magic-link signup/verify endpoints
in `routes_auth.rs`. The gateway has no signup/verify routes on `main` without
#1796 landing.

**Block multiplier**: this PR gates **5 BLOCKED rows** (claude / codex mix):
- `mission-magic-link-v1x-hardening`
- `claude-jwt-cookie-security-hardening`
- `codex-multi-tenant-isolation-bench`
- `mission-data-export-customer-flow`
- `mission-signup-magic-link-spam-protection`

All of them have BLOCK notes that explicitly cite "Pre-claim check: confirm
#1796 magic-link MERGED."

## Mission alignment

- **Reliability** (CLAUDE.md priority #1) — auth is the gateway's most
  load-bearing surface; merging a stale + conflict-laced version is worse
  than a clean rebase.
- **🎯 v1.0 SHIP** — operator decision 2026-05-17: tonight is the demo
  recording cut. Magic-link must be on `main` before recording.

## Scope

P0 SHIP-GATE. Rebase `claude-claude-rebase-pr-1756` onto current
`origin/main` and resolve conflicts. Do not expand scope. Do not refactor
neighbouring code.

### Conflict resolution discipline

Treat the existing PR diff (signup + verify route handlers + magic-link
issuance + DB rows for pending tokens) as load-bearing intent. Conflicts
typically arise from:

1. **Migration slot collisions** — if the PR adds a new migration, slot it
   AFTER current main's highest (slot 114 today). Check
   `crates/argo-store/migrations/` + `scripts/check-migration-slots.sh`
   guard for the current highest slot at rebase time.
2. **Gateway route table churn** — `routes_auth.rs` has been touched by
   #1818 (pricing-tier-enforcement) and others. Keep the magic-link
   routes; preserve the pricing-tier middleware ordering that landed.
3. **`AuditKind` enum expansion** — #1819 added `ApprovalAsked` /
   `ApprovalReplied`. If the magic-link branch added other variants,
   accept both — do not collapse.

### LOC budget

LOC-neutral relative to PR #1796 HEAD. The rebase resolves conflicts; it
does not add features. If conflict resolution surfaces a logic gap (e.g.
pricing-tier middleware needs to wrap the new routes), declare it as a
boundary deviation in the PR body rather than silently changing behaviour.

## Files you may touch

Whatever `git diff origin/main..claude-claude-rebase-pr-1756 --name-only`
shows after the rebase — typically:

- `crates/argo-cli/src/handlers/gateway/routes_auth.rs` (or `routes.rs` if not split).
- `crates/argo-cli/src/handlers/gateway/auth_magic_link.rs` (NEW) — token-issue, rate-limit.
- `crates/argo-cli/src/handlers/gateway/auth_magic_link_verify.rs` (NEW) — verify + session-mint.
- `crates/argo-store/migrations/<NEXT-SLOT>__magic_link.sql` (NEW).
- `crates/argo-store/src/magic_link_query.rs` (NEW).
- `crates/argo-cli/tests/magic_link_e2e.rs` (NEW or modified).

## Files you must NOT touch

- Pricing-tier enforcement landed in #1818 — preserve middleware order.
- `AuditKind` enum — only add new variants if the PR's original intent
  included them.
- `routes_auth.rs` >500 LOC ceiling — if rebase pushes it over, split
  routes_auth.rs OR carve magic-link into a sibling module (preferred).

## Acceptance criteria

- [ ] `gh pr view 1796 --json mergeable` returns MERGEABLE not CONFLICTING.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] `cargo test -p argo-cli` clean (magic-link tests pass).
- [ ] Smoke: `cargo run -p argo-cli -- gateway serve` boots; POST
      `/api/v1/auth/signup` returns 200 + token row inserted; POST
      `/api/v1/auth/verify` issues a session cookie.
- [ ] No new migration slot collisions (`bash scripts/check-migration-slots.sh`).
- [ ] BUILD_STATUS.md rows updated (this brief + #1796's row both → MERGED on
      landing).

## Branch + PR

- Branch name: REUSE `claude-claude-rebase-pr-1756` (force-push allowed on the
  feature branch — destructive operation OK *only* on this PR's branch, never
  main). If the worker prefers a fresh branch, `claude-claude-rebase-pr-1796`
  is also fine; close #1796 in favour of the new PR in that case.
- Title: `feat(gateway): magic-link signup + verify endpoints (claude-rebase-pr-1796)`
- Worker verification: full local verify on this one — gateway boot smoke +
  crate-scoped clippy + integration test. Magic-link is the demo-critical
  path; spend the extra 10 minutes.

## Hidden constraints

- **Demo recording is tonight.** Operator will admin-merge as soon as this
  PR is MERGEABLE + worker verification clean. Conductor will not Tier-2 stall.
- **DO NOT introduce new features** beyond what the original PR #1796 carried.
  Hardening (rate-limit, JWT-cookie hardening, signup-spam protection) are in
  separate BLOCKED briefs that re-claim once this lands. Keep the surface
  minimal — sign-up + verify + session-mint.
- **`scripts/check-migration-slots.sh` is the slot guard.** Run it before
  pushing.
- **Closes the v1.0 demo critical-path block.** When #1796 merges, 5 BLOCKED
  rows are immediately re-claimable.
