# fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request

**Owner**: claude or codex.
**Authored against**: origin/main `4b3aa443f` on `2026-05-27`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-audit-feed-filter-or-bucket-spa-self-traffic-http-request`.
**Source**: UX-AUDIT-T0519 2026-05-27 — `docs/skycot/ux-audits/2026-05-27-0519/scene-6-claim-6-trust-and-approvals.md` F9.

🟡 **P1 — Live gateway audit feed at `GET /api/v1/audit?range=7d` returns 31 rows. EVERY row is `worker.name == "unknown"`, `verb == "http.request"`, `action_type ∈ {audit.detail.viewed, audit.replay}`. These are the SPA's own audit telemetry of the auditor's clicks during the audit run — not real agent turns. Scene 6 narrates "the audit page renders ≥10 rows in Etherscan-style table" — what the camera sees is meta-rows from the SPA, not work the AI teammates did.**

## Root cause

The gateway audits every HTTP request to the gateway itself (including the SPA's read-only fetches: opening `/audit`, drilling into a row, hitting `/replay`). These rows are tagged `verb=http.request` and `worker.name=unknown` because there's no agent-turn behind them — they're navigational telemetry.

Two reasonable design positions:
- **(a) Filter `http.request` from the user-facing feed.** Operator only sees rows where an agent actually did something (`worker.name != unknown` AND `verb != http.request`).
- **(b) Bucket meta-rows into a separate "Audit access log" tab.** Keeps the data but separates it from agent activity.

The auditor's evidence:
```bash
curl -s "http://127.0.0.1:7400/api/v1/audit?range=7d" | jq '.items | (length, (map(.worker.name) | unique), (map(.verb) | unique))'
# → 31, ["unknown"], ["http.request"]
```

This compounds with brief `fix-audit-unknown-worker-and-hipaa-fixture-rows.md` (which flags `unknown` worker rows as fixture leak) — but that one is fixture-side; this brief is the gateway-emission side. Together they should land "audit page = real agent turns only".

If demo seed runs and the audit table fills with real agent-turn rows (Mariah/Peta/Priya/Jamal/Nori), the noise rows still need filtering to keep the demo page readable.

## Findings this brief closes

- scene-6-claim-6-trust-and-approvals F9 [P1] — Live-gateway audit feed is 31 rows of SPA self-traffic — no agent turns

## Fix recipe

Two paths; (a) is the demo-gate fix, (b) is the post-MVP polish.

**Path 1 — Filter at the list endpoint (preferred for demo).**

1. In `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` (the list handler) add a default filter:
   - Exclude rows where `action_type IN ('http.request', 'audit.detail.viewed', 'audit.replay', 'audit.list.viewed')` UNLESS the query string carries `?include_meta=1`.
   - This keeps the API surface intact (operators can still curl `?include_meta=1` for forensics) but the default user-facing feed shows agent turns only.

2. Cargo test: assert default response excludes `http.request` rows; `?include_meta=1` includes them.

**Path 2 — SPA-side filter (faster to land).**

3. In `web/lib/api/audit.ts:194-217` `normalizeAuditRow` (or in the feed-shape post-processor), drop rows where `verb == 'http.request'` AND `worker.name == 'unknown'`. Reasoning: these are guaranteed-noise; no real action attribution.

4. Vitest pin: feed normalizer drops noise rows.

**Path 3 — Separate tab (post-MVP polish).**

5. Build a `/audit/access-log` tab inside the audit shell. Default surface is agent turns; "Show access log" toggle reveals the meta-rows. Lower priority; not a demo gate.

## Acceptance criteria

- [ ] `/audit` default render shows zero rows where `worker.name == "unknown"` AND `verb == "http.request"`.
- [ ] Operator can still access meta-rows via a documented mechanism (query param OR separate tab).
- [ ] Cargo test (Path 1) OR Vitest pin (Path 2) protects against regression.
- [ ] If demo seed adds real agent-turn rows, those rows DO show up.

## May-touch

- `crates/argo-cli/src/handlers/gateway/routes_audit_list.rs` (Path 1)
- `crates/argo-cli/tests/audit_list_default_filter.rs` (Path 1 test)
- `web/lib/api/audit.ts` (Path 2 normalizer)
- `web/lib/api/__tests__/audit.test.ts` (Path 2 pin)
- `web/app/(app)/audit/access-log/page.tsx` (Path 3 — separate tab)

## Out of scope

- `unknown` worker fixture rows (covered by `fix-audit-unknown-worker-and-hipaa-fixture-rows.md`)
- Demo seed CLI (covered by `fix-demo-seed-cli-for-real-roster.md`)
- Hash-chain prev→this affordance (covered by `fix-audit-hash-chain-prev-this-hover-and-live-rows.md`)

## Effort

S — ~50-100 LOC. ~1-2 hours.
