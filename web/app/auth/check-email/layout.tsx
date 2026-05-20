import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Check your inbox",
  description:
    "We just sent a magic link. Open it on this device to finish signing into SKYCOT.",
  robots: { index: false, follow: false },
};

export default function CheckEmailLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
