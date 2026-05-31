import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Customers",
  description:
    "Representative SKYCOT deployment profiles for teams hiring approval-first AI teammates. Cleared customer stories replace these when they are published.",
  alternates: {
    canonical: "/customers",
  },
  openGraph: {
    title: "Customers — SKYCOT",
    description:
      "Representative deployment profiles for AI teammates that ask before they act, earn trust, and ship an audit trail.",
  },
};

export default function CustomersLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
