import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "SKYCOT vs Zapier, n8n, Crew.ai, Lindy",
  description:
    "SKYCOT vs Zapier, n8n, Crew.ai, Lindy — 13 dimensions side-by-side: trust ratchet, approval-first, audit chain, plain-English editing, enterprise.",
  alternates: {
    canonical: "/compare",
  },
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
