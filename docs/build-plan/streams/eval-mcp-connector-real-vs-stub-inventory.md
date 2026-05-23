# eval-mcp-connector-real-vs-stub-inventory

**Owner**: codex.
**Depends on**: none (read-only audit).
**Authored against**: origin/main `03515459c` on `2026-05-23`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh eval-mcp-connector-real-vs-stub-inventory`.

🟡 **P1 SALES-PROMISE-VS-CODE — for every shipped MCP connector, classify REAL / STUB-OK / STUB-NEEDS-WORK.** Source: `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` (the open `🟡 STILL OPEN` row under "27 → 29 MCP connectors").

> "Real-vs-stub inventory still owed. For each connector crate, check (a) does it have a real `connect()` that hits the upstream API, (b) is its `request()` keyed by per-tenant OAuth tokens, (c) is there at least one passing smoke test that doesn't hit real network? Tag each connector REAL / STUB-OK / STUB-NEEDS-WORK."

Credential-scoping was just audited (`eval-mcp-connector-credential-scoping-sweep`, 2026-05-23) and 3 P0 cwd-cache leaks already have a fix-brief in flight (`mission-mcp-connector-token-cache-tenant-keyed` — MERGED via #2103 tick-91). The remaining gap is the **substrate-vs-stub matrix**: which connectors actually call the upstream API, which are placeholder shells, and which are placeholders that mistakenly *look* real to the marketplace.

## Scope

Pure read-only audit. No production code changes. Output is a single Markdown report keyed by connector crate, plus a small sibling test that pins the inventory so future drift is caught.

### Connectors in scope (29 crates + 1 helper)

Enumerate via `ls crates/argo-mcp-connector-*` to get the authoritative list (operator memory says 4 consumer/redundant stubs are being deleted — signal, telegram, mailgun, resend; 3 enterprise stubs stay as DEFERRED-v1.x — apollo, zoominfo, zoom). Report the full list as you find it; don't assume.

### Per-connector inventory cells

For each connector, fill these cells in the report:

| Cell | Question | How to answer |
|---|---|---|
| **a. real connect** | Does the connector's `connect()` actually hit the upstream API (real OAuth, real REST handshake)? | grep for `reqwest::`, `Client::new()`, `https://`. STUB if all paths return placeholder data or mock. |
| **b. tenant-keyed creds** | Does `request()` / tool dispatch fetch creds via `cred::get(tenant_id, ...)` (not `cred::get_unscoped`)? | Check the production callsite signature. |
| **c. smoke-test exists** | Is there at least one `#[test]` or `#[tokio::test]` in `tests/` that exercises the connector without hitting real network (fake server / wiremock / hand-rolled stub)? | `ls crates/argo-mcp-connector-<name>/tests/`. |
| **d. operator-configurable** | Can the operator configure this connector via the SPA `/settings/connectors` page today? (Cross-check `web/app/settings/connectors/page.tsx`.) | grep for the connector name in the SPA settings. |
| **e. tier** | Per operator pricing — Free / Pro / Enterprise / DEFERRED-v1.x? | Cross-check `docs/skycot/marketing/PRICING.md` if it exists; else mark "?" and surface as a follow-up. |
| **verdict** | REAL (a+b+c green) / STUB-OK (admittedly a placeholder, marketing-honest) / STUB-NEEDS-WORK (looks real to marketplace but isn't) / DEFERRED-v1.x | One-line summary per connector. |

### Where the artefacts live

- `NEW: docs/skycot/evals/scheduled/2026-05-23/eval-mcp-connector-real-vs-stub-inventory.md` — the matrix report (table + 1-paragraph commentary per connector). ~300-500 LOC of markdown.
- `NEW: crates/argo-mcp/tests/connector_inventory_pin_test.rs` — a small structural test that asserts the connector count matches what the operator promises (e.g., "29 connectors" pin). When a connector is added or removed, this test forces an inventory refresh. ~50-100 LOC.

### Findings escalation

- If any connector is classified **STUB-NEEDS-WORK** (looks real but isn't), file a follow-up brief `mission-<connector>-real-implementation-or-deprecate` (one brief per finding). Don't fix in this brief — out of read-only scope.
- If any connector is **REAL but operator-not-configurable** (cell d is "no"), file `web-connector-<name>-settings-page-row`.
- If any connector is **REAL but missing smoke test** (cell c is "no"), file `eval-<connector>-add-smoke-test`.

Don't author the follow-up briefs in-line — list them as recommendations at the bottom of the report; the conductor will pick them up.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Auto-spawn: automatic_if_supported
- Manager package: n/a
- Shared hotspots: []

## Files you may touch

- `NEW: docs/skycot/evals/scheduled/2026-05-23/eval-mcp-connector-real-vs-stub-inventory.md`
- `NEW: crates/argo-mcp/tests/connector_inventory_pin_test.rs`
- `docs/skycot/evals/scheduled/2026-05-23/mission-vs-code-gap-manifest.md` — flip the "27 → 29 MCP connectors" row to 🟢 on the real-vs-stub axis.
- `docs/build-plan/BUILD_STATUS.md` — standard row flip.

## Files you must NOT touch

- `crates/argo-mcp-connector-*/src/**` — strict read-only. This is an inventory audit, NOT a refactor. If you find a connector that mis-advertises itself as real, file the follow-up brief; don't edit the connector.
- `argo-mcp` runtime code — out of scope.
- Other crates outside the connectors.

## Acceptance criteria

1. Report file exists at the specified path with every connector classified across all 6 cells.
2. Structural pin test added (asserts the connector count matches the report).
3. Gap-manifest row flipped 🟢 on the real-vs-stub axis.
4. At least one paragraph of commentary on each STUB-NEEDS-WORK finding (these are the operationally-load-bearing ones for marketplace credibility).
5. Pre-merge gate green: `bash scripts/conductor-pre-merge-gate.sh`.

## Why this is codex-eligible

- Pure read-only static analysis across many crates with a consistent grep pattern — fits codex's parallel-search strength.
- No architectural judgment needed — the cell answers are mechanically derivable from the source.
- Output is a matrix report, not a tricky refactor.
- Pre-flight: `ls crates/argo-mcp-connector-*` then per-crate `rg` calls.

## Bounded effort

- Expected ~300-500 LOC markdown + ~50-100 LOC test.
- 29 crates × ~5 minutes per crate of mechanical inspection = ~2.5 hours of focused work; budget conservatively as one codex session.
