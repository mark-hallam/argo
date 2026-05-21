import type { Metadata } from "next";
import type { ReactNode } from "react";
import { JsonLd, softwareApplicationSchema } from "@/components/seo/JsonLd";

export const metadata: Metadata = {
  title: "Hire your first AI teammate",
  description:
    "Don't buy another tool. Hire a teammate. AI teammates that live in Slack, ask before they act, earn trust over time, and ship a full audit trail.",
  openGraph: {
    title: "Hire your first AI teammate — SKYCOT",
    description:
      "AI teammates that ask before they act. Approval-first, audit-first, single-binary, enterprise-grade.",
  },
};

export default function LandingLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      <JsonLd
        data={softwareApplicationSchema({
          url: "https://skycot.com/landing",
          lowPrice: "0",
          highPrice: "29",
          offers: [
            {
              "@type": "Offer",
              name: "Free",
              price: "0",
              priceCurrency: "USD",
              category: "Free",
              url: "https://skycot.com/auth/signup",
            },
            {
              "@type": "Offer",
              name: "Pro",
              price: "29",
              priceCurrency: "USD",
              category: "Subscription",
              url: "https://skycot.com/auth/signup?tier=pro",
            },
          ],
        })}
      />
      {children}
    </main>
  );
}
