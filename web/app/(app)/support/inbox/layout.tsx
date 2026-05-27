import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Support inbox",
};

export default function SupportInboxLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
