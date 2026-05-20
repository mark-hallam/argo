import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Workflows",
  description:
    "How your AI teammates actually think. Plain-English step-by-step playbooks, versioned like code, edited in chat.",
  openGraph: {
    title: "Workflows — SKYCOT",
    description:
      "Visual step timelines with approval gates inline. Edit in chat, version like code.",
  },
};

export default function WorkflowsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
