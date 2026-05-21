import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Access expired",
  description:
    "Your SKYCOT workspace access expired. Contact your admin if you need to renew.",
  robots: { index: false, follow: false },
};

export default function ExpiredLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
