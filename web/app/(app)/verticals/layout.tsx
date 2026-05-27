import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Verticals",
};

export default function VerticalsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
