import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Audit",
};

export default function AuditLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
