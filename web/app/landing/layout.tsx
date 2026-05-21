import type { Metadata } from "next";
import type { ReactNode } from "react";

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
      {children}
    </main>
  );
}
