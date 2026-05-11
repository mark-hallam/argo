# Stream E1-a — Approvals inbox: gateway wire-up (replace mock with real fetch)

**Owner**: claude.
**Depends on**: E0-frontend-foundation MERGED.
**Authored against**: origin/main `1569d286` on `2026-05-11`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh E1-a-approvals-gateway-wireup`.
**Tier tag**: PRODUCT_ROADMAP.md Track E slice E1 (approvals inbox). First brief in the slice. Closes the "no real data" gap on the daily-driver surface.

## Why

The approvals-inbox demo built in Track C reads from a static
`SAMPLE_APPROVALS` array in `web/lib/sample-approvals.ts`. E0
introduced the typed gateway client at `lib/api/client.ts`. This
brief connects the two: `useApprovals()` fetches real data from
the gateway's JSON inbox endpoint; `submitDecision()` POSTs real
decisions; `submitUndo()` POSTs real undos.

Today the gateway serves `/inbox` as HTML
(`crates/argo-cli/src/handlers/gateway/routes_channels_webinbox.rs`).
A JSON endpoint at `/api/v1/approvals` (and `/decide` / `/undo`)
is a separate backend brief (`mission-gateway-approvals-json-api`,
to be authored — see Notes). When that endpoint lands, this
brief unblocks immediately.

Until the JSON endpoint exists, **this brief still ships**: it
wires the real fetch path BUT keeps the mock fallback when the
gateway returns 404 (per E0's spec). Result: behavior is
identical to today on dev machines without a JSON-aware gateway,
and identical to a real production setup once the endpoint
lands. No second migration.

## Scope

- Replace the bodies of `fetchApprovals`, `submitDecision`,
  `submitUndo` in `web/lib/api/approvals.ts` with real
  gateway calls via `lib/api/client.ts`.
- Preserve the existing mock fallback path: a 404 from the
  gateway (or `NetworkError` if gateway is unreachable)
  returns `SAMPLE_APPROVALS`. This keeps the dev experience
  bisectable.
- Add a tiny in-page indicator chip in the approvals header
  showing the data source: `live` (real gateway data),
  `mock (offline)` (gateway unreachable / 404), or `mock
  (auth-bypass)` (when `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`).
  Chip lives at the right side of the page subtitle row.
- Map gateway response shape to `ApprovalItem`. If shapes
  diverge from the existing TS type, write the mapping in
  `lib/api/approvals.ts` rather than changing the type
  (downstream consumers depend on the shape).
- Add 4 unit tests against `fetchApprovals`/`submitDecision`/
  `submitUndo` with mocked `apiGet`/`apiPost`: happy path,
  404 → mock fallback, 500 → ApiError thrown, auth-bypass
  path returns mock.
- Add ONE Playwright e2e: load `/` with no gateway running
  (mock fallback), click Approve on first card, assert it
  flips to "Approved" state.

## Parallelism

- Manager-worker allowed: no
- Max children: 0
- Max depth: 1
- Shared hotspots: `web/lib/api/approvals.ts` (locks until
  E1-a merges; E1-b through E1-f wait their turn)

## Files you may touch

- `web/lib/api/approvals.ts` — REPLACE the mock bodies.
  Real fetch + 404 fallback + auth-bypass detection. ≤ 200
  LOC.
- `web/lib/api/approvals.test.ts` — NEW. 4 unit tests. ≤ 200
  LOC.
- `web/lib/hooks/use-approvals.ts` — narrowly scoped: add a
  return field on `useApprovals()`:
  `dataSource: "live" | "mock-offline" | "mock-auth-bypass"`.
  Surfaced from a header set by `fetchApprovals` on the
  returned data. ≤ 120 LOC.
- `web/app/(app)/page.tsx` (or wherever the approvals
  inbox page lives after E0's move) — narrowly scoped:
  render a `DataSourceChip` near the page subtitle. ≤ 250
  LOC.
- `web/components/skycot/DataSourceChip.tsx` — NEW. Tiny
  chip ≤ 50 LOC. Reuses StatusPill primitive.
- `web/tests/e2e/approvals-inbox.spec.ts` — NEW. Playwright
  spec, mock-fallback path. ≤ 100 LOC.
- `web/playwright.config.ts` — NEW if not present;
  narrowly scoped if present. Configure the dev-server URL +
  retry settings. ≤ 60 LOC.

## Files you must NOT touch

- `web/lib/sample-approvals.ts` — keep as-is. The mock
  data is the fallback source.
- `web/components/skycot/ApprovalCard.tsx` — locked. This
  brief is data-only; no card structure changes.
- `web/components/skycot/ApprovalCard.test.tsx` — locked.
- Any file in `crates/**` — backend changes are a separate
  brief.

## Acceptance criteria

- [ ] When `NEXT_PUBLIC_GATEWAY_URL` resolves to a JSON-
      aware gateway, `useApprovals()` returns real data;
      `submitDecision` POSTs real decisions; `submitUndo`
      POSTs real undos.
- [ ] When the gateway returns 404 on `/api/v1/approvals`,
      `fetchApprovals` returns the mock dataset and
      `dataSource = "mock-offline"`.
- [ ] When the gateway is unreachable (`NetworkError`
      or `ECONNREFUSED`), same fallback as above.
- [ ] When `NEXT_PUBLIC_GATEWAY_AUTH_BYPASS=1`,
      `dataSource = "mock-auth-bypass"` and no gateway
      calls are made (purely local-dev mode).
- [ ] `DataSourceChip` renders the current source. Strings:
      `"live"` (tone `success`), `"mock (offline)"` (tone
      `warning`), `"mock (auth-bypass)"` (tone `info`).
      Hidden in production builds (`process.env.NODE_ENV
      === "production"`) where only `live` is acceptable.
- [ ] 4 unit tests pass.
- [ ] Playwright e2e passes against the mock-fallback path.
- [ ] `pnpm typecheck` clean.
- [ ] `pnpm build` clean. Bundle size delta < 2KB First
      Load JS.
- [ ] Every file < 500 LOC.
- [ ] BUILD_STATUS.md updated with PR number.

## Branch + PR

- Branch: `claude-E1-a-approvals-gateway-wireup`
- PR title: `web(approvals): wire approvals inbox to gateway with mock fallback (E1-a)`

## Notes for the implementer

- **Endpoint shape (assumed; verify with the backend brief
  before merge if it lands first):**
  - `GET /api/v1/approvals` returns
    `{ items: ApprovalItem[], cursor?: string }`. Map to
    `ApprovalItem[]`.
  - `POST /api/v1/approvals/<id>/decide` body
    `{ decision: "approve" | "refuse" }` returns
    `{ ok: true, approvalId, decision }`.
  - `POST /api/v1/approvals/<id>/undo` returns
    `{ ok: true, approvalId }`.
  - All require `Authorization: Bearer <token>` (handled by
    the E0 client wrapper).
  - 404 on any of these means the JSON endpoint isn't
    wired yet → fallback to mock.

- **If the gateway returns a different field shape**, do the
  mapping in `fetchApprovals` and the mutation handlers. Do
  NOT change `ApprovalItem` — downstream code depends on
  the TS shape. A separate backend brief (or a follow-up
  here) should align the gateway response.

- **DataSource detection.** The simplest signal is whether
  the fetch returned data from the real path vs the
  fallback. The hook stores this on a side-channel field
  in the cache:
  ```ts
  const data = await fetchApprovals();
  return { items: data, source: data.length === 0 ? "live" :
    data === SAMPLE_APPROVALS ? "mock-offline" : "live" };
  ```
  Actually simpler: make `fetchApprovals` return
  `{ items, source }` and have the hook split them.
  See the test file for the exact contract.

- **Bypass shortcut.** Read
  `process.env.NEXT_PUBLIC_GATEWAY_AUTH_BYPASS` at module
  load (it's a `NEXT_PUBLIC_*` so it's inlined at build
  time, same model as the gateway URL). When set, return
  the mock immediately without any fetch attempt. This
  preserves zero-config local dev.

- **Backend brief.** Author a separate brief
  `mission-gateway-approvals-json-api` (or similar) that
  adds the three endpoints on the Rust side. Until that
  brief MERGES, this brief's behavior on a real (HTML-
  serving) gateway is: returns 404 → falls back to mock.
  Both states are valid for v1; the user sees the
  DataSourceChip indicating which mode they're in.

- **Optimistic update preservation.** The existing
  `onMutate` in `useSubmitDecision` already does the
  optimistic flip. Leave it alone — the real POST path
  goes through `mutationFn`, which calls `submitDecision`,
  which now hits the gateway. The cache is already
  optimistic; success leaves it; error rolls it back via
  `onError`.

- **Test the chip visibility.** Production builds must NOT
  show the chip — it's a dev affordance. Use
  `NODE_ENV === "production"` as the suppression flag and
  render `null` in that case. Add a TS guard so the chip
  isn't dead-code-eliminated unintentionally.

- **Playwright config.** If `playwright.config.ts` doesn't
  exist, create one with:
  ```ts
  import { defineConfig } from "@playwright/test";
  export default defineConfig({
    testDir: "./tests/e2e",
    use: { baseURL: "http://localhost:3000" },
    webServer: {
      command: "pnpm dev",
      url: "http://localhost:3000",
      reuseExistingServer: !process.env.CI,
    },
  });
  ```
  The e2e spec should explicitly set
  `NEXT_PUBLIC_GATEWAY_URL` to a non-existent port so the
  test is deterministic (no flaky network).

- **What this brief explicitly does NOT do.**
  - Doesn't filter, paginate, search, or batch-decide.
    Those are E1-b through E1-f.
  - Doesn't add websocket live updates. That's E1-f.
  - Doesn't change the page layout. The DataSourceChip is
    a small inline addition; everything else is unchanged.

- **PII redaction is NOT wired here.** Approvals come back
  with full content; redaction happens in E1-e. If the
  gateway already redacts based on RBAC, that's fine — the
  client just renders what it gets.
