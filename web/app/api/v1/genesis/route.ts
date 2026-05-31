import { NextResponse, type NextRequest } from "next/server";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";

/**
 * Unbuffered SSE proxy for `POST /api/v1/genesis`.
 *
 * WHY THIS EXISTS (the bug it fixes): the `/genesis` page streams the
 * teammate-generation events (`event: progress` … `event: vertical_ready`)
 * from the Rust gateway over `text/event-stream`. In dev the SPA (`:3000`)
 * and gateway (`:7400`) are on different ports, so browser fetches go
 * through Next.js's `rewrites()` proxy (`web/next.config.ts`,
 * `/api/v1/:path*` → gateway). That proxy runs the events through a
 * compression layer: when the browser advertises `Accept-Encoding: gzip`
 * (every real browser does), Next gzips the response and the gzip window
 * BUFFERS the stream — the browser's `fetch().body.getReader()` never sees
 * a complete `\n\n` SSE block, so the `/genesis` UI hangs forever on
 * "Generating…" and never reaches the role-contract review screen.
 * (Reproduce: `curl` the proxy WITHOUT `Accept-Encoding` streams cleanly;
 * WITH `Accept-Encoding: gzip` the proxy stamps `Content-Encoding: gzip`
 * and the decoded event lines never emerge.)
 *
 * A Route Handler at this EXACT path takes precedence over the
 * `/api/v1/:path*` rewrite for `/api/v1/genesis` only — every other
 * `/api/v1/*` path keeps using the rewrite. This handler proxies the POST
 * to the gateway and returns the upstream `ReadableStream` body DIRECTLY
 * (never `await`s/collects it), with no compression, so the events reach
 * the browser the instant the gateway emits them.
 *
 * The SSE client (`web/lib/api/genesis.ts`) is unchanged — it already
 * fetches the same-origin `/api/v1/genesis`, which now hits this handler
 * instead of the rewrite.
 */

// SSE is inherently dynamic and must never be statically optimized or
// cached. `force-dynamic` + node runtime keep the stream alive for the
// full ~20s generation window without an edge-function timeout.
export const dynamic = "force-dynamic";
export const runtime = "nodejs";
export const fetchCache = "force-no-store";
// Genesis generation streams for ~20s; give the route ample headroom over
// the platform default so a slow gate doesn't truncate the stream.
export const maxDuration = 120;

function gatewayBase(): string {
  // Route handlers run inside the Next.js process (no `window`), so they
  // must use the absolute gateway URL — resolved exactly the way the rest
  // of the SERVER-side code does (`lib/config/gateway-url.ts`).
  return resolveGatewayUrl();
}

/**
 * Forward only the headers the gateway's genesis handler reads. We
 * deliberately DO NOT forward `Accept-Encoding`: the gateway streams
 * plain `text/event-stream`, and if we let it negotiate gzip the upstream
 * body would arrive compressed and we'd have to buffer-then-decompress —
 * exactly the failure mode this handler exists to avoid.
 */
function forwardHeaders(req: NextRequest): Headers {
  const out = new Headers();
  out.set("Content-Type", req.headers.get("content-type") ?? "application/json");
  out.set("Accept", "text/event-stream");
  // Force identity so the upstream stays an unbuffered, line-oriented
  // event stream end to end.
  out.set("Accept-Encoding", "identity");

  // Tenant id is load-bearing: Genesis writes the generated vertical under
  // `.genesis/<tenant>/` and the follow-up hire reads the same tenant, so
  // the id MUST survive the proxy hop.
  const tenant = req.headers.get("x-argo-tenant-id");
  if (tenant) out.set("X-Argo-Tenant-Id", tenant);

  // Auth headers, when present (non-bypass / production path). Under the
  // demo-live auth-bypass the gateway accepts the request without these,
  // so they're optional — but forwarded verbatim when the browser sent
  // them so this handler works in authenticated mode too.
  const authz = req.headers.get("authorization");
  if (authz) out.set("Authorization", authz);
  const principal = req.headers.get("x-argo-principal");
  if (principal) out.set("X-Argo-Principal", principal);

  return out;
}

export async function POST(req: NextRequest): Promise<Response> {
  // Read the raw body once and forward it verbatim. The gateway validates
  // the JSON shape (`role_description`, optional `overwrite`/`persona`/SOP
  // fields); this proxy does no business logic.
  const body = await req.text();

  let upstream: Response;
  try {
    upstream = await fetch(`${gatewayBase()}/api/v1/genesis`, {
      method: "POST",
      headers: forwardHeaders(req),
      body,
      cache: "no-store",
      // Stream the upstream response incrementally instead of letting the
      // fetch implementation buffer it.
      // @ts-expect-error — `duplex` is required by undici for streaming
      // request/response bodies but is not yet in the lib.dom RequestInit.
      duplex: "half",
    });
  } catch {
    return NextResponse.json(
      {
        error: {
          code: "gateway_unreachable",
          message: "Couldn't reach the Genesis service.",
        },
      },
      { status: 502 },
    );
  }

  // Non-2xx (e.g. the gateway's 400 on a too-short role_description) — pass
  // the gateway's JSON error envelope through unchanged so the SPA's
  // `streamGenesis` surfaces a real message rather than a generic hang.
  if (!upstream.ok || !upstream.body) {
    const text = await upstream.text().catch(() => "");
    return new NextResponse(text || null, {
      status: upstream.status,
      headers: {
        "Content-Type":
          upstream.headers.get("content-type") ?? "application/json",
        "Cache-Control": "no-store",
      },
    });
  }

  // Hand the upstream ReadableStream straight back — NOT awaited/collected.
  // The explicit headers below stop Next's dev compression middleware from
  // re-buffering the stream:
  //  - `Content-Encoding: identity` — no gzip/br, so nothing accumulates a
  //    compression window before flushing.
  //  - `Cache-Control: no-cache, no-transform` — `no-transform` forbids any
  //    proxy (incl. Next) from re-encoding the body.
  //  - `X-Accel-Buffering: no` — disables nginx/proxy buffering in prod.
  //  - `Connection: keep-alive` — hold the socket open for the full stream.
  return new Response(upstream.body, {
    status: 200,
    headers: {
      "Content-Type": "text/event-stream; charset=utf-8",
      "Cache-Control": "no-cache, no-transform",
      "Content-Encoding": "identity",
      "X-Accel-Buffering": "no",
      Connection: "keep-alive",
    },
  });
}
