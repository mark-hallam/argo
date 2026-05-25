# mission-demo-scene-6-audit-feed-hash-chain-renders

**Owner**: claude.
**Authored against**: origin/main `838e1141a` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh mission-demo-scene-6-audit-feed-hash-chain-renders`.
**Depends on**: `mission-demo-seed-data-bootstrap` MERGED (delivers the Pass A drill-in audit rows + Pass B dressing-tier filler).

🔴 **P0 DEMO-GATE — Scene 6 rendering verification (FUNCTION-SIDE per `feedback_no_theater_in_demo` clarified 2026-05-25T08:10Z).**

`v1-mission-mvp-runbook.md` Scene 6 demonstrates the audit chain — operator hovers a row to reveal the hash-chain affordance + clicks into a row to see real provenance. This is the ONLY scene that visually proves cross-tenant isolation (claim 6) and audit integrity (claim 1/2 substrate). Per the function-vs-dressing rule, the audit row the camera drills into MUST be from a real agent turn (Pass A); the visible filler rows around it can be Pass B fixtures as long as they share the same `AuditSink::emit` write path so chain integrity holds end-to-end.

## Scope

1. **Pre-flight**: confirm `mission-demo-seed-data-bootstrap` MERGED. Seed has ≥10 Pass A real-turn rows + 100-200 Pass B fixture rows.
2. **Run the seed**: `argo demo seed --tenant <demo-tenant-id>`. Capture transcript.
3. **Verify gateway**: `curl http://localhost:8088/api/v1/audit?limit=20` returns ≥10 rows with valid `audit_record` schema (real worker_id + real tenant_id + valid payload).
4. **Verify chain integrity**: `argo audit verify --tenant <demo>` prints `OK` with non-zero rows + zero breaks. This is load-bearing — the seed must NOT have broken the hash chain.
5. **Verify cross-tenant invisibility**: `curl 'http://localhost:8088/api/v1/audit?tenant=other_tenant'` returns 0 rows for the demo tenant's view of other_tenant. Same query authenticated as other_tenant returns ONLY other_tenant's rows.
6. **Render check** — open `/audit` in a headless browser:
   - Table renders ≥10 rows (combination of Pass A + Pass B)
   - B3-enterprise Etherscan-style design per memory `project_audit_page_design_locked` (Inter font, kind-column color-coded, pop-out modal on click)
   - Hash-chain hover affordance: hovering a row reveals the prev-hash + this-row-hash relationship visually (per Scene 6 talk track)
   - Clicking a Pass A row (drill-in) navigates to `/audit/action/{id}` and renders the REAL provenance — NOT the F67 fabricated-mock-report that the rejected audit doc flagged
7. **Drill-in real-ness check**: navigate to a Pass A row's detail page. The page must render real `payload` + real `created_at` + real `worker_id` linkage. If the page is currently the F67-style "Policy regression detected. Replayed against current policy. Operator notified." mock, file a SEPARATE fix brief — the F67 fabrication is dressing-side, not function-side, but if the camera drills into it during Scene 6 it becomes function-side.

## Acceptance criteria

- [ ] `argo audit verify --tenant <demo>` exits 0, non-zero rows, zero breaks.
- [ ] `curl http://localhost:8088/api/v1/audit?limit=20` returns ≥10 well-formed rows.
- [ ] Cross-tenant: foreign tenant query returns 0 rows for demo tenant's view.
- [ ] `curl -s http://localhost:3000/audit` HTML contains ≥10 audit rows.
- [ ] Hash-chain hover affordance present (verify via DOM inspection or screenshot).
- [ ] Click-through to a Pass A row's detail page shows real provenance (verify by matching the audit row's payload between the row-list JSON and the detail page DOM).
- [ ] PR body has screenshots: /audit page list + hover-state + click-through detail.

## May-touch

- `web/app/audit/page.tsx` (if rendering gaps found)
- `web/app/audit/action/[id]/page.tsx` (if drill-in still F67-style)
- `web/app/audit/action/[id]/replay/page.tsx` (out of scope if demo doesn't navigate there — note in PR if it's still F67-style for a follow-up)
- `web/lib/api/audit.ts` (if envelope mismatch with current gateway)
- This brief file (status updates)

## Out of scope

- Re-implementing the audit page design (locked + presumably JSX per 25-audit sweep)
- Changing the audit-row schema
- Touching `argo-audit` Rust code

## Effort

~30-60 min if rendering correct; +30-90 min per fix-up.
