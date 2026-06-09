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

  // IDENTITY — forward the session + CSRF cookies (R4-F1 root fix).
  //
  // The gateway resolves the tenant a generated vertical is written under from
  // the `__Host-session` cookie (`resolve_tenant_for_request` → `session_tenant_id`),
  // EXACTLY as the follow-up hire (`POST /api/v1/workers`) does. The hire reaches
  // the gateway through Next's generic `/api/v1/:path*` rewrite, which forwards
  // ALL request headers — including `Cookie`. THIS bespoke handler (which shadows
  // that rewrite for `/api/v1/genesis` only, to keep the SSE stream unbuffered)
  // previously hand-picked a header subset and DROPPED `Cookie` — so the generate
  // ran as the cwd/anonymous tenant while the hire ran as the signed-in tenant.
  // The two then searched different `.genesis/<tenant>/` segments and the hire
  // failed `400 unknown vertical`. Forwarding the cookie makes the write segment
  // and the read segment the SAME signed-in tenant by construction.
  //
  // Forwarding `__Host-session` makes this a cookie-authenticated MUTATING POST,
  // which the gateway's `middleware_csrf` double-submit gate enforces — so the
  // matching `X-CSRF-Token` header (which the SSE client now sends, like every
  // other SPA mutation) must ride along too, or the gateway returns 403
  // csrf_missing. The anonymous demo path is unchanged: with no `__Host-session`
  // cookie the CSRF gate skips and both generate + hire fall to the cwd tenant.
  const cookie = req.headers.get("cookie");
  if (cookie) out.set("Cookie", cookie);
  const csrf = req.headers.get("x-csrf-token");
  if (csrf) out.set("X-CSRF-Token", csrf);

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

  // Re-emit the upstream stream through OUR OWN ReadableStream rather than
  // handing `upstream.body` straight to `new Response(...)`.
  //
  // WHY (the second half of the bug): the Rust gateway sends all its SSE
  // events (`progress` … `vertical_ready`) and then ABRUPTLY closes the TCP
  // socket — it does not send a graceful stream terminator. If we pass
  // `upstream.body` directly, Node/undici's internal pipe sees that close
  // as `SocketError: other side closed` (`UND_ERR_SOCKET`) and throws
  // `failed to pipe response`, which Next turns into a 500 AFTER the bytes
  // have already streamed. The browser's reader then errors and the SPA's
  // `streamGenesis().catch` fires a spurious "network error" — even though
  // every event (incl. `vertical_ready`) already reached the client and the
  // role-contract review should render. (Confirmed in the dev log:
  // `POST /api/v1/genesis 500 in ~24000ms` with `remotePort: 7400`,
  // `bytesRead: 4197` — i.e. the full payload was read first.)
  //
  // By pumping the reader ourselves we treat "upstream closed" — whether a
  // clean `done` OR a post-data socket error — as a NORMAL end of stream
  // (`controller.close()`), so the response completes 200 and the browser's
  // reader sees a clean end after the last event.
  const reader = upstream.body.getReader();
  const passthrough = new ReadableStream<Uint8Array>({
    async pull(controller) {
      try {
        const { value, done } = await reader.read();
        if (done) {
          controller.close();
          return;
        }
        if (value) controller.enqueue(value);
      } catch {
        // Upstream half-closed the socket after delivering its events.
        // The SSE payload (incl. `vertical_ready`) has already been
        // forwarded, so this is a normal end-of-stream from the browser's
        // point of view — close cleanly instead of erroring the response.
        controller.close();
      }
    },
    cancel(reason) {
      // Client navigated away / aborted — release the upstream reader.
      void reader.cancel(reason).catch(() => {});
    },
  });

  // The explicit headers below stop Next's dev compression middleware from
  // re-buffering the stream:
  //  - `Content-Encoding: identity` — no gzip/br, so nothing accumulates a
  //    compression window before flushing.
  //  - `Cache-Control: no-cache, no-transform` — `no-transform` forbids any
  //    proxy (incl. Next) from re-encoding the body.
  //  - `X-Accel-Buffering: no` — disables nginx/proxy buffering in prod.
  //  - `Connection: keep-alive` — hold the socket open for the full stream.
  return new Response(passthrough, {
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
