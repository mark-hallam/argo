import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Docs",
  description: "SKYCOT documentation — guides, references, and examples.",
  openGraph: {
    title: "Docs — SKYCOT",
    description:
      "SKYCOT documentation — operator guides, API reference, workflow recipes, and integration docs.",
  },
};

export default function DocsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
