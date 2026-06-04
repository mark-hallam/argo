import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Features",
  description:
    "The six load-bearing claims SKYCOT delivers: teammate-not-tool, delegation as the unit, platform-generated workers, shared skills, real-world execution, and multi-channel approval.",
  alternates: {
    canonical: "/features",
  },
};

export default function FeaturesLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
