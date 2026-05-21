import type { Metadata } from "next";
import type { ReactNode } from "react";
import {
  faqPageSchema,
  JsonLd,
  softwareApplicationSchema,
} from "@/components/seo/JsonLd";

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

const PRICING_FAQ = [
  {
    question: "How does per-teammate billing work?",
    answer:
      "On Pro, every named teammate you hire (Mariah, Priya, Jamal, etc.) is $29/month. You only pay for teammates that are actively working — pause them and billing stops. Enterprise is a flat negotiated rate regardless of teammate count.",
  },
  {
    question: "Can I bring my own AI model keys?",
    answer:
      "Yes, on Pro and Enterprise. Add your OpenAI, Anthropic, Bedrock, or self-hosted endpoint keys in Settings → Providers. SKYCOT routes through them transparently and you pay the model provider directly for inference.",
  },
  {
    question: "What happens if my teammate goes over my action limit?",
    answer:
      "On Free, the teammate pauses and asks you to upgrade. On Pro, you get overage at $0.01 per action with an alert at 80% used. On Enterprise, limits are negotiated up-front — no surprises.",
  },
  {
    question: "Is data shared between tenants?",
    answer:
      "No. Each customer workspace is fully isolated — every audit entry, skill run, integration call, and memory key carries the workspace stamp, and our automated checks block any code that could leak across the boundary. Tenant isolation is the load-bearing security guarantee.",
  },
  {
    question: "Can I self-host SKYCOT?",
    answer:
      "Yes, on Enterprise. SKYCOT ships as a single binary with SQLite + SQLCipher, runs on a Mac mini or any Linux server, and includes a Docker image. The same binary you run locally runs in production — no separate self-host build.",
  },
  {
    question: "How is this different from Zapier / n8n?",
    answer:
      "Zapier and n8n call them 'zaps' and 'workflows'. SKYCOT teammates have names, traits, and signature behaviors — they show up in Slack like a colleague, ask before they act, and earn trust over time. See /compare for the side-by-side breakdown.",
  },
];

export default function PricingLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      <JsonLd
        data={[
          softwareApplicationSchema({
            url: "https://skycot.com/pricing",
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
          }),
          faqPageSchema(PRICING_FAQ),
        ]}
      />
      {children}
    </main>
  );
}
