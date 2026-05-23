# eval-mcp-connector-credential-scoping-sweep

**Owner**: claude (evaluator-shape brief — investigation + findings + tests).
**Depends on**: `mission-cred-get-tenant-parameter` (informational — outcome of this audit may need the tenant-parameterized cred::get).
**Authored against**: origin/main `<latest>` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-mcp-connector-credential-scoping-sweep`.

🟡 **P1 — per-connector OAuth credential scoping unaudited.** Source: evaluator round-10 SHARED.md handoff 2026-05-23T10:55:00Z.

> "27 MCP connector structural inventory PASS (29 real crates compile + tests; substrate healthy). 3 follow-up: ... eval-mcp-connector-credential-scoping-sweep (P1 — per-connector OAuth scoping NOT yet verified, canonical next-round focus)."

Round 10 cleared the substrate-level inventory (all 27+ MCP connector crates compile + their tests pass). But the cross-tenant credential scoping per-connector was NOT audited. Each connector handles its own OAuth refresh + token storage; cross-tenant leak HERE undoes the work of `mission-cred-get-tenant-parameter`.

## Scope

Walk all 27 MCP connector crates (`crates/argo-mcp-*`). For each:

1. **OAuth token source**: where does the connector read its refresh token / access token from?
2. **Tenant scoping**: is the read tenant-bound, or does it use the process-wide `cred::get(name)` API (the leak)?
3. **Token refresh**: when the connector refreshes an access token, where does the new token land? Tenant-bound, or cwd-shared?
4. **Disk caches**: per #2080 finding §5, OAuth access-token caches at `<cwd>/.argo/connectors/<name>/access_token.json` are process-cwd shared. How many connectors use this pattern?

Build a per-connector matrix:

| Connector | OAuth source | Tenant-scoped? | Refresh tenant-scoped? | Disk cache location | Verdict |
|---|---|---|---|---|---|

Deliverable: `docs/skycot/evals/scheduled/2026-05-23/eval-mcp-connector-credential-scoping-sweep.md` with the full matrix + per-row findings.

## Parallelism

- Manager-worker allowed: yes (one child per connector cluster — slack/gmail/gcal/etc.)
- Max children: 4
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: docs/skycot/evals/scheduled/2026-05-23/eval-mcp-connector-credential-scoping-sweep.md` — the matrix report.
- `OPTIONAL NEW: crates/argo-mcp-<connector>/tests/eval_findings_cred_scoping.rs` for any connector with a missing test.
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — update connector rows.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- Any `crates/argo-mcp-*/src/**` — strict read-only.
- The credentials crate itself.

## Acceptance criteria

- [ ] Per-connector matrix covers all 27 MCP connectors.
- [ ] Each connector classified: tenant-scoped / partially / leak.
- [ ] Findings prioritized P0/P1/P2 with fix-brief recommendations.
- [ ] Gap manifest updated.
- [ ] Cost target 50-100K tokens.
- [ ] BUILD_STATUS.md updated.

## Branch + PR

- Branch: `eval-mcp-connector-credential-scoping-sweep`
- PR title: `eval(mcp): per-connector credential scoping sweep — 27-row matrix (eval-mcp-connector-credential-scoping-sweep)`

## Notes for the implementer

- ~200-500 LOC of report + optional tests.
- This is the canonical next-round focus per round-10. If you find a P0 leak in any connector, surface it as 🚨 ESCALATION.
- Cite round-10 handoff verbatim.
