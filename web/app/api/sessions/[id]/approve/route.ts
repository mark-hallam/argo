import { NextResponse, type NextRequest } from "next/server";
import { auth } from "@/auth";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";

/**
 * Thin proxy to `POST /sessions/{id}/approve` on the gateway — the
 * escalation-grant route shipped in #665 (see
 * `crates/argo-cli/src/handlers/gateway/routes_session_approval.rs`;
 * the gateway mounts it WITHOUT the `/api/v1` prefix — verified
 * against `routes_dispatch_post_paths.rs::dispatch_session_path`).
 *
 * - Reads the NextAuth session, forwards the bearer token as
 *   `Authorization: Bearer <token>`.
 * - Forwards the JSON body verbatim. The gateway accepts
 *   `{ tool: string, note?: string, one_shot?: boolean }` and answers
 *   `{ ok, grant_id, session_id, tool, one_shot }`.
 * - Passes through the gateway's status + body so the client surface
 *   can read the grant envelope / error envelope unchanged.
 *
 * Mirrors `web/app/api/workers/[id]/run/route.ts` — no business
 * logic here; capability gating lives on the gateway. Consumed by
 * `lib/api/session-approvals.ts` for the in-SPA probation tool
 * approval flow (day-1 unblock FIX 1).
 */

function gatewayBase(): string {
  return resolveGatewayUrl();
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
      `${gatewayBase()}/sessions/${encodeURIComponent(id)}/approve`,
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
