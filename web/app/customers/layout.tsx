import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Customers",
  description:
    "300+ teams shipping with SKYCOT. From solo founders to Fortune 500. Hear how they're using their AI teammates.",
  openGraph: {
    title: "Customers — SKYCOT",
    description:
      "Teams shipping with SKYCOT. AI teammates that ask before they act, earn trust, ship an audit trail.",
  },
};

export default function CustomersLayout({ children }: { children: ReactNode }) {
  return <>{children}</>;
}
