# fix-loopback-canonicalize-host-header-runtime

Authored against: origin/main cb7d4cfb23d993d758d8d61c987d3b3bd73bee71 on 2026-06-10
Validate before claim: bash scripts/check-brief-freshness.sh fix-loopback-canonicalize-host-header-runtime

## Goal

Make the loopback-host canonicalization shipped in #2758 actually fire at
runtime. Today `canonicalLoopbackRedirect` ([web/middleware.ts:163](../../../web/middleware.ts))
conditions on `req.nextUrl.hostname === "127.0.0.1"` — but in the live
Next.js server `nextUrl.hostname` never reads `127.0.0.1`; it normalizes to
the derived origin (`localhost`). The unit test had to stub `nextUrl` for
exactly this reason (see the comment block in `web/middleware.test.ts` —
"NextRequest's nextUrl normalizes a 127.0.0.1 input back to localhost"),
which made the test green while the runtime branch is dead.

## Why now (conductor browser-verify evidence, DEC-044, 2026-06-10)

Probed live on the turbopack dev server with the merged code on disk
(`canonicalLoopbackRedirect` present at middleware.ts:163, wired at :207):

```
curl -D - http://127.0.0.1:3000/team
  → HTTP/1.1 307, location: /auth/signin?returnTo=%2Fteam     (RELATIVE — stays on 127.0.0.1)
curl -D - -H "Host: 127.0.0.1:3000" http://127.0.0.1:3000/workers
  → HTTP/1.1 307, location: /auth/signin?returnTo=%2Fworkers  (same)
```

Expected: `307 → http://localhost:3000/team`. The auth gate fires instead,
meaning the loopback branch returned `null` → the `__Host-session` cookie
drop on `127.0.0.1` (the original bug) is still live. Honest caveat: probed
on the dev profile; the fix below is correct for both dev and `next start`.

## Fix

Condition on the request `Host` HEADER, not `nextUrl.hostname`:

```ts
export function canonicalLoopbackRedirect(req: NextRequest): NextResponse | null {
  const host = req.headers.get("host") ?? "";
  if (!host.startsWith("127.0.0.1")) return null;
  const target = new URL(req.nextUrl.href);
  target.hostname = "localhost";
  // keep the port from the Host header if present (nextUrl may differ)
  const port = host.split(":")[1];
  if (port) target.port = port;
  return NextResponse.redirect(target, 307);
}
```

Keep the existing call site (runs before any cookie read). Update the unit
tests to set the `host` header on a real `NextRequest` (no `nextUrl` stub
needed any more — the header is faithfully readable in the vitest harness,
which kills the test/runtime divergence this bug rode in on).

## May touch

- `web/middleware.ts` (the helper only)
- `web/middleware.test.ts` (re-point the loopback describe block at the header)

## Acceptance / verifier

1. `pnpm --dir web vitest run middleware.test.ts` green; the loopback tests
   construct `NextRequest` with `headers: { host: "127.0.0.1:3000" }` and do
   NOT stub `nextUrl`.
2. Runtime check (the checkable verifier, run with any dev server up):
   `curl -s -o /dev/null -D - http://127.0.0.1:3000/team | grep -i location`
   prints an ABSOLUTE `http://localhost:3000/...` location. Paste the curl
   output in the PR body.
3. Control: same curl against `http://localhost:3000/team` still returns the
   auth redirect (no loop).

## Size / owner

~15 LOC + test updates. Owner: claude. Web-only fast lane (DEC-006).

## 🔴 BLOCKED 2026-06-10 — wrong premise + may-touch too tight (claude worker)

The prescribed middleware fix was implemented and **proven non-functional at
runtime**, and would REGRESS (introduce an infinite redirect loop). Do NOT
re-attempt the middleware approach. Evidence, gathered live on a `next dev`
server (Next.js 15.5.18) running the candidate code:

1. **The Host-header condition DOES fire** — `req.headers.get("host")` reads
   `127.0.0.1:3001` faithfully at runtime, so the loopback branch now runs
   (the auth gate no longer fires for a `127.0.0.1` request). That part of the
   brief is correct.
2. **But Next.js relativizes the redirect Location**, defeating the host hop.
   Next collapses a redirect `Location` to a relative path **iff** its
   `host:port` equals the request's *normalized* origin. The dev/prod server
   normalizes the `127.0.0.1` request's `nextUrl` to `localhost:3001` (the very
   premise this brief relied on), so a redirect to `localhost:3001` (our goal)
   is emitted as `/team`. Proven by probing three targets from the same
   handler:
   - `http://localhost:3001/team`  → `location: /team`            (RELATIVIZED)
   - `http://example.com/team`      → `location: http://example.com/team`     (absolute)
   - `http://localhost:59999/team`  → `location: http://localhost:59999/team` (absolute)
   Forcing it via `res.headers.set("location", target.href)` and via a raw
   `new NextResponse(null,{status:307,headers:{location:target.href}})` BOTH
   still came out relative — the relativization is in the edge/middleware
   serializer, not in `NextResponse.redirect`, so it is unavoidable from
   middleware.
3. **Net effect = infinite loop / strictly worse.** The browser requested
   `127.0.0.1:3001`; `location: /team` resolves against that real host → stays
   on `127.0.0.1` → middleware fires the loopback branch again → `/team` →
   loop. The `__Host-session` cookie is still dropped. Acceptance verifier #2
   (an ABSOLUTE `http://localhost:3000/...` location) is therefore
   **unachievable from Next middleware** for the same-port `localhost` target.

**Correct fix is OUTSIDE the may-touch list** (`middleware.ts` only):
- **Option A (recommended):** a `next.config.ts` `redirects()` entry with
  `has: [{ type: "host", value: "127.0.0.1" }]` and an absolute `destination`
  on `localhost` — Next does NOT relativize config-level host redirects (they
  exist for exactly this apex→www / host-canonicalization case).
- **Option B:** accept that `__Host-` cookies are host-bound and simply
  document/standardize on `localhost` (never `127.0.0.1`) for the dev/demo
  origin; drop the canonicalization attempt entirely.

Either path needs a new brief with `next.config.ts` (and/or docs) in
may-touch. Re-author against current `origin/main`; this brief's middleware-only
scope is a dead end.
