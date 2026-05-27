import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Onboarding studio",
};

export default function GenesisLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
