# sprint-ws1-canonicalize-loopback-host-and-fix-demo-mode-error

**Owner**: claude.
**Authored against**: origin/main `eddcab780` on `2026-06-09`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh sprint-ws1-canonicalize-loopback-host-and-fix-demo-mode-error`.

## Why (chip task_341a020b, VERIFIED 2026-06-09)
The session cookie is `__Host-session`, minted with `__Host-` prefix attributes — `Path=/; HttpOnly; Secure; SameSite=Lax` and **no `Domain`** (`web/app/api/auth/demo-mint/route.ts:136-137` `sessionCookieAttrs()`; the same prefix is documented as host-pinned at `:127-129`). The `__Host-` prefix host-pins the cookie to the exact origin that set it: a cookie set on `localhost:3000` is NOT sent to `127.0.0.1:3000` (they are different hosts even though both are loopback). So opening the app on the non-canonical loopback host drops the session entirely.

When the session is dropped, the front door does one of two wrong things:
1. The middleware (`web/middleware.ts:216` `readSessionCookie` → `:137` reads `__Host-session`) sees no cookie. With `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1` set (the dev/demo profile) it passes the request through WITHOUT a session marker (`:218-226`), so the SPA renders but every authed save then hits the gateway with no verified principal.
2. The onboarding-save catch path scrubs the gateway's auth-reject error and **mislabels it as "demo mode."** `web/app/(setup)/onboarding/_scrub-error.ts:14-20` matches `/\bprincipal\b/i` / `/x-argo-principal/i`; the gateway's auth middleware returns exactly `X-Argo-Principal header must carry a verified human:<email> value` (cited in the scrubber's own header comment `:4-10`). On match the scrubber returns the hardcoded copy `"We can't save this in demo mode. Sign in to continue with a real workspace."` (`_scrub-error.ts:22-23`, returned at `:43-45`). That copy is consumed by the workspace / connect-provider / pick-vertical / first-hire steps (`rg` below). So an authed user whose session was silently dropped by a host mismatch is told they are "in demo mode" — which is false and misdirects them away from the real fix (re-auth on the canonical host).

`git log origin/main | rg -i loopback` shows ONLY CORS + bind-hardening work (#2359 `fix gateway dev cors loopback origins`, #1689 `harden non-loopback bind`, #1267 `refuse non-loopback bind`) — none touch the cookie-host mismatch or the misleading scrub copy. No BUILD_STATUS row exists for this. The gap is unaddressed.

## Grep-before-write
- `rg -n 'sessionCookieAttrs|__Host-session|Domain' web/app/api/auth/demo-mint/route.ts` — the cookie mint + its host-pinning attrs.
- `rg -n '__Host-session|readSessionCookie|AUTH_BYPASS|withSessionMarker' web/middleware.ts` — the front-door host gate (where canonicalization belongs).
- `rg -n 'DEMO_MODE_LEAK_COPY|principal|wasProtocolLeak' web/app/\(setup\)/onboarding/_scrub-error.ts` — the misleading-copy source.
- `rg -n 'scrubOnboardingErrorMessage' web/` — the four consumers (`workspace/page.tsx`, `connect-provider/page.tsx`, `pick-vertical/page.tsx`, `first-hire/page.tsx`) that render the scrubbed string.
- New symbols (confirm none exist first): `rg -n 'pub fn|export function (canonicalizeLoopbackHost|isNonCanonicalLoopbackHost)' web/` — both must return zero hits; this brief introduces `canonicalLoopbackRedirect()` as a middleware helper consumed only by `web/middleware.ts`.

## Fix
1. **Canonicalize the front-door host (middleware).** In `web/middleware.ts`, before the auth branches, add a small helper `canonicalLoopbackRedirect(req): NextResponse | null` that, when `req.nextUrl.hostname === "127.0.0.1"`, returns a 307 redirect to the same URL with hostname rewritten to `localhost` (preserving port, path, search). Return it early so the browser re-requests on the canonical host BEFORE any cookie read — the existing `localhost`-set `__Host-session` cookie is then sent and the session survives. (Choosing `localhost` as canonical because the cookie mint comment `demo-mint/route.ts:5` already names `localhost:3000` as the mint origin.) The only consumer is the middleware itself. Guard it so it never loops (skip if hostname is already `localhost`) and never touches non-loopback hosts.
2. **Replace the misleading "demo mode" copy with an honest re-auth message.** In `web/app/(setup)/onboarding/_scrub-error.ts`, change `DEMO_MODE_LEAK_COPY` (`:22-23`) from the "demo mode" wording to an honest session-expired message, e.g. `"Your session expired. Please sign in again to keep your changes."` Keep the leak-detection patterns and `wasProtocolLeak` flag exactly as-is (still scrub the internal header name); only the user-facing substitute string changes. This makes an authed-but-session-dropped save tell the truth instead of claiming demo mode.
3. Do NOT weaken auth, change cookie attributes, or alter the gateway. Host canonicalization (redirect) + copy hygiene only.

## Acceptance (truth-bounded)
1. Vitest `web/middleware.test.ts` (extend if present, else new file <500 LOC): a request with `nextUrl.hostname === "127.0.0.1"` returns a redirect whose `Location` host is `localhost`, same port/path/search, and does NOT 404 or emit a "demo mode" string; a `localhost` request is NOT redirected (no loop). A request carrying `__Host-session` on `localhost` keeps its session marker (existing behaviour unchanged).
2. New vitest `web/app/(setup)/onboarding/__tests__/_scrub-error.test.ts` (<500 LOC): an error containing `principal` / `X-Argo-Principal` scrubs to a message that does NOT contain `"demo mode"` AND contains a re-auth phrase (`/sign in again|session expired/i`), with `wasProtocolLeak === true`; a benign message passes through unchanged; empty input → generic-failure copy.
3. CHECKABLE verifier: `cd web && npx vitest run middleware _scrub-error` is green; `rg -n "demo mode" web/app/\(setup\)/onboarding/_scrub-error.ts` returns ZERO hits (the misleading copy is gone).

## On ship (same commit)
Note it closes chip `task_341a020b` (loopback-host session drop mislabelled as demo mode). No FEATURE_MATRIX row changes (bug-fix on existing onboarding surface); if a row is touched, update only `docs/build-plan/FEATURE_MATRIX.md` (the .md is the in-band ledger — do NOT regenerate the gitignored HTML, DEC-043).

## Effort
S (one middleware helper + one copy-string change + two focused vitest files; no gateway, no schema, no migration).
