import { NextResponse } from "next/server";
import { readMagicLinkClaims } from "@/lib/auth/session-cookie";

/**
 * `GET /api/principal` — surface the verified human principal (`human:<email>`)
 * derived from the `__Host-session` cookie set by the gateway's magic-link
 * verify.
 *
 * Why this exists: in the split topology (SPA on a separate origin proxying to
 * the gateway) login is via the gateway magic-link, so NextAuth holds no token
 * and `/api/auth/session` returns no `principal`. The browser API client
 * (`lib/api/client.ts`) needs the principal to stamp `X-Argo-Principal` on
 * gateway calls — without it every onboarding/billing POST 401s
 * ("X-Argo-Principal header must carry a verified human:<email>"). The
 * `__Host-session` cookie itself is HttpOnly + not proxied cross-origin, so the
 * client can't read it directly and the gateway can't derive it from the proxied
 * request; this server-only route reads it via `next/headers` and returns ONLY
 * the principal (never the JWT — no secret leak).
 */
export const dynamic = "force-dynamic";

export async function GET() {
  const claims = await readMagicLinkClaims();
  return NextResponse.json({ principal: claims?.principal ?? null });
}
