import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "SKYCOT vs Zapier, n8n, Crew.ai, Lindy",
  description:
    "Side-by-side comparison across 13 dimensions that matter for AI teammates: trust ratchet, approval-first, audit chain, plain-English editing, multi-channel, enterprise-grade.",
  openGraph: {
    title: "SKYCOT vs Zapier, n8n, Crew.ai, Lindy",
    description:
      "Side-by-side, dimension-by-dimension. Spoiler: 8 of 13 rows are clean wins.",
  },
};

export default function CompareLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
