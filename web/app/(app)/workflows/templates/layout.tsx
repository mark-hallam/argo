import type { Metadata } from "next";
import type { ReactNode } from "react";
import { TEMPLATE_COUNT } from "@/lib/workflow-templates";

export const metadata: Metadata = {
  title: "Workflow templates",
  description: `${TEMPLATE_COUNT} pre-built workflows your AI teammates can use today. Pick one, customize in chat, ship in minutes.`,
  openGraph: {
    title: "Workflow templates — SKYCOT",
    description:
      "Curated gallery of plain-English workflow templates across Sales, Support, Finance, People Ops, Marketing, Ops, Engineering and Compliance.",
  },
};

export default function WorkflowTemplatesLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
