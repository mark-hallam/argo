import { NextResponse, type NextRequest } from "next/server";
import { signupSchema } from "@/lib/auth/signup-schema";

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

// SECURITY (P0 fix, 2026-05-20): We do NOT forward `X-Forwarded-For` from the
// client to the gateway. The previous behaviour pulled the header from the
// incoming request and re-sent it as the apparent rate-limit key, which let an
// attacker rotate `X-Forwarded-For` per request to email-bomb arbitrary
// addresses through the magic-link signup endpoint. The gateway rate-limits
// by the email + the connecting peer instead, which is what the audit asked
// for. Do not re-introduce header forwarding without per-email throttling
// landing in the gateway first. See docs/skycot/audit/security/2026-05-20.

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
  const parsed = signupSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      {
        error: {
          code: "invalid_email",
          message: parsed.error.issues[0]?.message ?? "Invalid email",
        },
      },
      { status: 400 },
    );
  }

  try {
    const headers: HeadersInit = {
      "Content-Type": "application/json",
      Accept: "application/json",
    };

    const upstream = await fetch(`${gatewayBase()}/api/v1/auth/signup`, {
      method: "POST",
      headers,
      body: JSON.stringify({ email: parsed.data.email }),
      cache: "no-store",
    });
    const text = await upstream.text();
    const payload = text ? safeJson(text) : { status: "sent" };
    return NextResponse.json(payload, { status: upstream.status });
  } catch {
    return NextResponse.json(
      {
        error: {
          code: "gateway_unreachable",
          message: "Couldn't reach SKYCOT. Try again shortly.",
        },
      },
      { status: 502 },
    );
  }
}

function safeJson(text: string): unknown {
  try {
    return JSON.parse(text);
  } catch {
    return { raw: text };
  }
}
