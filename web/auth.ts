import NextAuth from "next-auth";
import { authConfig } from "./auth.config";
import { readMagicLinkClaims } from "./lib/auth/session-cookie";

/**
 * NextAuth v5 server entrypoint. Wraps the edge-safe `authConfig`
 * (providers, pages, JWT callback) and adds the session callback
 * that enriches the session with the magic-link cookie claims via
 * the helper in `web/lib/auth/session-cookie.ts`. That helper keeps
 * its `next/headers` access behind a dynamic `await import(...)` so
 * the static-import graph Turbopack traces — including the
 * `lib/api/client.ts → @/auth` chain — never reaches `next/headers`
 * from a client-bundled file. See the
 * `auth-config-no-server-imports.test.ts` guards.
 */
export const { handlers, auth, signIn, signOut } = NextAuth({
  ...authConfig,
  callbacks: {
    ...authConfig.callbacks,
    async session({ session, token }) {
      const gw = (token as { gatewayToken?: unknown }).gatewayToken;
      if (typeof gw === "string") session.gatewayToken = gw;

      const magicLink = await readMagicLinkClaims();
      if (magicLink) {
        if (magicLink.jwt) session.gatewayToken = magicLink.jwt;
        if (magicLink.role) session.role = magicLink.role;
        if (magicLink.principal) session.principal = magicLink.principal;
      }
      return session;
    },
  },
});
