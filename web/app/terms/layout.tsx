import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Terms of service",
  description:
    "How SKYCOT and your team agree to work together. Approval-first product. Audit-first product. No surprise lock-in.",
  alternates: {
    canonical: "/terms",
  },
};

export default function TermsLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
