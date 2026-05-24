/**
 * Server-only helper that decodes the magic-link `__Host-session`
 * cookie set by `/api/auth/magic-link/verify`. The cookie carries an
 * HS256 JWT with `{ sub, email, tnt, role, iat, exp }`; we body-decode
 * (no signature verify) so the SPA can surface `role` for read-only
 * UI gates and propagate the JWT as `Authorization: Bearer`. The
 * gateway is the trust boundary — signature verification happens
 * there.
 *
 * Why this file does not use a module-level `import "server-only"`
 * even though the brief asks for it: the dynamic-import chain
 * `lib/api/client.ts → @/auth → ./lib/auth/session-cookie` is still
 * statically traced by Turbopack into Client Component SSR bundles
 * (verified 2026-05-22 against turbopack 15.x). A top-level
 * `import "server-only"` would trip Turbopack's poison-pill check
 * for the bundle that the dev-mode signin page tries to compile,
 * keeping the original 500 in place. Per the eval scenario's option
 * (b) — `docs/skycot/evals/scheduled/2026-05-22/spa-cookies-import-app-routes-500.md`
 * — the protection here is the **dynamic `next/headers` import inside
 * the function body**: client bundles can statically include this
 * file without crashing the compile, but the `next/headers` import
 * is never reached at runtime in a browser (no caller exists that's
 * not server-only). A unit test (`auth-config-no-server-imports.test.ts`)
 * still guards `web/auth.config.ts` against the original regression
 * pattern.
 */
export interface MagicLinkClaims {
  jwt: string;
  role?: string;
  /**
   * `X-Argo-Principal` value derived from the JWT body's `sub` or
   * `email` claim. Format: `human:<email>` so the gateway's
   * dev-bypass auth path scopes onboarding + workspace routes to a
   * real user identity rather than the `system:dev-bypass` fallback.
   * `undefined` when the JWT doesn't include either claim (legacy
   * cookies pre-demo-mode).
   */
  principal?: string;
}

export async function readMagicLinkClaims(): Promise<MagicLinkClaims | null> {
  if (typeof window !== "undefined") return null;
  try {
    const { cookies } = await import("next/headers");
    const store = await cookies();
    const cookie = store.get("__Host-session")?.value;
    if (!cookie) return null;
    const parts = cookie.split(".");
    if (parts.length !== 3) return { jwt: cookie };
    const payload = parts[1];
    const json = Buffer.from(
      payload.replace(/-/g, "+").replace(/_/g, "/"),
      "base64",
    ).toString("utf8");
    const claims = JSON.parse(json) as {
      role?: string;
      sub?: string;
      email?: string;
    };
    const emailish = typeof claims.email === "string" && claims.email.length > 0
      ? claims.email
      : typeof claims.sub === "string" && claims.sub.includes("@")
        ? claims.sub
        : null;
    const principal = emailish ? `human:${emailish}` : undefined;
    return { jwt: cookie, role: claims.role, principal };
  } catch {
    return null;
  }
}
