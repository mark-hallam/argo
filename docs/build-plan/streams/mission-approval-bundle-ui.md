# Mission load-bearing wire — Approval-bundle UI

**Owner**: claude. **Depends on**: E0 + E1-a + (sibling backend) — bundling needs a server endpoint to fetch grouped approvals.
**Authored against**: origin/main `1cca6a69` on `2026-05-12`.

> SOURCE: MISSION_BACKLOG.md Section 3 Claim 6 #128. The current approvals page shows individual approvals. For a worker doing a batch of similar low-risk actions (e.g. responding to 8 social-media mentions, all approved by the same policy), the operator wants a single "approve bundle" decision. E1-d shipped the similar-cards batch UI; this brief is the named-bundle variant — a server-defined bundle that workers explicitly request.

## Scope

NEW `/api/v1/approvals/bundles` endpoints + frontend UI:

1. **Backend** `GET /api/v1/approvals/bundles?status=pending` returns `[{ bundle_id, label, count, members: [approval_id], created_at }]`. NEW SQL: `approval_bundle` table + `approval_bundle_member` join table. Migration slot 71 per header pin.
2. **Backend** `POST /api/v1/approvals/bundles/<id>/decide` accepts `{ decision: approve | refuse }` and applies to every member transactionally.
3. **Frontend** new `BundleCard` component on `/approvals` route — distinct visual style (stacked card metaphor) above individual cards.
4. **Bundle-creation** is worker-driven: a worker fires `POST /api/v1/approvals/bundles` with member approval-ids when emitting a coherent group of actions. Workers know their own batch boundaries.

This is NOT in scope: cross-worker bundling (one worker = one bundle); auto-bundling by similarity (E1-d already covers that); bundle expiry (members expire individually via the timeout brief); per-member partial decisions (atomic-only — bundle = all or none).

## Files you may touch

- `crates/argo-store/migrations/71__approval_bundle.sql` (NEW, ≤ 50 LOC)
- `crates/argo-store/src/approval_bundle.rs` (NEW, ≤ 300 LOC) + tests
- `crates/argo-cli/src/handlers/gateway/routes_approval_bundles.rs` (NEW, ≤ 380 LOC) + tests
- `web/components/skycot/BundleCard.tsx` (NEW, ≤ 200 LOC) + tests
- `web/lib/hooks/use-approval-bundles.ts` (NEW, ≤ 200 LOC)
- `web/app/(app)/page.tsx` (≤ 490 LOC after — close to ceiling; narrowly scoped)
- `docs/skycot/RUNBOOK.md` — small note

## Files you must NOT touch

- Existing per-approval CRUD (orthogonal); the E1-d batch-card grouping (different model — that's client-side similarity; this is server-defined bundles)

## Acceptance criteria

- [ ] Migration 71 applies cleanly
- [ ] Bundle CRUD endpoints + transactional decide
- [ ] BundleCard renders distinctly from individual ApprovalCard
- [ ] Worker can post a bundle; SPA picks it up via TanStack invalidation
- [ ] 8 unit tests (4 backend + 4 frontend); `pnpm test` + `cargo clippy` clean
- [ ] page.tsx stays under 500 LOC; BUILD_STATUS.md updated

## Branch + PR

- Branch: `claude-mission-approval-bundle-ui`
- PR title: `approvals: server-defined bundle UI (mission-approval-bundle-ui)`

## Notes

- Bundles are atomic. If 7 of 8 members are still pending, approving the bundle approves all 7 + leaves the 8th as a member-of-resolved-bundle (already-decided). Document this corner case.
- The visual treatment (stacked-card metaphor) matters — operators MUST see that this is a bundle of N, not 1 of N. Use shadows/offsets to convey the metaphor.
