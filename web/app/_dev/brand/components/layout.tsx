import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Brand components",
};

export default function BrandComponentsLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
