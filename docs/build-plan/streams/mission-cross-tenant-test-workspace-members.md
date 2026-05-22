# mission-cross-tenant-test-workspace-members

**Owner**: claude.
**Depends on**: none.
**Authored against**: origin/main `917290b98` on `2026-05-22`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-cross-tenant-test-workspace-members`.

🟡 **P2 SECURITY HYGIENE.** Source: `mission-cross-tenant-audit-sweep` (this PR) — the workspace-members surface is the **top-3 data-exposure risk** in `docs/skycot/audit/cross-tenant-endpoint-coverage.md`. It's IAM-grade: GET lists current members + pending invitations, POST `/invite` mints a token, POST `/accept` claims one. A cross-tenant leak here is identity confusion — at minimum a B-caller seeing A's roster; at worst a B-caller accepting A's invite into A's tenant. The sibling test file `routes_workspace_members_tests.rs` exercises the invite + accept + list flow end-to-end but does NOT include a cross-tenant negative.

## Scope

Add three cross-tenant negative tests to `routes_workspace_members_tests.rs`:

1. **GET /api/v1/workspace/members tenant scoping.** Seed two tenants
   each with a member + a pending invite via `provision_response`.
   Tenant-A caller GETs members — list contains only A's member +
   A's pending invite; B's data is excluded.
2. **POST /api/v1/workspace/members/invite cross-tenant target.**
   Tenant B caller invites an email that's already pending in tenant
   A's `workspace_member_invitation` table. The B-invite must be
   created under tenant B (not pollute A's invite row), and the A
   row must remain untouched (no shared-state leak).
3. **POST /api/v1/workspace/members/accept token-tenant binding.**
   Tenant A creates an invite token. A caller authenticated as
   tenant B attempts to accept that token. The accept handler must
   bind the new membership to the **invite's tenant** (A), not the
   caller's existing tenant (B) — OR refuse the accept entirely
   (whichever the documented behavior is — check `accept_response`).
   The test pins the documented behavior so a future refactor cannot
   silently swap it.

Test 3 is the most security-sensitive — if the handler silently uses
the caller's tenant binding instead of the invite's, the accept flow
could be coerced into cross-tenant membership grants.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `crates/argo-cli/src/handlers/gateway/routes_workspace_members_tests.rs` — three new `#[tokio::test]` fns, additive only.
- `crates/argo-cli/src/handlers/gateway/routes_workspace_members.rs` — ONLY if test 1 surfaces a list-scoping bug AND the fix is ≤30 LOC (e.g. a missing tenant filter on `list_pending_for_tenant` callsite). Otherwise BLOCK + queue follow-up.
- `crates/argo-cli/src/handlers/gateway/routes_workspace_members_invite.rs` — same trivial-fix carve-out for test 2.
- `crates/argo-cli/src/handlers/gateway/routes_workspace_members_accept.rs` — same trivial-fix carve-out for test 3.

## Files you must NOT touch

- Any other handler under `crates/argo-cli/src/handlers/gateway/`.
- `argo-identity` (`user_store`, `user_tenant_store`).
- `argo-store/workspace_member_invitation.rs`.
- Migrations / schema.
- BUILD_STATUS.md outside the standard row flip.

## Acceptance criteria

- [ ] Three new `#[tokio::test]` fns in `routes_workspace_members_tests.rs`, named with the `cross_tenant_` prefix.
- [ ] Tests use tenant ids `"tenant-a"` / `"tenant-b"` (literal strings).
- [ ] Each test seeds via the existing `provision_response` helper — do not introduce a parallel tenant-seeding path.
- [ ] Test 1 asserts: list contains only the caller-tenant's member + pending invite (cross-counts AND cross-IDs both zero).
- [ ] Test 2 asserts: after B's invite, the A invite row at `lookup_pending_by_token_hash(...)` is bit-identical to its pre-state, AND a new invite row exists under B's tenant.
- [ ] Test 3 asserts the documented behavior — pin whichever the handler implements (404 / 403 / accept-but-rebind). If the handler accepts AND silently binds to caller's tenant, that's a security bug — the brief BLOCKs with a one-line finding.
- [ ] `cargo test -p argo-cli routes_workspace_members` passes.
- [ ] `cargo clippy -p argo-cli --all-targets -- -D warnings` clean.
- [ ] If any test surfaces a real cross-tenant bug, brief BLOCKs with one-line finding in BUILD_STATUS; PR body's "Findings" names the broken endpoint + missing check; no production code edited unless trivial-fix carve-out applies.
- [ ] Every modified file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-mission-cross-tenant-test-workspace-members`
- PR title: `test(gateway): cross-tenant negative for /api/v1/workspace/members* (mission-cross-tenant-test-workspace-members)`

## Notes for the implementer

- ~250-400 LOC expected (three test fns + seed helpers).
- Read `routes_workspace_members_tests.rs` lines 1-40 first — the
  test file pins `provision_for_user` as the canonical tenant-seed
  path. Each new test should call `provision_response` twice (once
  per tenant) before invoking any of the members endpoints.
- Test 3 is the security-critical one. Before writing it, READ
  `routes_workspace_members_accept.rs` (`accept_response` fn) and
  trace which tenant id the new `user_tenant` row is created under
  (the invite's vs the caller's). Pin whichever the code actually
  does; if it pins to the caller (a bug), BLOCK and the conductor
  will queue a fix.
- This is a TEST brief. Production-code edits are only allowed under
  the ≤30 LOC trivial-fix carve-out, and only if the fix is local to
  one of the three may-touch handler files. Anything bigger ships in
  its own follow-up brief.
