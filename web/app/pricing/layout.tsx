import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Pricing",
  description:
    "Hire teammates. Pay like they're real. Start free, add a Pro tier when you're ready for production, talk to us for enterprise.",
  openGraph: {
    title: "Pricing — SKYCOT",
    description:
      "Per-teammate billing. Approval-first, audit-first, enterprise-grade. Start free with no credit card.",
  },
};

export default function PricingLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
