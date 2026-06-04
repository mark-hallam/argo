import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Data Processing Addendum",
  description:
    "GDPR Article 28 controller-processor template for enterprise SKYCOT customers.",
  alternates: {
    canonical: "/dpa",
  },
};

export default function DpaLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
