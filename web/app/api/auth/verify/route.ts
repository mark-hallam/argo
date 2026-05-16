import { NextResponse, type NextRequest } from "next/server";
import { verifySchema } from "@/lib/auth/signup-schema";

const DEFAULT_GATEWAY = "http://127.0.0.1:8787";
const COOKIE_NAME = "skycot_session";
const COOKIE_MAX_AGE_SECS = 7 * 24 * 60 * 60;

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

interface GatewayVerifyResponse {
  jwt?: string;
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
  if (!upstream.ok || !payload.jwt) {
    return NextResponse.json(payload, { status: upstream.status || 401 });
  }
  const res = NextResponse.json({
    ok: true,
    needs_onboarding: payload.needs_onboarding ?? true,
    tenant_id: payload.tenant_id ?? null,
  });
  res.cookies.set(COOKIE_NAME, payload.jwt, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: COOKIE_MAX_AGE_SECS,
  });
  return res;
}
