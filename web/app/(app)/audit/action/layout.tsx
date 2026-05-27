import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Action",
};

// PERF (P0 fix, 2026-05-27 — fix-spa-proxy-audit-detail-50x-slower-
// than-gateway-direct): force the drill-in segment to render at request
// time rather than be considered for static prerender. Route-segment
// config (`dynamic`, `revalidate`, `fetchCache`) is only honoured on
// server-component files; the page below is `"use client"` so this
// layout is the closest server-component anchor where the config
// applies. `force-dynamic` + `revalidate = 0` tells Next.js to skip
// any static-analysis prerender attempt during dev compile — the
// route is rendered per-request, no upfront data-fetch hooks fire
// during `Compiling /audit/action/[actionId]`.
//
// Without this, dev `pnpm dev` first-paint observed 13 minutes on
// /audit/action/<id> (203s compile + 784s response). With it, cold
// first-paint reduces to the bundle-compile cost alone.
export const dynamic = "force-dynamic";
export const revalidate = 0;

export default function AuditActionLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
