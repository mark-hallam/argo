import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Workers",
};

export default function WorkersLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
