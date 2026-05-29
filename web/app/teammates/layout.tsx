import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Meet the team you'll hire",
  description:
    "Seven named teammates the SKYCOT platform ships with on day one — real personas with real responsibilities. See who's already on the team.",
  alternates: {
    canonical: "/teammates",
  },
  openGraph: {
    title: "Meet the team — SKYCOT",
    description:
      "Seven teammates with names, traits, and signature behaviours. Hire your own on Day 1.",
  },
};

export default function TeammatesLayout({ children }: { children: ReactNode }) {
  return (
    <main id="main" tabIndex={-1} className="focus:outline-none">
      {children}
    </main>
  );
}
