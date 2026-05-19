import { NextResponse, type NextRequest } from "next/server";
import { auth } from "@/auth";

/**
 * Thin proxy to `POST /api/v1/workers/{id}/freeze` on the gateway.
 *
 * - Reads the NextAuth session, forwards the bearer token as
 *   `Authorization: Bearer <token>`.
 * - Forwards the JSON body verbatim (the only field the gateway
 *   reads is `reason`).
 * - Passes through the gateway's status + body so the client surface
 *   can read the freeze record / error envelope unchanged.
 *
 * Capability gating lives on the gateway. This proxy does no
 * business logic.
 */

const DEFAULT_GATEWAY = "http://127.0.0.1:8787";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const session = await auth();
  const body = await req.text();

  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "application/json",
  };
  if (session?.gatewayToken) {
    headers.Authorization = `Bearer ${session.gatewayToken}`;
  }

  try {
    const upstream = await fetch(
      `${gatewayBase()}/api/v1/workers/${encodeURIComponent(id)}/freeze`,
      { method: "POST", headers, body, cache: "no-store" },
    );
    const text = await upstream.text();
    return new NextResponse(text, {
      status: upstream.status,
      headers: { "Content-Type": "application/json" },
    });
  } catch {
    return NextResponse.json(
      { error: { code: "gateway_unreachable", message: "Couldn't reach the gateway." } },
      { status: 502 },
    );
  }
}
