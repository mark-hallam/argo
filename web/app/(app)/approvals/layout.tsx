import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Approvals",
  description:
    "Every decision your AI teammates queue for a human shows up here. SKYCOT pages you the moment something arrives.",
};

export default function ApprovalsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
