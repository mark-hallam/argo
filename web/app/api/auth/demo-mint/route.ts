/**
 * Same-origin proxy for the gateway's `/api/v1/demo/mint-principal`
 * endpoint. The SPA calls THIS route (not the gateway directly) so
 * the response can land the gateway-style auth cookies
 * (`__Host-session` + `__Host-csrf`) on the `localhost:3000` origin —
 * which is what NextAuth's session callback reads to populate
 * `session.gatewayToken` and `session.principal`.
 *
 * Pre-fix history: the signup page called the gateway at
 * `localhost:7400` directly, got back the JWT in JSON, then discarded
 * it on `router.replace("/onboarding/welcome")`. Every subsequent
 * gateway call from the SPA fell through to the dev-bypass
 * `system:dev-bypass` principal, which the onboarding routes reject
 * with `X-Argo-Principal header must carry a verified 'human:<email>'
 * value`. The 2026-05-24 demo SWOT logged this as the P0 blocker for
 * the v1.0 demo recording.
 *
 * Mirrors `web/app/api/auth/verify/route.ts` (the magic-link verify
 * path) in shape; the difference is that the gateway's demo-mint
 * endpoint returns the JWT in its body rather than setting cookies
 * upstream, so this route does the cookie-set work itself.
 */
import { NextResponse, type NextRequest } from "next/server";
import { resolveGatewayUrl } from "@/lib/config/gateway-url";

// Resolution order: server-only `GATEWAY_URL` first, then the
// canonical public gateway URL (which itself honours
// `NEXT_PUBLIC_GATEWAY_URL` → `NEXT_PUBLIC_API_URL` → documented dev
// port). `GATEWAY_URL` is the escape hatch for split deployments
// where the SPA's browser-visible gateway URL differs from the URL
// the SPA server reaches the gateway on (e.g. internal DNS /
// sidecar / localhost-only socket). Keeping it explicit avoids the
// 2026-05-26 F6 failure mode where a stale `NEXT_PUBLIC_GATEWAY_URL`
// in the SPA's environment masked an otherwise-healthy gateway.
function gatewayBase(): string {
  return process.env.GATEWAY_URL ?? resolveGatewayUrl();
}

// Server-side health probe used to disambiguate "gateway is down" from
// "demo-mint route 5xx'd". Called only on a fetch throw to keep the
// happy path one round-trip. Short timeout because we're racing the
// human's perception of latency — the user already waited for the
// first fetch to fail, so the probe must add less than ~200ms to be
// invisible.
async function probeGatewayHealth(base: string): Promise<boolean> {
  try {
    const ctrl = new AbortController();
    const t = setTimeout(() => ctrl.abort(), 200);
    const res = await fetch(`${base}/health`, {
      method: "GET",
      cache: "no-store",
      signal: ctrl.signal,
    });
    clearTimeout(t);
    return res.ok;
  } catch {
    return false;
  }
}

interface MintBody {
  email?: unknown;
}

interface GatewayMintOk {
  ok: boolean;
  principal: string;
  session_token: string;
}

interface GatewayError {
  error?: string | { code?: string; message?: string };
}

interface GatewayTeamList {
  workers?: unknown[];
  total_count?: number;
}

interface GatewayVerticalsList {
  verticals?: unknown[];
}

/**
 * Probe the gateway to decide whether the just-minted tenant has
 * already completed onboarding. The tenant is "onboarded" when it
 * has at least one worker AND at least one installed vertical
 * (matches the runbook's Scene 1 entry path: `/workers` populated
 * with the 5-persona deck after `demo-seed` has been run).
 *
 * Defaults to `needs_onboarding: true` on any gateway error so the
 * wizard remains the safe fallback for unseeded / unreachable
 * tenants — never silently skip onboarding.
 */
async function deriveNeedsOnboarding(sessionToken: string): Promise<boolean> {
  const base = gatewayBase();
  const headers = {
    Accept: "application/json",
    Authorization: `Bearer ${sessionToken}`,
  };
  try {
    const [teamRes, verticalsRes] = await Promise.all([
      fetch(`${base}/api/v1/team`, { headers, cache: "no-store" }),
      fetch(`${base}/api/v1/verticals`, { headers, cache: "no-store" }),
    ]);
    if (!teamRes.ok || !verticalsRes.ok) return true;
    const team = (await teamRes.json().catch(() => null)) as GatewayTeamList | null;
    const verticals = (await verticalsRes
      .json()
      .catch(() => null)) as GatewayVerticalsList | null;
    const workerCount =
      typeof team?.total_count === "number"
        ? team.total_count
        : Array.isArray(team?.workers)
          ? team.workers.length
          : 0;
    const verticalCount = Array.isArray(verticals?.verticals)
      ? verticals.verticals.length
      : 0;
    return !(workerCount > 0 && verticalCount > 0);
  } catch {
    return true;
  }
}

function isProd(): boolean {
  return process.env.NODE_ENV === "production";
}

