import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Security",
  description:
    "SKYCOT's security posture — audit chain, capability-based permissions, workspace isolation, encryption-at-rest, responsible disclosure.",
  alternates: {
    canonical: "/security",
  },
};

export default function SecurityLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
