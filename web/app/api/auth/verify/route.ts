import { NextResponse, type NextRequest } from "next/server";
import { verifySchema } from "@/lib/auth/signup-schema";

const DEFAULT_GATEWAY = "http://127.0.0.1:8787";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

interface GatewayVerifyResponse {
  needs_onboarding?: boolean;
  tenant_id?: string | null;
  error?: { code: string; message: string };
}

export async function POST(req: NextRequest) {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json(
      { error: { code: "invalid_body", message: "Request body must be JSON." } },
      { status: 400 },
    );
  }
  const parsed = verifySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: { code: "invalid_token", message: "Token is missing or malformed." } },
      { status: 400 },
    );
  }

  let upstream: Response;
  try {
    upstream = await fetch(`${gatewayBase()}/api/v1/auth/verify`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ token: parsed.data.token }),
      cache: "no-store",
    });
  } catch {
    return NextResponse.json(
      { error: { code: "gateway_unreachable", message: "Couldn't reach SKYCOT." } },
      { status: 502 },
    );
  }
  const payload = (await upstream.json().catch(() => ({}))) as GatewayVerifyResponse;
  if (!upstream.ok) {
    return NextResponse.json(payload, { status: upstream.status || 401 });
  }
  const res = NextResponse.json({
    ok: true,
    needs_onboarding: payload.needs_onboarding ?? true,
    tenant_id: payload.tenant_id ?? null,
  });
  // Gateway mints `Set-Cookie: __Host-session=...; HttpOnly; Secure;
  // SameSite=Lax; Path=/; Max-Age=...` directly
  // (mission-magic-link-v1x-hardening). Forward it untouched so the
  // SPA never sees the JWT and the cookie attributes have a single
  // source of truth.
  const upstreamCookie = upstream.headers.get("set-cookie");
  if (upstreamCookie) {
    res.headers.set("set-cookie", upstreamCookie);
  }
  return res;
}
