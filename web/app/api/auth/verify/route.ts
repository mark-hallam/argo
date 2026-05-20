import { NextResponse, type NextRequest } from "next/server";
import { verifySchema } from "@/lib/auth/signup-schema";

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

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
  // Gateway mints two `Set-Cookie` headers — `__Host-session`
  // (HttpOnly, the JWT) and `__Host-csrf` (NOT HttpOnly, the CSRF
  // token = JWT `jti`). Forward both untouched via `getSetCookie()`
  // because `headers.get("set-cookie")` returns only the first
  // value and joining set-cookie headers with `, ` is unsafe
  // (cookie values can contain commas). (Stream
  // `claude-rebase-pr-1835-cookies`.)
  for (const cookie of upstream.headers.getSetCookie()) {
    res.headers.append("set-cookie", cookie);
  }
  return res;
}
