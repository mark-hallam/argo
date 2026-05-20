import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Security",
  description:
    "SKYCOT's security posture — audit chain, capability-based permissions, workspace isolation, encryption-at-rest, responsible disclosure.",
};

export default function SecurityLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
