import type { Metadata } from "next";
import type { ReactNode } from "react";

export const metadata: Metadata = {
  title: "Subscribe",
  description: "Pick a SKYCOT plan that fits your team.",
};

export default function BillingSubscribeLayout({
  children,
}: {
  children: ReactNode;
}) {
  return <>{children}</>;
}
