import type { NextAuthConfig } from "next-auth";
import Credentials from "next-auth/providers/credentials";

/**
 * NextAuth v5 config — single credentials provider that verifies a
 * gateway bearer token by hitting `/healthz` with it attached.
 *
 * Token verification is intentionally lightweight: the gateway's own
 * RBAC layer is the authority. We only confirm that the token is
 * accepted before we persist it into the session. The session is
 * stored as a JWT (no DB) and carries the bearer token forward to
 * the API client wrapper.
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
          process.env.NEXT_PUBLIC_GATEWAY_URL ?? "http://127.0.0.1:8787";

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
      return session;
    },
  },
} satisfies NextAuthConfig;

declare module "next-auth" {
  interface Session {
    gatewayToken?: string;
  }
  interface User {
    token?: string;
  }
}
