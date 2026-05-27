import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Orchestration",
};

export default function OrchestrationLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
