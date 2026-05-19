import { NextResponse, type NextRequest } from "next/server";
import { auth } from "@/auth";

/**
 * Thin proxy to `POST /api/v1/workers/{id}/unfreeze` on the gateway.
 *
 * Mirrors the freeze route — forwards the session bearer token,
 * passes through the upstream status + body. Unfreeze is the safe
 * direction, so the gateway accepts an empty body.
 */

const DEFAULT_GATEWAY = "http://127.0.0.1:8787";

function gatewayBase(): string {
  return process.env.NEXT_PUBLIC_GATEWAY_URL ?? DEFAULT_GATEWAY;
}

export async function POST(
  _req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const session = await auth();

  const headers: Record<string, string> = { Accept: "application/json" };
  if (session?.gatewayToken) {
    headers.Authorization = `Bearer ${session.gatewayToken}`;
  }

  try {
    const upstream = await fetch(
      `${gatewayBase()}/api/v1/workers/${encodeURIComponent(id)}/unfreeze`,
      { method: "POST", headers, cache: "no-store" },
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
