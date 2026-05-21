# web-approvals-strict-prod-403-toast

**Authored against**: `origin/main` `652a1825` on `2026-05-21`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh web-approvals-strict-prod-403-toast`.

## Summary

Companion to #1956 (`mission-rbac-approvals-decide-role-gate`). The new server-side role gate returns 403 when a non-admin tries to approve/refuse — but the client at `web/lib/api/approvals.ts:submitDecision` currently doesn't have a dedicated path for 403 (it falls into the generic error path that drops the optimistic mutation). This brief adds a friendly, role-aware toast: "You don't have permission to approve. Ask an admin in your workspace." Pure UX polish on top of the new gate.

Pre-condition: **#1956 MERGED** (the gate must exist before this can be verified end-to-end).

## Acceptance

- [ ] `submitDecision` in `web/lib/api/approvals.ts` catches the 403 from `/api/v1/approvals/<id>/decide` and surfaces a distinct toast: "You don't have permission to approve. Ask an admin in your workspace." (or similar plain-language copy).
- [ ] The optimistic mutation rolls back cleanly (no stuck "Approving…" state).
- [ ] Unit test (or integration test depending on existing test infra) covering: 403 from mock fetch → toast appears + state restored.
- [ ] Existing 200 / 404 / 500 / network-error paths still work unchanged.
- [ ] Plain-language copy per operator preference (no "Forbidden" or "Unauthorized" jargon — use the friendly variant).
- [ ] No regression on the cross-tenant audit gate from #1901.

## May touch

- `web/lib/api/approvals.ts` (the `submitDecision` function)
- `web/components/skycot/approvals/` if a new toast component is the cleanest path
- Toast helper if existing one needs a new variant
- Tests for the new 403 path

## Must NOT touch

- The server-side role gate (#1956 owns that).
- Other `/api/v1/...` endpoints.
- The approvals page register (`/approvals` is brand-restrained per locked design — no Audiowide here).

## Notes

- Operator preference: plain language. "You don't have permission" not "Forbidden".
- Operator memory: `project_approvals_page_design_locked.md` — register stays enterprise-restrained, Inter only, no gradient.
- Sizing: 80–150 LOC.
- This is a "pairs with" brief — fast to land once #1956 ships.
