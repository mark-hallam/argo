import type { NextAuthConfig } from "next-auth";
import Credentials from "next-auth/providers/credentials";

/**
 * NextAuth v5 config — edge-safe, client-bundle-safe surface.
 *
 * Anything in this file must remain importable from Client Components
 * and the Edge runtime, which means no server-only APIs from
 * `next/headers` and no other request-scoped Node APIs. Server-only
 * callbacks (those that need request cookies, request headers, or the
 * DB) live in `web/auth.ts`, which wraps this config when it
 * constructs the NextAuth instance. See the unit test at
 * `web/__tests__/auth-config-no-server-imports.test.ts` which enforces
 * this boundary.
 *
 * The credentials provider verifies a gateway bearer token by hitting
 * `/healthz` with it attached. Token verification is intentionally
 * lightweight: the gateway's own RBAC layer is the authority. We only
 * confirm that the token is accepted before persisting it into the
 * session. The session is stored as a JWT (no DB) and carries the
 * bearer token forward to the API client wrapper.
 *
 * Session enrichment from the magic-link `__Host-session` cookie
 * (decoded server-side in `web/auth.ts`):
 *   - `session.gatewayToken` = the JWT (sent as `Authorization: Bearer`
 *     so the gateway can verify it + read the embedded role).
 *   - `session.role` = the role claim, populated for `useViewerPolicy`
 *     and other client-side gates (read-only display only — the
 *     gateway is the trust boundary).
 */
export const authConfig = {
  session: { strategy: "jwt" },
  pages: { signIn: "/auth/signin" },
  providers: [
    Credentials({
      id: "gateway-bearer",
      name: "Gateway bearer token",
      credentials: {
        token: { label: "Bearer token", type: "password" },
      },
      async authorize(credentials) {
        const token =
          typeof credentials?.token === "string" ? credentials.token.trim() : "";
        if (!token) return null;

        const gatewayUrl =
          process.env.NEXT_PUBLIC_GATEWAY_URL ?? "http://127.0.0.1:7400";

        try {
          // `/readyz` is the canonical gateway health probe (the old
          // `/healthz` route was removed in the 2026-Q2 health-router
          // consolidation). With the gateway auth bypass enabled
          // (for example `gateway.auth_bypass=true`), any presented
          // Bearer is accepted; in production the bearer-token gate validates
          // against the configured secret.
          const res = await fetch(`${gatewayUrl}/readyz`, {
            headers: { Authorization: `Bearer ${token}` },
            cache: "no-store",
          });
          if (!res.ok) return null;
        } catch {
          return null;
        }

        return { id: "gateway-user", token };
      },
    }),
  ],
  callbacks: {
    async jwt({ token, user }) {
      if (user && "token" in user && typeof user.token === "string") {
        (token as { gatewayToken?: string }).gatewayToken = user.token;
      }
      return token;
    },
  },
} satisfies NextAuthConfig;

declare module "next-auth" {
  interface Session {
    gatewayToken?: string;
    role?: string;
    /**
     * Stable `X-Argo-Principal` value (e.g. `human:demo@throwaway.test`).
     * Populated from the `__Host-session` JWT's `sub`/`email` claim
     * by the server-side callback in `web/auth.ts`. The SPA's API
     * client reads it from `/api/auth/session` and stamps it as the
     * `X-Argo-Principal` header on gateway calls so the dev-bypass
     * auth path can scope the principal correctly. Without this,
     * onboarding routes reject the request as
     * `system:dev-bypass` rather than `human:<email>`.
     */
    principal?: string;
  }
  interface User {
    token?: string;
  }
}
