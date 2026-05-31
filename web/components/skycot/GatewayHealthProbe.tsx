"use client";

import { useEffect, useState } from "react";
import {
  CANONICAL_LOCAL_GATEWAY_URL,
  gatewayHealthProbeUrl,
  isCanonicalLocalGateway,
  resolveGatewayUrl,
} from "@/lib/config/gateway-url";
import { isDemoMode } from "@/lib/skycot/demo-mode";

/**
 * Boot-time gateway reachability check.
 *
 * Fires a single 2-second `/health` fetch when the (app) shell first
 * mounts. Crucially it probes the SAME-ORIGIN `/health` path — the one
 * Next's server-side rewrite (see `next.config.ts`) proxies through to
 * the gateway — NOT a direct cross-origin `:7400/health` fetch. That
 * matters because the real data path (`lib/api/client.ts`) also goes
 * through the same-origin Next proxy: a direct browser fetch to the
 * gateway's `:7400` origin is refused by the browser (the Rust gateway
 * sends no `Access-Control-Allow-Origin` header / has no OPTIONS
 * handler), so probing `:7400` directly produced a FALSE
 * "Gateway unreachable" banner even when the gateway was healthy and
 * every `/api/v1/*` request succeeded. Probing the proxy makes the
 * banner reflect the data path's TRUE health.
 *
 * On failure surfaces a dismissible amber banner that tells the
 * operator the most likely cause: the dev gateway the Next proxy
 * forwards to is down, or `NEXT_PUBLIC_GATEWAY_URL` does not match the
 * URL the proxy is targeting. Without this banner the symptom is a
 * silently broken page — every `/api/v1/*` call fails with a small
 * console line that non-engineers never see.
 *
 * Suppressed in demo mode: when the SPA is mounted with
 * `NEXT_PUBLIC_SKYCOT_DEMO_MODE=1` the page is serving sample data
 * and the gateway is not expected to be reachable. The amber banner
 * would be a permanent decoration, which is the opposite of what we
 * want for a record-day demo.
 *
 * Suppressed in production builds: a public-facing buyer should never
 * see a self-diagnostic banner. Production deploys must catch this
 * upstream via the build-time canonical-port check
 * (`scripts/check-canonical-gateway-port.mjs`).
 */
export function GatewayHealthProbe() {
  const [state, setState] = useState<"checking" | "ok" | "failed">("checking");
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (process.env.NODE_ENV === "production") {
      setState("ok");
      return;
    }
    if (isDemoMode()) {
      setState("ok");
      return;
    }
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), 2000);
    // Same-origin `/health` (proxied by Next → gateway). Mirrors the
    // real data path in `lib/api/client.ts`; never crosses origins, so
    // the CSP/CORS boundary is untouched and the result reflects the
    // proxy's true reachability rather than a browser-refused direct
    // cross-origin fetch.
    fetch(gatewayHealthProbeUrl(), {
      method: "GET",
      cache: "no-store",
      credentials: "omit",
      signal: ctrl.signal,
    })
      .then((res) => {
        clearTimeout(timer);
        setState(res.ok ? "ok" : "failed");
      })
      .catch(() => {
        clearTimeout(timer);
        setState("failed");
      });
    return () => {
      clearTimeout(timer);
      ctrl.abort();
    };
  }, []);

  if (state !== "failed" || dismissed) return null;

  const gatewayUrl = resolveGatewayUrl();
  const onCanonical = isCanonicalLocalGateway(gatewayUrl);
  const headline = onCanonical
    ? "Gateway unreachable at the documented dev port."
    : "Gateway unreachable at the configured URL.";
  const detail = onCanonical
    ? `The same-origin /health proxy got no response from ${gatewayUrl}. Is the dev gateway running? Start it with \`make dev-gateway\` or \`scripts/restart-live-gateway.sh\`.`
    : `The same-origin /health proxy got no response from ${gatewayUrl}. The canonical dev port is ${CANONICAL_LOCAL_GATEWAY_URL}; if your shell still exports a stale \`NEXT_PUBLIC_GATEWAY_URL\` the proxy is forwarding to the wrong gateway.`;

  return (
    <div
      role="alert"
      aria-live="polite"
      data-testid="gateway-health-probe-banner"
      style={{
        borderBottom: "1px solid #f59e0b",
        background: "#fffbeb",
        color: "#78350f",
        padding: "8px 16px",
        fontSize: 13,
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
      }}
    >
      <div style={{ flex: 1, lineHeight: 1.45 }}>
        <strong style={{ fontWeight: 600 }}>{headline}</strong>{" "}
        <span>{detail}</span>
      </div>
      <button
        type="button"
        onClick={() => setDismissed(true)}
        aria-label="Dismiss gateway-unreachable banner"
        style={{
          background: "transparent",
          border: "1px solid #f59e0b",
          color: "#78350f",
          padding: "2px 10px",
          borderRadius: 4,
          fontSize: 12,
          cursor: "pointer",
          flexShrink: 0,
        }}
      >
        Dismiss
      </button>
    </div>
  );
}
