import { NextResponse, type NextRequest } from "next/server";
import { signupSchema } from "@/lib/auth/signup-schema";

const DEFAULT_GATEWAY = "http://127.0.0.1:7400";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

function clientIp(req: NextRequest): string {
  const fwd = req.headers.get("x-forwarded-for");
  if (fwd) return fwd.split(",")[0]?.trim() ?? "";
  return req.headers.get("x-real-ip") ?? "";
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
    const ip = clientIp(req);
    if (ip) (headers as Record<string, string>)["X-Forwarded-For"] = ip;

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
