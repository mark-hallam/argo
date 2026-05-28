"use client";

import { useEffect, useState } from "react";
import {
  CANONICAL_LOCAL_GATEWAY_URL,
  gatewayHealthUrl,
  isCanonicalLocalGateway,
  resolveGatewayUrl,
} from "@/lib/config/gateway-url";
import { isDemoMode } from "@/lib/skycot/demo-mode";

/**
 * Boot-time gateway reachability check.
 *
 * Fires a single 2-second `/health` fetch when the (app) shell first
 * mounts. On failure surfaces a dismissible amber banner that tells
 * the operator the most likely cause: the dev server's
 * `NEXT_PUBLIC_GATEWAY_URL` does not match the URL the SPA is
 * currently trying to reach. Without this banner the symptom is a
 * silently broken page — the browser blocks every fetch with a small
 * console line (Content-Security-Policy `connect-src` refusal) that
 * non-engineers never see.
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
    const gatewayUrl = resolveGatewayUrl();
    fetch(gatewayHealthUrl(gatewayUrl), {
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
    ? `Tried ${gatewayUrl}/health and got no response. Is the dev gateway running? Start it with \`make dev-gateway\` or \`scripts/restart-live-gateway.sh\`.`
    : `Tried ${gatewayUrl}/health and got no response. The canonical dev port is ${CANONICAL_LOCAL_GATEWAY_URL}; if your shell still exports a stale \`NEXT_PUBLIC_GATEWAY_URL\` your browser fetches will be blocked silently by the page's Content-Security-Policy.`;

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
