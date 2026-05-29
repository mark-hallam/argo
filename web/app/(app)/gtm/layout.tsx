import type { Metadata } from "next";
import type { ReactNode } from "react";

// `/gtm` is the operator-facing console for Grant, the GTM (go-to-market)
// Engineer. The page is a `"use client"` component — it calls the GTM
// API and manages form state — so it cannot export `metadata` directly.
// This sibling layout supplies the static per-route metadata so the
// browser tab reads "GTM — Grant · SKYCOT" rather than the (app) group
// default (a camera-readable fallback during demo recording).
export const metadata: Metadata = {
  title: "GTM — Grant",
  description: "Watch Grant draft truth-bounded inbound follow-ups, filed to your approval queue.",
  openGraph: {
    title: "GTM — Grant · SKYCOT",
    description: "Watch Grant draft truth-bounded inbound follow-ups, filed to your approval queue.",
  },
};

export default function GtmLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
