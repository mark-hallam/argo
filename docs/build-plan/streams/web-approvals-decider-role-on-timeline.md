# web-approvals-decider-role-on-timeline

**Authored against**: `origin/main` `a586ab07f` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-approvals-decider-role-on-timeline`.

## Summary

**REAUTHORED 2026-05-21 (tick-26)** after a previous worker self-BLOCKED with a thorough premise audit showing `/approvals` route doesn't exist, ActivityRow doesn't render approver names, and `decider_role` doesn't flow through `/api/v1/audit` feed. The reauthored scope targets the surfaces that ACTUALLY exist:

1. `ActionSummaryHero` at `/audit/action/<id>` (the drill-in page) currently renders "Approver: <name>" — extend to "Approver: <Role> <name>" when `decider_role` is present.
2. `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs:241-253` projects `approver` but not `decider_role` — extend the feed extractor to surface the field.
3. `web/lib/types/audit.ts` extends the `AuditRow` shape with optional `decider_role` (additive, backward-compat).

Pre-condition: **#1956 MERGED ✅** (`decider_role` stamped into the audit-memory row).

## Acceptance

- [ ] Gateway feed `GET /api/v1/audit` returns `decider_role` in each row's JSON when the underlying audit-memory row carries it (additive field — backward-compat with pre-#1956 rows).
- [ ] `web/lib/types/audit.ts` `AuditRow` type extended with `decider_role?: "owner" | "admin" | "user" | "guest"`.
- [ ] `ActionSummaryHero` (`/audit/action/<id>` drill-in) renders the role chip alongside the approver name when `decider_role` is present. Plain-language: "Admin Mark", not "Mark (admin)".
- [ ] Backward-compat: rows without `decider_role` render existing "Approver: Mark" format unchanged.
- [ ] Subtle role-chip styling (muted text, no gradient, no Audiowide — matches the audit page's restrained register).
- [ ] One gateway integration test: row with `decider_role` in payload → feed JSON includes the field.
- [ ] One component test: `ActionSummaryHero` with `decider_role: "admin"` renders the chip; row without renders without.
- [ ] No regression on existing audit page or ActivityRow rendering.

## May touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs` (feed extractor)
- `crates/argo-cli/src/handlers/gateway/routes_audit_feed_tests.rs` (extend tests)
- `web/lib/types/audit.ts` (additive field)
- `web/components/skycot/audit/ActionSummaryHero.tsx` (or similar — verify exact path with `rg -ln 'Approver:' web/components/`)
- Component test files

## Must NOT touch

- The `/approvals` route — it doesn't exist yet.
- `ActivityRow` — it's the timeline-row primitive; scope is the drill-in page only.
- The server-side role gate (#1956 settled).
- The audit memory row schema (extend at projection layer, not at storage).
- The audit page's locked register (`docs/skycot/audit/...` design lock).

## Notes

- Pre-flight: `rg -ln 'ActionSummaryHero' web/components/` to find the canonical location.
- Pre-flight: `cat crates/argo-cli/src/handlers/gateway/routes_audit_feed.rs | grep -B2 -A5 'approver'` to see current shape.
- Sizing: 150–300 LOC (backend additive field + frontend chip + 2 tests).
- Previous claim BLOCKED with premise audit; this reauthor closes the gaps.
- Demo-visible improvement to mission-claim-6 procurement story.
