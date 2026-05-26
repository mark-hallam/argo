# fix-audit-isolation-shape-docs-list-vs-detail

**Owner**: claude.
**Authored against**: origin/main `4585055d32eba8a24f5872354c0225246c0aa250` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-isolation-shape-docs-list-vs-detail`.
**Source**: e2e-ux-audit `docs/skycot/ux-audits/2026-05-27-0623/scene-6-claim-6-trust-and-approvals.md` F16 [P3].

🟢 **P3 — Cross-tenant isolation is genuinely live (one of the strongest sub-claims of Claim 6), but the LIST endpoint and the DETAIL endpoint return different response shapes for a cross-tenant query. List returns `{"items":[]}` (HTTP 200) — silent zero rows. Detail returns `{"error":{"code":"not_found","message":"..."}}` (HTTP 404). Both are correct (detail hides existence; list reveals nothing). But a buyer asking "prove tenant A can't see tenant B's data" gets two different demonstrations on what marketing presents as one guarantee. This is a docs / runbook narration finding, not a bug — the operator needs to know the shape difference to narrate it correctly.**

## Root cause

Curl-verified at audit time:

```bash
# Detail endpoint with bogus tenant header
curl -s -H "x-argo-tenant-id: another_tenant_test" \
  http://127.0.0.1:7400/api/v1/audit/<some-id>
# → 404 {"error":{"code":"not_found","message":"audit row not found"}}

# List endpoint with same bogus tenant header
curl -s -H "x-argo-tenant-id: another_tenant_test" \
  "http://127.0.0.1:7400/api/v1/audit?range=7d"
# → 200 {"items":[]}
```

The asymmetry is by design and correct:
- **Detail**: returning 404 (not 403) is the standard defence against existence-leak: an attacker can't tell whether the row exists in another tenant.
- **List**: returning an empty array is the natural shape — a bogus tenant truly has zero rows to list.

But the demo runbook (`docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 6) narrates a single "cross-tenant isolation PASS" without distinguishing the shapes. An investor probing "what happens if I forge a tenant header?" needs the operator to be able to articulate why:
- Showing an empty list is the response when iterating over rows.
- Showing a 404 is the response when looking up by ID.

Both prove isolation. Neither is wrong. The operator just needs the language.

This is also relevant for the SPA-side: `web/lib/api/audit.ts` reads the list endpoint, while `web/lib/api/audit-detail.ts` reads the detail endpoint. Their error-handling already diverges (detail throws `AuditDetailNotFoundError`; list returns empty). The SPA is fine — no code change. The fix is documentation.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F16 [P3] — List endpoint returns empty array (not 404) for bogus tenant — different but compatible isolation shape

## Fix recipe

1. **Add a sub-section to `docs/skycot/demos/v1-mission-mvp-runbook.md` Scene 6** under the "Cross-tenant isolation" beat:

   > **Isolation response shapes — narrate the difference**:
   > - **List endpoints** (e.g. `GET /api/v1/audit?range=7d`) return `{"items":[]}` (HTTP 200) for queries from a bogus or wrong-tenant header. A bogus tenant truly has zero rows to enumerate — empty is the honest shape.
   > - **Detail endpoints** (e.g. `GET /api/v1/audit/<id>`) return HTTP 404 for the same row from a wrong-tenant context. The 404 is intentional — returning 403 would leak existence-information; returning 404 means "this row is not in your tenant" indistinguishably from "this row does not exist."
   >
   > Both are correct; the asymmetry is by design.
2. **Add a similar note to `docs/skycot/operator/cross-tenant-isolation.md`** (or wherever the operator-facing isolation contract lives) — link from the runbook.
3. **Add a backend test that pins the shape contract** in `crates/argo-gateway/src/.../tests/cross_tenant_isolation.rs` (or sibling): assert list returns 200+empty, detail returns 404, for the same bogus-tenant header.
4. **Optionally**: add an integration test in `tests/` that scripts the operator's narrated demonstration: curl list (200, empty) + curl detail (404), with stdout assertions.

## Acceptance criteria

- [ ] Scene 6 of the runbook explicitly describes the list-vs-detail isolation shape difference.
- [ ] Backend test pins the shape contract (list 200+empty, detail 404).
- [ ] Operator can quote the response shapes during the demo without confusion.

## May-touch

- `docs/skycot/demos/v1-mission-mvp-runbook.md` (Scene 6 cross-tenant narration)
- `docs/skycot/operator/cross-tenant-isolation.md` (if exists; or NEW)
- `crates/argo-gateway/src/.../tests/cross_tenant_isolation.rs` (or sibling — pin the shape contract)

## Out of scope

- Code changes to either endpoint (no bug — both shapes are correct).
- Marketing copy on the landing page.
- RBAC enforcement beyond tenant binding (separate concern).

## Effort

~30-50 LOC doc + ~20 LOC test. ~30 min. P3 because there is no code bug; it's a narration / docs gap.