/**
 * `__Host-` cookie attributes per the spec: Path=/ exact, no Domain,
 * Secure required. Browsers honour `Secure` cookies on `localhost`
 * even over plain HTTP (Chrome + Firefox + Safari), so we keep Secure
 * on in dev — the cookie still lands.
 *
 * SameSite=Lax matches what the gateway's magic-link verify mints,
 * so the same forwarding pattern as `routes_auth_logout.rs:34` is
 * preserved across the two signup paths.
 */
function sessionCookieAttrs(): string {
  return "Path=/; HttpOnly; Secure; SameSite=Lax; Max-Age=604800";
}

function csrfCookieAttrs(): string {
  return "Path=/; Secure; SameSite=Lax; Max-Age=604800";
}

export async function POST(req: NextRequest) {
  let body: MintBody;
  try {
    body = (await req.json()) as MintBody;
  } catch {
    return NextResponse.json(
      { error: { code: "invalid_body", message: "Request body must be JSON." } },
      { status: 400 },
    );
  }
  const email =
    typeof body.email === "string" ? body.email.trim().toLowerCase() : "";
  if (email.length === 0) {
    return NextResponse.json(
      { error: { code: "missing_email", message: "Email is required." } },
      { status: 400 },
    );
  }

  const base = gatewayBase();
  let upstream: Response;
  try {
    upstream = await fetch(`${base}/api/v1/demo/mint-principal`, {
      method: "POST",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify({ email }),
      cache: "no-store",
    });
  } catch (err) {
    // Disambiguate "gateway truly unreachable" from "mint route
    // misbehaved" — the latter is invisible to the operator under the
    // pre-fix code path, which surfaced `gateway_unreachable` on any
    // fetch throw even when `/health` would have answered 200.
    const healthy = await probeGatewayHealth(base);
    if (healthy) {
      const detail =
        err instanceof Error && err.message.length > 0
          ? err.message
          : "unknown demo-mint transport error";
      return NextResponse.json(
        {
          error: {
            code: "gateway_demo_mint_failed",
            message: `Gateway is up but demo-mint failed: ${detail}`,
          },
        },
        { status: 502 },
      );
    }
    return NextResponse.json(
      {
        error: {
          code: "gateway_unreachable",
          message:
            "Couldn't reach SKYCOT. Confirm the gateway is running on the configured port.",
        },
      },
      { status: 502 },
    );
  }

  if (!upstream.ok) {
    const errBody = (await upstream.json().catch(() => ({}))) as GatewayError;
    const message =
      typeof errBody.error === "string"
        ? errBody.error
        : errBody.error?.message ?? "Demo mode is not enabled on the gateway.";
    const code =
      typeof errBody.error === "object" && errBody.error?.code
        ? errBody.error.code
        : upstream.status === 404
          ? "demo_mode_off"
          : "gateway_error";
    return NextResponse.json(
      {
        error: {
          code,
          message:
            upstream.status === 404
              ? "Demo mode isn't enabled on the gateway. Boot it with ARGO_DEMO_MODE=1."
              : message,
        },
      },
      { status: upstream.status },
    );
  }

  const minted = (await upstream.json().catch(() => null)) as GatewayMintOk | null;
  if (!minted || !minted.session_token || !minted.principal) {
    return NextResponse.json(
      {
        error: {
          code: "malformed_mint",
          message: "Gateway returned an unexpected demo-mint payload.",
        },
      },
      { status: 502 },
    );
  }

  // CSRF cookie value: first 32 chars of the session JWT. The csrf
  // middleware compares cookie-value vs header-value (constant-time)
  // and accepts any matching string — the value just needs to be JS-
  // readable on the SPA side and consistent across both surfaces.
  // See `crates/argo-cli/src/handlers/gateway/middleware_csrf.rs` for
  // the gate's exact rules.
  const csrfValue = minted.session_token.slice(0, 32);

  // Derive from real tenant state: workers + vertical installed = no
  // wizard needed. The demo-seed CLI provisions both, so a seeded
  // recording tenant lands on `/workers` (Scene 1 entry path) instead
  // of being bounced into the 4-step onboarding flow. Never-seeded
  // tenants — and any gateway failure — fall back to `true` so the
  // wizard remains the safe default.
  const needsOnboarding = await deriveNeedsOnboarding(minted.session_token);

  const res = NextResponse.json({
    ok: true,
    principal: minted.principal,
    session_token: minted.session_token,
    needs_onboarding: needsOnboarding,
  });
  res.headers.append(
    "set-cookie",
    `__Host-session=${minted.session_token}; ${sessionCookieAttrs()}`,
  );
  res.headers.append(
    "set-cookie",
    `__Host-csrf=${csrfValue}; ${csrfCookieAttrs()}`,
  );
  // No-op in prod (the safety rail) — the gateway refuses to start
  // with `ARGO_DEMO_MODE=1` outside of `ARGO_GATEWAY_AUTH_BYPASS=1`,
  // so this route can only succeed in non-prod environments anyway.
  // Keep the variable referenced so a future linter rule can spot
  // misuse without warning today.
  void isProd;
  return res;
}
