import type { NextAuthConfig } from "next-auth";
import Credentials from "next-auth/providers/credentials";
import { cookies } from "next/headers";

/**
 * NextAuth v5 config — single credentials provider that verifies a
 * gateway bearer token by hitting `/healthz` with it attached.
 *
 * Token verification is intentionally lightweight: the gateway's own
 * RBAC layer is the authority. We only confirm that the token is
 * accepted before we persist it into the session. The session is
 * stored as a JWT (no DB) and carries the bearer token forward to
 * the API client wrapper.
 *
 * The magic-link flow sets a `__Host-session` HttpOnly cookie carrying
 * an HS256 JWT with `{ sub, email, tnt, role, iat, exp }`. The session
 * callback below decodes that cookie server-side so:
 *
 *   - `session.gatewayToken` = the JWT (sent as `Authorization: Bearer`
 *     so the gateway can verify it + read the embedded role).
 *   - `session.role` = the role claim, populated for `useViewerPolicy`
 *     and other client-side gates (read-only display only — the
 *     gateway is the authority).
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
          const res = await fetch(`${gatewayUrl}/healthz`, {
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
    async session({ session, token }) {
      const gw = (token as { gatewayToken?: unknown }).gatewayToken;
      if (typeof gw === "string") session.gatewayToken = gw;

      // Surface the magic-link JWT + role into the session shape used
      // by the API client and `useViewerPolicy`. Decode is body-only
      // (no signature verify); the gateway is the trust boundary.
      const magicLink = await readMagicLinkClaims();
      if (magicLink) {
        if (magicLink.jwt) session.gatewayToken = magicLink.jwt;
        if (magicLink.role) session.role = magicLink.role;
      }
      return session;
    },
  },
} satisfies NextAuthConfig;

interface MagicLinkClaims {
  jwt: string;
  role?: string;
}

async function readMagicLinkClaims(): Promise<MagicLinkClaims | null> {
  try {
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
    const claims = JSON.parse(json) as { role?: string };
    return { jwt: cookie, role: claims.role };
  } catch {
    return null;
  }
}

declare module "next-auth" {
  interface Session {
    gatewayToken?: string;
    role?: string;
  }
  interface User {
    token?: string;
  }
}
