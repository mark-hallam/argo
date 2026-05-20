import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Help",
  description: "How to reach SKYCOT support, where the docs live, and how the v1 product is shaped.",
};

export default function HelpLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
