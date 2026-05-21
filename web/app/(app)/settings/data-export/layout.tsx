import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Data export",
};

export default function DataExportLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
