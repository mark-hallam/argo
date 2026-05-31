import type { Metadata } from "next";
import type { ReactNode } from "react";

// `/marketing` is the operator-facing console for Marcus, the marketing
// teammate (DEC-028). The page is a `"use client"` component — it calls
// the marketing API and manages form state — so it cannot export
// `metadata` directly. This sibling layout supplies the static per-route
// metadata so the browser tab reads "Marketing — Marcus · SKYCOT" rather
// than the (app) group default.
export const metadata: Metadata = {
  title: "Marketing — Marcus",
  description:
    "Watch Marcus draft truth-bounded replies + articles in Mark's voice, filed to your approval queue — never auto-posted.",
  openGraph: {
    title: "Marketing — Marcus · SKYCOT",
    description:
      "Watch Marcus draft truth-bounded replies + articles in Mark's voice, filed to your approval queue — never auto-posted.",
  },
};

export default function MarketingLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
