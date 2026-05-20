import { NextResponse, type NextRequest } from "next/server";

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";
const ALLOWED_PROVIDERS = new Set(["google", "github", "microsoft"]);

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

interface GatewayCallbackResponse {
  needs_onboarding?: boolean;
  tenant_id?: string | null;
  error?: { code: string; message: string };
}

/**
 * OAuth callback proxy.
 *
 * Forwards `{ code, state }` to the gateway and re-emits the upstream
 * Set-Cookie so the session cookie lands on the SPA origin. Mirrors
 * `/api/auth/verify` exactly — the SPA never sees the JWT and cookie
 * attributes have a single source of truth in the gateway.
 *
 * We do NOT forward `X-Forwarded-For` from the incoming request (same
 * P0 rationale as signup): an attacker could rotate the header to
 * bypass the gateway's connecting-peer rate limit. The gateway
 * rate-limits by (provider, peer-IP) instead.
 */
export async function POST(
  req: NextRequest,
  context: { params: Promise<{ provider: string }> },
) {
  const { provider } = await context.params;
  if (!ALLOWED_PROVIDERS.has(provider)) {
    return NextResponse.json(
      { error: { code: "unknown_provider", message: "Unknown SSO provider." } },
      { status: 400 },
    );
  }

  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json(
      { error: { code: "invalid_body", message: "Request body must be JSON." } },
      { status: 400 },
    );
  }
  const parsed = parseCallbackBody(body);
  if (!parsed) {
    return NextResponse.json(
      {
        error: {
          code: "invalid_callback",
          message: "Missing `code` or `state` from provider callback.",
        },
      },
      { status: 400 },
    );
  }

  let upstream: Response;
  try {
    upstream = await fetch(
      `${gatewayBase()}/api/v1/auth/oauth/${provider}/callback`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json", Accept: "application/json" },
        body: JSON.stringify({ code: parsed.code, state: parsed.state }),
        cache: "no-store",
      },
    );
  } catch {
    return NextResponse.json(
      { error: { code: "gateway_unreachable", message: "Couldn't reach SKYCOT." } },
      { status: 502 },
    );
  }
  const payload = (await upstream.json().catch(() => ({}))) as GatewayCallbackResponse;
  if (!upstream.ok) {
    return NextResponse.json(payload, { status: upstream.status || 401 });
  }
  const res = NextResponse.json({
    ok: true,
    needs_onboarding: payload.needs_onboarding ?? false,
    tenant_id: payload.tenant_id ?? null,
  });
  const upstreamCookie = upstream.headers.get("set-cookie");
  if (upstreamCookie) {
    res.headers.set("set-cookie", upstreamCookie);
  }
  return res;
}

interface CallbackBody {
  code: string;
  state: string;
}

function parseCallbackBody(body: unknown): CallbackBody | null {
  if (typeof body !== "object" || body === null) return null;
  const obj = body as Record<string, unknown>;
  if (typeof obj.code !== "string" || obj.code.length === 0) return null;
  if (typeof obj.state !== "string" || obj.state.length === 0) return null;
  return { code: obj.code, state: obj.state };
}
