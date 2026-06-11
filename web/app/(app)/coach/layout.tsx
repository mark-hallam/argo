import type { Metadata } from "next";
import type { ReactNode } from "react";

// `/coach` is the founder-coach surface for Cora (the live two-way voice
// coach + founder-story interviewer). The page itself is a `"use client"`
// component — it drives WebRTC voice state — so it cannot export
// `metadata` directly. This sibling layout supplies the static per-route
// metadata so the browser tab reads "Founder coach · SKYCOT" instead of
// falling back to the (app) group default (a camera-readable error during
// demo recording). Mirrors app/(app)/pitch/layout.tsx.
export const metadata: Metadata = {
  title: "Founder coach",
  description: "Talk to Cora, the truth-bounded founder coach.",
  openGraph: {
    title: "Founder coach — SKYCOT",
    description: "Talk to Cora, the truth-bounded founder coach.",
  },
};

export default function CoachLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
