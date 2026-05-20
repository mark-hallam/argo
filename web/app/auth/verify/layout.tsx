import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Verifying your magic link",
  description: "Just a second — we&apos;re signing you into SKYCOT.",
  robots: { index: false, follow: false },
};

export default function VerifyLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
