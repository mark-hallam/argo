# fix-approvals-live-ws-blocked-by-csp-connect-src

**Priority**: P1 — the live approvals inbox WebSocket is blocked by the page's own Content-Security-Policy, so the real-time approvals feature silently dies and falls back to a static snapshot. Claim-6 ("multi-channel + real-time approvals") demo surface degrades to non-live without any visible error.
**Source**: 2026-05-29 full-site walkthrough (item-4), `/approvals` console error captured headless against the real gateway.
**Authored against**: origin/main `ac974cb06` on 2026-05-29
**Validate before claim**: `bash scripts/check-brief-freshness.sh fix-approvals-live-ws-blocked-by-csp-connect-src`
**Authored by**: claude conductor (interactive session)

## The bug (verified live)

On `/approvals`, the browser console shows:

```
Connecting to 'ws://127.0.0.1:7400/inbox/ws' violates the following Content
Security Policy directive: "connect-src 'self' http://127.0.0.1:7400
https://api.skycot.com". The action has been blocked.
```

Root cause: CSP `connect-src` governs **both** `fetch()` and **WebSocket** connections, and treats `http:` and `ws:` as **distinct schemes**. The directive in `web/next.config.ts:116` whitelists only the **http** gateway origin:

```js
`connect-src 'self' ${gatewayConnect} https://api.skycot.com`,   // gatewayConnect = http://127.0.0.1:7400
```

But the live-approvals hook connects over **ws://** — `web/lib/hooks/use-approvals-live.ts:71`:

```js
const wsBase = resolveGatewayUrl().replace(/^http(s?):/, "ws$1:");  // -> ws://127.0.0.1:7400
return `${wsBase}/inbox/ws${qs}`;
```

So the WS handshake is refused by the browser. The hook then degrades to its polling/offline path — the inbox still renders the seeded snapshot, so there is **no visible failure**, only the console line. Live `approval.created` / `approval.decided` events never arrive in the SPA.

## Scope / fix

In `web/next.config.ts` `headers()`, derive the websocket origin(s) from the existing gateway URL and add them to `connect-src`:

```js
const gatewayConnect = gatewayUrl.replace(/\/$/, "");
const gatewayWsConnect = gatewayConnect.replace(/^http(s?):/, "ws$1:");   // ws://127.0.0.1:7400 (or wss:// in prod)
const csp = [
  "default-src 'self'",
  `connect-src 'self' ${gatewayConnect} ${gatewayWsConnect} https://api.skycot.com wss://api.skycot.com`,
  // ...rest unchanged
].join("; ");
```

- The local dev gateway resolves to `ws://127.0.0.1:7400`; production (`https://api.skycot.com`) needs `wss://api.skycot.com`. Add both the derived ws origin AND the explicit prod `wss://api.skycot.com`.
- Keep the directive minimal — only add the ws/wss origins, nothing else.

## Acceptance

- [ ] `/approvals` opens the live WS with no CSP console error (verify headless: load `/approvals` against a real gateway, assert zero `connect-src` violations in console).
- [ ] CSP `connect-src` contains the ws origin derived from the gateway URL + `wss://api.skycot.com`.
- [ ] A test pins the CSP string includes the ws/wss origins (extend the existing next.config / headers test, or add one) so this can't silently regress.
- [ ] `pnpm exec tsc --noEmit` clean; `pnpm exec next lint` clean for touched files.
- [ ] No other CSP directive changed (diff shows only the `connect-src` line).

## Out of scope

- Any change to the WS protocol / `use-approvals-live.ts` itself (the hook is correct; only the CSP needs to allow the scheme).
- Gateway-side `/inbox/ws` behavior.

## May-touch

- `web/next.config.ts` (the `connect-src` line in `headers()`)
- the next.config / security-headers test file (add the ws-origin assertion)

## Operator authority

Web-only, small surface — eligible for the web-only admin-merge lane (≤200 LOC, ≤5 files, vitest/headless check green in PR body) per the 2026-05-26 operator authorization.
