import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Connectors",
};

export default function ConnectorsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
