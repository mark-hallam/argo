# ops-marketing-pages-smoke-test-post-2242-env-rename

**Owner**: claude or codex.
**Authored against**: origin/main `bec35dd45` on `2026-05-25`.
**Validate before claim**: `bash scripts/check-brief-freshness.sh ops-marketing-pages-smoke-test-post-2242-env-rename`.

🟢 **P2 — v1.0 launch remaining item (f) from CLAUDE.md "Remaining for v1.0 launch" list**

CLAUDE.md item (f): "agent (post-relight): curl-check landing/marketing pages after #2242 `NEXT_PUBLIC_` env-var rename — must not 500 (per memory `styled_jsx_requires_use_client`)".

Memory `feedback_styled_jsx_requires_use_client` documents a 2026-05-20 wave-2 trap: pages with `<style jsx>` blocks return HTTP 500 when `"use client"` is removed. PR #2242 renamed the browser-visible env var; this brief verifies no marketing page regressed during that rename or any subsequent edit.

## Scope

1. **Start dev server** —
   - Confirm `.env.local` has `NEXT_PUBLIC_ARGO_GATEWAY_AUTH_OVERRIDE=mock` (or whatever the post-#2242 var name is — verify by `rg 'NEXT_PUBLIC_ARGO' web/lib web/app` and reading the actual rename in PR #2242's diff).
   - `cd web && npm install && npm run dev` (run in background, capture PID).
   - Wait for the server-ready log line (look for `"Ready in"` or `"Local:"` line; or poll `curl -s http://localhost:3000/ -o /dev/null -w '%{http_code}'` until non-000).
   - If the dev server depends on gateway being up (port 8088), check `ops-gateway-restart-after-2026-05-25-relight` brief first — but most pages should render without the gateway, just with mock data.

2. **Enumerate marketing pages**:
   - `find web/app -name 'page.tsx' \( -path '*/landing/*' -o -path '*/customers/*' -o -path '*/compare/*' -o -path '*/onboarding/*' -o -path '*/skills/*' -o -path '*/marketing/*' \)`
   - Also include the root `web/app/page.tsx` (the home page).
   - Build a list of routes from the file paths (e.g. `web/app/landing/page.tsx` → `/landing`).

3. **curl each route**:
   ```bash
   for r in / /landing /customers /compare /onboarding /skills <add others as found>; do
     code=$(curl -s -o /dev/null -w '%{http_code}' "http://localhost:3000$r")
     echo "$r : $code"
   done
   ```
   Save the output as a fenced block for the PR body.

4. **For any 500**:
   - Grep the dev-server log for the error stack: `node` will dump a clear styled-jsx warning if `"use client"` is missing.
   - Cross-reference against memory `feedback_styled_jsx_requires_use_client` — fix is usually to add `"use client";` as the literal first line of the offending `page.tsx`.
   - Apply the fix, re-curl, confirm 200.

5. **For any 404**: not a regression — possibly the route doesn't exist; remove from the matrix and note in PR body.

6. **Stop dev server cleanly** — `kill $DEV_PID` and confirm port 3000 freed.

## Verification status — 2026-05-25 codex run

Post-#2242 smoke run completed with
`NEXT_PUBLIC_ARGO_GATEWAY_AUTH_BYPASS=1 pnpm --dir web dev`.
All enumerated routes avoided HTTP 500; final status after redirects
was 200 for every route. `/onboarding` intentionally returns a raw
307 to `/onboarding/welcome`, whose final status is 200.

```text
/                                      raw=200 final=200
/landing                               raw=200 final=200
/customers                             raw=200 final=200
/customers/northwind-commerce          raw=200 final=200
/customers/beacon-health-ops           raw=200 final=200
/customers/maple-finance               raw=200 final=200
/customers/apex-recruiting             raw=200 final=200
/customers/skyline-engineering         raw=200 final=200
/customers/polaris-marketing           raw=200 final=200
/compare                               raw=200 final=200
/onboarding                            raw=307 final=200
/onboarding/welcome                    raw=200 final=200
/onboarding/workspace                  raw=200 final=200
/onboarding/pick-vertical              raw=200 final=200
/onboarding/connect-provider           raw=200 final=200
/onboarding/first-hire                 raw=200 final=200
/skills                                raw=200 final=200
/for/marketing                         raw=200 final=200
```

Captured dev-server output contained no `styled-jsx` warnings.

## Acceptance criteria

- Every enumerated marketing page returns HTTP 200 (not 500).
- PR body has the curl matrix as a fenced block (route + code).
- If any page 500'd: PR INCLUDES the fix commit (add `"use client"` directive) AND the post-fix curl matrix showing green.
- `dev-server.log` does NOT contain any "styled-jsx" warning post-fix.

## May-touch

- `web/app/**/page.tsx` — ONLY if a 500 is found and the fix is well-understood (add `"use client";` literal at top)
- This brief file (status updates)

## Out of scope

- Migrating styled-jsx → CSS modules / Tailwind v4 (the proper long-term fix; flagged but separate brief)
- Touching server-side routes
- Touching gateway (separate brief: `ops-gateway-restart-after-2026-05-25-relight`)
- Touching `.env.local` itself (only read it; if it's misconfigured, document in PR body)

## Effort

~15-30 minutes if all pages green; +30-60 min per 500 found-and-fixed.
