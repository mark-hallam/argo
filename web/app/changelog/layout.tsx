import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Changelog",
  description: "What's new in SKYCOT — features, fixes, and ships.",
  alternates: {
    canonical: "/changelog",
  },
  openGraph: {
    title: "Changelog — SKYCOT",
    description:
      "What's new in SKYCOT — features, fixes, and ships. Every meaningful change to the platform lands here.",
  },
};

export default function ChangelogLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
