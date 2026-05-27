import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Retro",
};

export default function RetroLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
